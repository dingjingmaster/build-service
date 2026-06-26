#!/usr/bin/env bash
set -eu

MODE="${1:-}"
if [ -z "$MODE" ]; then
    echo "usage: $0 deb|rpm|emerge" >&2
    exit 2
fi

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
NAME=buildsvc
VERSION=$(awk -F'"' '/^version =/ { print $2; exit }' "$ROOT_DIR/Cargo.toml")
RELEASE=1
OUT_DIR="$ROOT_DIR/target/package"
BIN="$ROOT_DIR/target/release/buildsvc"

if [ ! -x "$BIN" ]; then
    echo "missing release binary: $BIN" >&2
    echo "run: cargo build --release" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

host_arch() {
    uname -m
}

deb_arch() {
    case "$(host_arch)" in
        x86_64|amd64) echo amd64 ;;
        aarch64|arm64) echo arm64 ;;
        armv7l) echo armhf ;;
        *) host_arch ;;
    esac
}

gentoo_keywords() {
    if [ -n "${GENTOO_KEYWORDS:-}" ]; then
        echo "$GENTOO_KEYWORDS"
        return
    fi

    case "$(host_arch)" in
        x86_64|amd64) echo "amd64" ;;
        aarch64|arm64) echo "arm64" ;;
        *) echo "~$(host_arch)" ;;
    esac
}

stage_root() {
    dest="$1"
    rm -rf "$dest"
    mkdir -p "$dest"
    install -Dm755 "$BIN" "$dest/usr/bin/buildsvc"
    install -Dm644 "$ROOT_DIR/packaging/buildsvc.ini" "$dest/etc/buildsvc/buildsvc.ini"
    install -Dm644 "$ROOT_DIR/packaging/buildsvc.service" "$dest/usr/lib/systemd/system/buildsvc.service"
    install -Dm644 "$ROOT_DIR/configs/server.ini" "$dest/usr/share/doc/buildsvc/examples/server.ini"
    install -Dm644 "$ROOT_DIR/configs/agent.ini" "$dest/usr/share/doc/buildsvc/examples/agent.ini"
}

build_deb() {
    command -v dpkg-deb >/dev/null 2>&1 || {
        echo "dpkg-deb not found" >&2
        exit 1
    }

    arch=$(deb_arch)
    pkgdir="$OUT_DIR/deb/${NAME}_${VERSION}-${RELEASE}_${arch}"
    stage_root "$pkgdir"
    mkdir -p "$pkgdir/DEBIAN"
    installed_size=$(du -sk "$pkgdir" | awk '{ print $1 }')

    cat > "$pkgdir/DEBIAN/control" <<EOF
Package: $NAME
Version: $VERSION-$RELEASE
Section: devel
Priority: optional
Architecture: $arch
Maintainer: buildsvc maintainers <root@localhost>
Installed-Size: $installed_size
Description: Lightweight distributed build service
 buildsvc is a single Rust binary that can run as a server or agent.
 It distributes source archives to agents, runs preset scripts, streams logs,
 and provides an optional trusted-network web terminal through the agent.
EOF

    cat > "$pkgdir/DEBIAN/conffiles" <<EOF
/etc/buildsvc/buildsvc.ini
EOF

    out="$OUT_DIR/${NAME}_${VERSION}-${RELEASE}_${arch}.deb"
    dpkg-deb --root-owner-group --build "$pkgdir" "$out"
    echo "$out"
}

