use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, bail};
use flate2::read::GzDecoder;

use crate::protocol::ArchiveFormat;

pub fn extract_archive(
    archive_path: &Path,
    format: ArchiveFormat,
    destination: &Path,
) -> anyhow::Result<PathBuf> {
    if destination.exists() {
        fs::remove_dir_all(destination)
            .with_context(|| format!("clear {}", destination.display()))?;
    }
    fs::create_dir_all(destination).with_context(|| format!("create {}", destination.display()))?;

    let top_dirs = match format {
        ArchiveFormat::TarGz => extract_tar_gz(archive_path, destination)?,
        ArchiveFormat::Zip => extract_zip(archive_path, destination)?,
    };

    select_source_root(destination, top_dirs)
}

fn extract_tar_gz(archive_path: &Path, destination: &Path) -> anyhow::Result<BTreeSet<PathBuf>> {
    let file =
        File::open(archive_path).with_context(|| format!("open {}", archive_path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut top_dirs = BTreeSet::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let top_dir = validate_archive_path(&path)?;
        top_dirs.insert(top_dir);
        entry
            .unpack_in(destination)
            .with_context(|| format!("extract {}", path.display()))?;
    }

    Ok(top_dirs)
}

fn extract_zip(archive_path: &Path, destination: &Path) -> anyhow::Result<BTreeSet<PathBuf>> {
    let file =
        File::open(archive_path).with_context(|| format!("open {}", archive_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut top_dirs = BTreeSet::new();

    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let Some(path) = file.enclosed_name().map(PathBuf::from) else {
            bail!("zip entry contains an unsafe path: {}", file.name());
        };
        let top_dir = validate_archive_path(&path)?;
        top_dirs.insert(top_dir);
        let out_path = destination.join(&path);
        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&out_path)?;
            std::io::copy(&mut file, &mut output)?;
        }
    }

    Ok(top_dirs)
}

fn validate_archive_path(path: &Path) -> anyhow::Result<PathBuf> {
    let mut components = path.components();
    let Some(first) = components.next() else {
        bail!("archive contains empty path");
    };

    let Component::Normal(top_dir) = first else {
        bail!("archive path must be relative and start with a directory");
    };

    for component in components {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => bail!("archive path contains unsafe component: {}", path.display()),
        }
    }

    Ok(PathBuf::from(top_dir))
}

fn select_source_root(destination: &Path, top_dirs: BTreeSet<PathBuf>) -> anyhow::Result<PathBuf> {
    if has_build_script(destination) {
        return Ok(destination.to_owned());
    }

    if top_dirs.len() == 1 {
        let top_dir = top_dirs.into_iter().next().expect("checked length");
        let source_root = destination.join(top_dir);
        if source_root.is_dir() && has_build_script(&source_root) {
            return Ok(source_root);
        }
    }

    bail!(
        "archive must contain run-build.sh/run-build.bat either at archive root or inside one top-level directory"
    )
}

fn has_build_script(path: &Path) -> bool {
    path.join("run-build.sh").is_file() || path.join("run-build.bat").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_components() {
        let err = validate_archive_path(Path::new("src/../evil")).unwrap_err();
        assert!(err.to_string().contains("unsafe"));
    }

    #[test]
    fn accepts_single_relative_path() {
        let top = validate_archive_path(Path::new("project/run-build.sh")).unwrap();
        assert_eq!(top, PathBuf::from("project"));
    }

    #[test]
    fn selects_archive_root_when_script_is_at_root() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("run-build.sh"), b"#!/bin/sh\n").unwrap();
        let mut dirs = BTreeSet::new();
        dirs.insert(PathBuf::from("run-build.sh"));
        dirs.insert(PathBuf::from("src"));

        let root = select_source_root(temp.path(), dirs).unwrap();

        assert_eq!(root, temp.path());
    }

    #[test]
    fn selects_single_top_dir_when_script_is_inside_it() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("project")).unwrap();
        fs::write(
            temp.path().join("project").join("run-build.sh"),
            b"#!/bin/sh\n",
        )
        .unwrap();
        let mut dirs = BTreeSet::new();
        dirs.insert(PathBuf::from("project"));

        let root = select_source_root(temp.path(), dirs).unwrap();

        assert_eq!(root, temp.path().join("project"));
    }

    #[test]
    fn rejects_ambiguous_archive_without_root_script() {
        let temp = tempfile::tempdir().unwrap();
        let mut dirs = BTreeSet::new();
        dirs.insert(PathBuf::from("a"));
        dirs.insert(PathBuf::from("b"));
        assert!(select_source_root(temp.path(), dirs).is_err());
    }
}
