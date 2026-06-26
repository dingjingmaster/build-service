# 包安装卸载 systemd Hook 任务记录

> 日期：2026-06-26
> 级别：L2
> 状态：已完成

## 背景

用户需要安装包在安装后自动执行 `systemd daemon-reload`、`systemd enable`、`systemd restart`，并在卸载后自动执行 `systemd disable` 等清理操作。

## 实现

- deb：
  - `postinst configure`：`systemctl daemon-reload`、`enable buildsvc.service`、`restart buildsvc.service`。
  - `prerm remove|deconfigure`：`stop buildsvc.service`、`disable buildsvc.service`。
  - `postrm remove|purge`：`disable buildsvc.service`、`daemon-reload`。
- rpm：
  - `%post`：daemon-reload、enable、restart。
  - `%preun`：最终卸载时 stop、disable。
  - `%postun`：daemon-reload。
- Gentoo ebuild：
  - `pkg_postinst`：daemon-reload、enable、restart。
  - `pkg_prerm`：非升级卸载时 stop、disable。
  - `pkg_postrm`：daemon-reload。

所有 systemctl 调用都会检查 `systemctl` 是否存在，并且失败不阻断包管理器流程。

## 验证

- `make deb`
- `make rpm`
- `make emerge`
- 检查 deb maintainer scripts、rpm scriptlets 和 Gentoo ebuild hook 内容。