build_rpm() {
    command -v rpmbuild >/dev/null 2>&1 || {
        echo "rpmbuild not found" >&2
        exit 1
    }

    top="$OUT_DIR/rpmbuild"
    srcroot="$top/SOURCES/${NAME}-${VERSION}"
    rm -rf "$top"
    mkdir -p "$top/BUILD" "$top/BUILDROOT" "$top/RPMS" "$top/SOURCES" "$top/SPECS" "$top/SRPMS"
    stage_root "$srcroot"
    tar -C "$top/SOURCES" -czf "$top/SOURCES/${NAME}-${VERSION}.tar.gz" "${NAME}-${VERSION}"

    spec="$top/SPECS/${NAME}.spec"
    cat > "$spec" <<EOF
Name:           $NAME
Version:        $VERSION
Release:        $RELEASE%{?dist}
Summary:        Lightweight distributed build service
License:        MIT
Source0:        %{name}-%{version}.tar.gz

%global debug_package %{nil}

%description
buildsvc is a single Rust binary that can run as a server or agent. It
distributes source archives to agents, runs preset scripts, streams logs,
and provides an optional trusted-network web terminal through the agent.

%prep
%setup -q

%build

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a . %{buildroot}

%files
%dir /etc/buildsvc
%config(noreplace) /etc/buildsvc/buildsvc.ini
%attr(0755,root,root) /usr/bin/buildsvc
/usr/lib/systemd/system/buildsvc.service
%dir /usr/share/doc/buildsvc
%dir /usr/share/doc/buildsvc/examples
%doc /usr/share/doc/buildsvc/examples/server.ini
%doc /usr/share/doc/buildsvc/examples/agent.ini

%changelog
* Fri Jun 26 2026 buildsvc maintainers <root@localhost> - $VERSION-$RELEASE
- Package buildsvc.
EOF

    rpmbuild --define "_topdir $top" -bb "$spec"
    find "$top/RPMS" -type f -name "*.rpm" -exec cp {} "$OUT_DIR/" \;
    find "$OUT_DIR" -maxdepth 1 -type f -name "${NAME}-${VERSION}-${RELEASE}"'*.rpm' -print
}

build_emerge() {
    overlay="$OUT_DIR/gentoo-overlay"
    pkgdir="$overlay/app-admin/$NAME"
    files="$pkgdir/files"
    rm -rf "$overlay"
    mkdir -p "$files" "$overlay/metadata" "$overlay/profiles"

    install -Dm755 "$BIN" "$files/buildsvc"
    install -Dm644 "$ROOT_DIR/packaging/buildsvc.ini" "$files/buildsvc.ini"
    install -Dm644 "$ROOT_DIR/packaging/buildsvc.service" "$files/buildsvc.service"
    install -Dm644 "$ROOT_DIR/configs/server.ini" "$files/server.ini"
    install -Dm644 "$ROOT_DIR/configs/agent.ini" "$files/agent.ini"

    echo "buildsvc-local" > "$overlay/profiles/repo_name"
    cat > "$overlay/metadata/layout.conf" <<EOF
masters = gentoo
EOF

    ebuild="$pkgdir/${NAME}-${VERSION}.ebuild"
    keywords=$(gentoo_keywords)
    cat > "$ebuild" <<EOF
EAPI=8

inherit systemd

DESCRIPTION="Lightweight distributed build service"
HOMEPAGE=""
LICENSE="MIT"
SLOT="0"
KEYWORDS="$keywords"
RESTRICT="strip"
QA_PREBUILT="/usr/bin/buildsvc"

S="\${WORKDIR}"

src_install() {
	dobin "\${FILESDIR}/buildsvc"

	insinto /etc/buildsvc
	newins "\${FILESDIR}/buildsvc.ini" buildsvc.ini

	systemd_dounit "\${FILESDIR}/buildsvc.service"
	dodoc "\${FILESDIR}/server.ini" "\${FILESDIR}/agent.ini"
}
EOF

    if command -v ebuild >/dev/null 2>&1; then
        (cd "$pkgdir" && ebuild "${NAME}-${VERSION}.ebuild" manifest)
    else
        echo "ebuild not found; generated overlay without Manifest" >&2
    fi

    out="$OUT_DIR/${NAME}-${VERSION}-gentoo-overlay.tar.gz"
    tar -C "$OUT_DIR" -czf "$out" gentoo-overlay
    echo "$out"
    echo "To install on Gentoo, add the generated overlay or run with PORTDIR_OVERLAY=$overlay emerge -av app-admin/$NAME" >&2
}

case "$MODE" in
    deb) build_deb ;;
    rpm) build_rpm ;;
    emerge) build_emerge ;;
    *)
        echo "unknown package mode: $MODE" >&2
        exit 2
        ;;
esac
