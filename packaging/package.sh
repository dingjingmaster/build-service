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

package_config_src() {
    if [ -s "$ROOT_DIR/configs/buildsvc.ini" ]; then
        echo "$ROOT_DIR/configs/buildsvc.ini"
    else
        echo "$ROOT_DIR/packaging/buildsvc.ini"
    fi
}

install_examples() {
    dest="$1"
    examples="$dest/usr/share/doc/buildsvc/examples"
    mkdir -p "$examples"
    install -Dm644 "$(package_config_src)" "$examples/buildsvc.ini"
    if [ -f "$ROOT_DIR/configs/server.ini" ]; then
        install -Dm644 "$ROOT_DIR/configs/server.ini" "$examples/server.ini"
    fi
    if [ -f "$ROOT_DIR/configs/agent.ini" ]; then
        install -Dm644 "$ROOT_DIR/configs/agent.ini" "$examples/agent.ini"
    fi
}

stage_root() {
    dest="$1"
    rm -rf "$dest"
    mkdir -p "$dest"
    install -Dm755 "$BIN" "$dest/usr/bin/buildsvc"
    install -Dm644 "$(package_config_src)" "$dest/etc/buildsvc/buildsvc.ini"
    install -Dm644 "$ROOT_DIR/packaging/buildsvc.service" "$dest/usr/lib/systemd/system/buildsvc.service"
    install_examples "$dest"
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

    cat > "$pkgdir/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e

case "$1" in
    configure)
        if command -v systemctl >/dev/null 2>&1; then
            systemctl daemon-reload >/dev/null 2>&1 || true
            systemctl enable buildsvc.service >/dev/null 2>&1 || true
            systemctl restart buildsvc.service >/dev/null 2>&1 || true
        fi
        ;;
esac

exit 0
EOF

    cat > "$pkgdir/DEBIAN/prerm" <<'EOF'
#!/bin/sh
set -e

case "$1" in
    remove|deconfigure)
        if command -v systemctl >/dev/null 2>&1; then
            systemctl stop buildsvc.service >/dev/null 2>&1 || true
            systemctl disable buildsvc.service >/dev/null 2>&1 || true
        fi
        ;;
esac

exit 0
EOF

    cat > "$pkgdir/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e

case "$1" in
    remove|purge)
        if command -v systemctl >/dev/null 2>&1; then
            systemctl disable buildsvc.service >/dev/null 2>&1 || true
            systemctl daemon-reload >/dev/null 2>&1 || true
        fi
        ;;
esac

exit 0
EOF

    chmod 755 "$pkgdir/DEBIAN/postinst" "$pkgdir/DEBIAN/prerm" "$pkgdir/DEBIAN/postrm"

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

%post
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload >/dev/null 2>&1 || :
    systemctl enable buildsvc.service >/dev/null 2>&1 || :
    systemctl restart buildsvc.service >/dev/null 2>&1 || :
fi

%preun
if [ "\$1" -eq 0 ] && command -v systemctl >/dev/null 2>&1; then
    systemctl stop buildsvc.service >/dev/null 2>&1 || :
    systemctl disable buildsvc.service >/dev/null 2>&1 || :
fi

%postun
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload >/dev/null 2>&1 || :
fi

%files
%dir /etc/buildsvc
%config(noreplace) /etc/buildsvc/buildsvc.ini
%attr(0755,root,root) /usr/bin/buildsvc
/usr/lib/systemd/system/buildsvc.service
%dir /usr/share/doc/buildsvc
%dir /usr/share/doc/buildsvc/examples
%doc /usr/share/doc/buildsvc/examples/*

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
    install -Dm644 "$(package_config_src)" "$files/buildsvc.ini"
    install -Dm644 "$ROOT_DIR/packaging/buildsvc.service" "$files/buildsvc.service"
    if [ -f "$ROOT_DIR/configs/server.ini" ]; then
        install -Dm644 "$ROOT_DIR/configs/server.ini" "$files/server.ini"
    fi
    if [ -f "$ROOT_DIR/configs/agent.ini" ]; then
        install -Dm644 "$ROOT_DIR/configs/agent.ini" "$files/agent.ini"
    fi

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
	dodoc "\${FILESDIR}/buildsvc.ini"
	if [[ -f "\${FILESDIR}/server.ini" ]]; then
		dodoc "\${FILESDIR}/server.ini"
	fi
	if [[ -f "\${FILESDIR}/agent.ini" ]]; then
		dodoc "\${FILESDIR}/agent.ini"
	fi
}

pkg_postinst() {
	if command -v systemctl >/dev/null 2>&1; then
		systemctl daemon-reload >/dev/null 2>&1 || true
		systemctl enable buildsvc.service >/dev/null 2>&1 || true
		systemctl restart buildsvc.service >/dev/null 2>&1 || true
	fi
}

pkg_prerm() {
	if [[ -z \${REPLACED_BY_VERSION} ]] && command -v systemctl >/dev/null 2>&1; then
		systemctl stop buildsvc.service >/dev/null 2>&1 || true
		systemctl disable buildsvc.service >/dev/null 2>&1 || true
	fi
}

pkg_postrm() {
	if command -v systemctl >/dev/null 2>&1; then
		systemctl daemon-reload >/dev/null 2>&1 || true
	fi
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
