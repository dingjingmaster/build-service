# buildsvc 自重启导致包配置挂起修复记录

> 日期：2026-06-26
> 级别：L3
> 状态：已完成

## 背景

用户在 buildsvc Web 终端中执行 `dpkg --configure -a`，输出 `正在设置 buildsvc ...` 后长时间不返回。该命令会触发 deb 包 `postinst configure`，而当前包脚本会同步执行 `systemctl restart buildsvc.service`。

## 根因

Web 终端 shell、`dpkg` 和包维护脚本都是 `buildsvc.service` 进程树的子进程。维护脚本在这个上下文里同步重启 `buildsvc.service` 时，会影响正在执行的 `dpkg`/PTY/WebSocket 链路，表现为命令不返回或页面停在最后一行输出。

## 修复

- deb/rpm/Gentoo 安装后脚本检测当前进程是否位于 `buildsvc.service` cgroup。
- 如果位于自身 service 中，优先使用 `systemd-run --on-active=2s` 在独立 transient unit 中延迟重启；失败时退到 `systemctl --no-block restart`。
- 如果不是从自身 service 中触发，仍保持同步 `systemctl restart buildsvc.service`。
- agent 远程升级后的重启请求改为优先通过 `systemd-run` 延迟调度，失败时使用 `systemctl --no-block restart`。
- Web 终端 WebSocket 异常关闭时追加 `[terminal disconnected]`，避免页面看起来像命令仍在执行。

## 验证

- `bash -n packaging/package.sh`
- `cargo fmt --check`
- `cargo test`

## 残余风险

旧版本已安装到系统中的 maintainer script 不会被源码修改直接改变。已经卡住的 `dpkg --configure -a` 需要从 buildsvc Web 终端以外的 shell 处理；新打出的包会包含该修复。
