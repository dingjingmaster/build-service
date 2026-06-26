# Linux 打包安装任务记录

> 日期：2026-06-26
> 级别：L3
> 状态：已完成

## 背景

用户需要通过 Makefile 生成 Gentoo emerge、Debian deb、RPM 三种安装包或包定义，目标命令为 `make emerge`、`make deb`、`make rpm`。

## 方案

- Makefile 新增 `deb`、`rpm`、`emerge` 目标，均依赖 release 二进制。
- 新增 `packaging/package.sh`，统一 staging 安装树。
- 包内安装内容：
  - `/usr/bin/buildsvc`
  - `/etc/buildsvc/buildsvc.ini`
  - `/usr/lib/systemd/system/buildsvc.service`
  - `/usr/share/doc/buildsvc/examples/server.ini`
  - `/usr/share/doc/buildsvc/examples/agent.ini`
- deb 使用 `dpkg-deb --build` 生成。
- rpm 使用 `rpmbuild -bb` 生成。
- Gentoo 使用本地 Portage overlay 形式生成 `app-admin/buildsvc` ebuild，并打包为 overlay tarball；本机有 `ebuild` 时生成 Manifest。
- Gentoo ebuild 默认按当前架构生成稳定 keyword（如 `amd64`、`arm64`），避免自用本地 overlay 在稳定系统上被 `~amd64 keyword` mask；如需 unstable keyword，可用 `GENTOO_KEYWORDS='~amd64' make emerge` 覆盖。

## 验证

- `make deb`
- `make rpm`
- `make emerge`
- `dpkg-deb -c target/package/buildsvc_0.1.0-1_amd64.deb`
- `rpm -qpl target/package/buildsvc-0.1.0-1.x86_64.rpm`
- `tar -tzf target/package/buildsvc-0.1.0-gentoo-overlay.tar.gz`
- `make test`

## 结果

三种打包入口均已实现并在本地验证通过。产物输出到 `target/package/`。
