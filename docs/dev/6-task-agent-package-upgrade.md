# Agent 包升级任务记录

> 日期：2026-06-26
> 级别：L3
> 状态：已完成

## 背景

用户需要在 server Web UI 中推送升级包给 agent，并在 Agents 表中看到 agent 当前版本。升级包要求使用已有 Linux 包格式：deb、rpm、Gentoo emerge overlay。升级相当于完整安装，但不能强制覆盖配置文件。

## 方案

- agent hello 上报版本、平台、架构和升级能力。
- Agents 表新增 Version 列。
- 新增 `server.upgrade_enabled` 和 `agent.upgrade_enabled`，默认关闭。
- 新增 `agent.upgrade_work_dir`，默认 `<agent_work_dir>/upgrades`。
- Web UI 新增 Upgrade Agents 区域：
  - 选择包类型：deb、rpm、emerge。
  - 上传升级包。
  - 选择在线且启用升级的 agent。
  - 显示升级状态和安装日志。
- server 上传升级包后保存到 `<server_data_dir>/upgrades/<upgrade_id>/`，计算 sha256，并通过 agent WebSocket 下发升级指令。
- agent 下载升级包到 `<agent_work_dir>/upgrades/<upgrade_id>/`，校验 sha256 后执行安装：
  - deb：优先 `apt-get install -y -o Dpkg::Options::=--force-confold`，无 apt-get 时回退 `dpkg --force-confold -i`。
  - rpm：优先 `dnf install -y`，其次 `yum install -y`，最后 `rpm -Uvh --replacepkgs`。
  - emerge：解包 overlay tarball 后用 `PORTDIR_OVERLAY=<overlay> emerge app-admin/buildsvc`。
- 安装完成后 agent 执行 `systemctl daemon-reload`，再请求 `systemctl restart buildsvc`。
- agent 有活动 run 时拒绝升级；server 在升级排队后不再向该 agent 派发新 run。

## 配置保护

- deb：包内 `/etc/buildsvc/buildsvc.ini` 是 conffile，升级命令使用 `--force-confold`。
- rpm：spec 使用 `%config(noreplace)`。
- Gentoo：安装路径受 Portage `CONFIG_PROTECT` 管理。

## 限制

- 第一版远程包升级只支持 Linux agent。
- agent 进程需要有安装系统包和重启 `buildsvc` service 的权限，通常应作为 root service 运行。
- server 运行时保存升级包元数据；服务重启后已上传但未完成的升级包不恢复。
- deb/rpm/Gentoo 实际安装流程仍需在对应发行版实机验证。

## 验证

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `make`

## 结果

远程包升级协议、server API、agent 安装执行、Web UI 和版本显示已实现。配置默认关闭，开发样例中已启用以方便本地调试。
