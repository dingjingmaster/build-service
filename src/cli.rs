use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Run(CliOptions),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOptions {
    pub config_path: Option<PathBuf>,
}

pub fn parse() -> anyhow::Result<CliAction> {
    parse_from(std::env::args_os())
}

fn parse_from<I>(args: I) -> anyhow::Result<CliAction>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let _program = args.next();
    let mut config_path = None;

    while let Some(arg) = args.next() {
        let value = arg.to_string_lossy();
        match value.as_ref() {
            "-h" | "--help" => return Ok(CliAction::Help),
            "-c" | "--config" => {
                let path = args.next().context("missing value for --config")?;
                set_config_path(&mut config_path, path)?;
            }
            _ if value.starts_with("--config=") => {
                let path = value
                    .strip_prefix("--config=")
                    .context("missing value for --config")?;
                if path.is_empty() {
                    bail!("missing value for --config");
                }
                set_config_path(&mut config_path, OsString::from(path))?;
            }
            _ => bail!("unknown argument: {value}"),
        }
    }

    Ok(CliAction::Run(CliOptions { config_path }))
}

fn set_config_path(slot: &mut Option<PathBuf>, path: OsString) -> anyhow::Result<()> {
    if slot.is_some() {
        bail!("--config can only be specified once");
    }
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        bail!("missing value for --config");
    }
    *slot = Some(path);
    Ok(())
}

pub fn usage() -> &'static str {
    "Usage: buildsvc [-c|--config <path>]\n\nOptions:\n  -c, --config <path>  Read configuration from the specified INI file\n  -h, --help           Show this help\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_short_config_arg() {
        let action = parse_from(args(&["buildsvc", "-c", "server.ini"])).unwrap();
        assert_eq!(
            action,
            CliAction::Run(CliOptions {
                config_path: Some(PathBuf::from("server.ini"))
            })
        );
    }

    #[test]
    fn parses_long_config_arg() {
        let action = parse_from(args(&["buildsvc", "--config=agent.ini"])).unwrap();
        assert_eq!(
            action,
            CliAction::Run(CliOptions {
                config_path: Some(PathBuf::from("agent.ini"))
            })
        );
    }

    #[test]
    fn rejects_unknown_arg() {
        let err = parse_from(args(&["buildsvc", "--bad"])).unwrap_err();
        assert!(err.to_string().contains("unknown argument"));
    }
}
