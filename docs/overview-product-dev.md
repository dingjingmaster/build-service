# buildsvc 开发概览

> 文档元数据
> - 文档版本：v1.0.0
> - 最后更新：2026-06-26
> - 更新来源：docs/dev/1-*.md
> - 关联产品文档：docs/overview-product.md

## 1. 技术栈

| 类别 | 技术/版本 | 用途 | 备注 |
|------|-----------|------|------|
| 语言 | Rust 2024 edition | 单二进制 server/agent | 当前验证工具链 Rust 1.93.1 |
| 构建系统 | Makefile + Cargo | 构建、测试、格式化、打包 | `make`、`make debug`、`make test`、`make deb`、`make rpm`、`make emerge` |
| 运行平台 | Linux、Windows、macOS | agent/server 目标平台 | 当前仅在 Linux 工作区自动验证 |
| HTTP/WebSocket | axum、tokio、tokio-tungstenite | server API/UI 和 agent WS client | WebSocket 消息为 JSON text |
| 持久化 | rusqlite + SQLite | build/run 元数据 | 日志和源码包放文件系统 |
| 解包 | tar、flate2、zip | `tar.gz`/`zip` source archive | 校验顶层唯一目录和路径安全 |
| 校验 | sha2 | 远程升级包 sha256 校验 | server 上传时计算，agent 下载后复核 |
| PTY | portable-pty | Agent 交互式 Web 终端 | server 只转发，PTY 在 agent 本机创建 |
| Linux 打包 | dpkg-deb、rpmbuild、Portage ebuild overlay | 生成 deb/rpm/Gentoo emerge 包定义 | 输出到 `target/package/` |

## 2. 架构边界

- 模块划分：
  - `config`：INI 发现、解析、role 分派配置。
  - `protocol`：agent/server/UI JSON 消息和 UI view model。
  - `storage`：SQLite schema、build/run 状态、日志文件。
  - `server`：HTTP API、Web UI、agent WebSocket、调度。
  - `agent`：WebSocket client、源码下载、解包、脚本执行、日志回传、PTY 终端会话。
  - `archive`：跨格式解包与路径校验。
- 进程/线程边界：
  - server 和每个 agent 是独立进程。
  - server 使用 tokio 异步处理 HTTP/WS；SQLite 操作用 mutex 包装。
  - agent 每个 run 使用独立任务和独立工作目录。
  - agent 每个 terminal session 使用独立 PTY、读写线程和 wait 线程。
- 客户端/服务端边界：
  - Browser 只连接 server。
  - Agent 主动连接 server，不由 server SSH 推送。
- 数据流：
  - Browser 上传源码包到 server。
  - Agent 通过 HTTP 下载源码包。
  - Agent 通过 WebSocket 回传状态和日志。
  - Web terminal 通过 browser-server WebSocket 和 server-agent WebSocket 双跳转发输入输出。
- Web UI：
  - Builds tab 显示编译主流程：Submit Build、Runs、Builds、Agents、Run Log。
  - Upgrades tab 显示远程升级流程：升级包上传、升级 agent 选择、升级日志。
- 控制流：
  - server 根据 online agent 容量调度 queued run。
  - cancel/rerun 由 server API 触发。
  - run delete 由 server API 触发，server 通过 agent WebSocket 等待工作区删除确认后再删除 SQLite 记录。
  - terminal start/input/resize/close 由 browser WebSocket 触发，server 转发给在线 agent。
  - upgrade start 由 browser 上传包触发，server 保存包并下发给在线 agent，agent 下载校验后调用系统包管理器安装并请求重启 service。
- 外部依赖：
  - 无外部数据库或消息队列。
  - agent 执行脚本和 terminal shell 依赖本机 shell 和编译环境。

## 3. 关键接口

| 接口/协议/ABI | 调用方 | 提供方 | 兼容约束 | 说明 |
|---------------|--------|--------|----------|------|
| `/api/agent/ws` | agent | server | JSON text WebSocket | hello、computer_name、heartbeat、run_start、run_log、run_finished、run_cancel、run_delete、run_deleted、terminal_start/input/resize/close、terminal_output/exit、upgrade_start/status/log |
| `/api/ui/ws` | browser | server | JSON text WebSocket | 推送完整 UI state 和日志增量 |
| `/api/agents/{name}/terminal/ws` | browser | server | JSON text WebSocket | 为在线 agent 打开 PTY 终端会话并转发输入输出 |
| `POST /api/builds` | browser | server | multipart form | 上传 source，传 target agents/labels |
| `POST /api/upgrades` | browser | server | multipart form | 上传 deb/rpm/Gentoo overlay 包并推送给在线 agent |
| `GET /api/upgrades/{id}/package` | agent | server | agent name/token header | 下载已下发升级包 |
| `DELETE /api/builds/{id}` | browser | server | build with no runs only | 删除 server 侧 source 目录和 build 记录 |
| `DELETE /api/agents/{name}` | browser | server | offline agent only | 从 server 运行时 agent 列表删除离线 agent |
| `GET /api/runs/{id}/source` | agent | server | agent name/token header | 下载已分配源码包 |
| `GET /api/runs/{id}/log` | browser | server | text/plain | 读取 run 完整日志 |
| `DELETE /api/runs/{id}` | browser | server | terminal run only | 请求对应在线 agent 删除工作区，确认后删除 run 记录和日志 |
| `POST /api/runs/{id}/rerun` | browser | server | finished run only | 为同一 agent/source 新建 run |
| `POST /api/runs/{id}/cancel` | browser | server | active run | 下发 cancel 或直接标记 canceled |

## 4. 数据与配置

- 核心数据结构：
  - `builds`：build id、source name、archive format、source path、created_at、status。
  - `runs`：run id、build id、agent name、labels、status、exit_code、timestamps、source path、archive format、timeout。
- 配置文件/参数：
  - 支持 `-c <path>` / `--config <path>` 指定 INI 配置文件。
  - 不指定配置参数时自动发现默认配置。
  - Linux：`/etc/buildsvc/buildsvc.ini`。
  - Windows：`C:\ProgramData\buildsvc\buildsvc.ini`。
  - 开发 fallback：`./buildsvc.ini`。
  - 显式 `--config` 优先；未指定时系统服务路径优先，当前目录只作 fallback。
  - 项目内发布/打包样例：`configs/server.ini`、`configs/agent.ini`。
  - 项目内本地测试样例：`configs/server.test.ini`、`configs/agent.test.ini`。
  - `server.agent_heartbeat_sec` 默认 5 秒，由 server 在 `hello_accepted` 中下发给 agent。
  - `server.agent_offline_after_sec` 默认 15 秒，超时后 server 将 agent 从在线表移除，Web UI 显示 `offline`。
  - `server.terminal_enabled` 默认 `false`；关闭时 Web UI 不能打开 agent 终端。
  - `server.upgrade_enabled` 默认 `false`；关闭时 Web UI 不能推送远程包升级。
  - `agent.advertise_ip` 可选；多网卡或非标准网卡名机器建议显式配置。
  - 未配置 `agent.advertise_ip` 时，agent 枚举本机网卡，优先选择有线/无线物理网卡地址，并过滤 lo/docker/veth/tun/bridge 等虚拟接口。
  - `agent.terminal_enabled` 默认 `false`；打开后 agent hello 上报终端能力。
  - `agent.terminal_shell` 可选；未配置时 Linux/macOS 使用 `SHELL` 或 `/bin/sh`，Windows 使用 `COMSPEC` 或 `cmd.exe`。
  - `agent.terminal_work_dir` 默认 `<agent_work_dir>/terminal`。
  - `agent.terminal_max_sessions` 默认 1。
  - `agent.upgrade_enabled` 默认 `false`；打开后 agent hello 上报升级能力并可执行远程包升级。
  - `agent.upgrade_work_dir` 默认 `<agent_work_dir>/upgrades`。
- 持久化数据：
  - SQLite：`<server_data_dir>/buildsvc.db`。
  - 源码包：`<server_data_dir>/sources/<build_id>/source.*`。
  - 日志：`<server_data_dir>/logs/<run_id>.log`。
  - agent 工作目录：`<agent_work_dir>/runs/<run_id>/...`。
  - server 侧升级包：`<server_data_dir>/upgrades/<upgrade_id>/...`。
  - agent 侧升级包：`<agent_work_dir>/upgrades/<upgrade_id>/...`。
  - 删除 build 时，如果该 build 下无 run 记录，server 删除 `<server_data_dir>/sources/<build_id>` 和 `builds` 表记录。
  - 删除 run 时，agent 删除 `<agent_work_dir>/runs/<run_id>`，server 删除 `runs` 表记录和 `<server_data_dir>/logs/<run_id>.log`。
  - terminal session 当前不持久化命令记录和输出日志。
- Linux 包安装内容：
  - `/usr/bin/buildsvc`。
  - `/etc/buildsvc/buildsvc.ini`。
  - `/usr/lib/systemd/system/buildsvc.service`。
  - `/usr/share/doc/buildsvc/examples/server.ini` 和 `agent.ini`。
- 迁移/兼容规则：
  - 第一版没有 schema migration 框架，仅 `CREATE TABLE IF NOT EXISTS`。
- 敏感信息处理：
  - agent token 存在 INI 文件。
  - UI 不显示 token。
- Agent 状态：
  - agent hello 自动上报计算机名，优先读取 `COMPUTERNAME`、`HOSTNAME`，再读取 `/etc/hostname`，最后回退到 agent 配置名。
  - server 运行时保存最新计算机名；agent 离线后仍显示最新值，直到该 agent 被删除。
  - Agents UI 排序为在线优先，同组按计算机名排序。
  - 删除 offline agent 会从运行时白名单移除；不写回 INI，server 重启后配置内 agent 会重新出现。

## 5. 高风险区域

| 风险区域 | 关注点 | 验证方式 | 关联文档 |
|----------|--------|----------|----------|
| 并发/锁 | runtime mutex 与 SQLite mutex 的锁顺序 | `cargo test` 覆盖 storage 死锁回归；人工审查无 await 持锁跨网络 | docs/dev/1-plan-buildsvc-mvp.md |
| 协议 | WebSocket JSON 兼容和坏消息处理 | `cargo check`、人工审查错误路径 | docs/dev/1-plan-buildsvc-mvp.md |
| 权限/系统调用 | agent 运行用户脚本、PTY shell 并终止进程组/终端会话 | Linux 编译验证；Windows/macOS 需后续实机验证 | docs/dev/1-plan-buildsvc-mvp.md |
| Web 终端 | 键盘输入、resize、关闭会话和断线清理 | 本地 server/agent PTY smoke；Windows/macOS 需实机补测 | docs/dev/4-task-agent-terminal.md |
| 远程升级 | 系统包安装权限、配置文件保护、service 重启、升级过程中 WebSocket 断开 | Linux 编译验证；deb/rpm/Gentoo 需目标系统实机验证 | docs/dev/6-task-agent-package-upgrade.md |
| 文件系统 | archive 路径穿越和顶层目录约束 | archive 单元测试 | docs/dev/1-plan-buildsvc-mvp.md |
| 数据保留 | 日志保留和运行中任务重启恢复 | storage 测试和人工审查 | docs/dev/1-plan-buildsvc-mvp.md |

## 6. 构建与验证

- Release 构建：`make`
- Debug 构建：`make debug`
- 单元测试：`make test`
- Debian 包：`make deb`
- RPM 包：`make rpm`
- Gentoo overlay：`make emerge`
- 底层 Cargo 验证：`cargo fmt --check`、`cargo check`
- 格式检查：`cargo fmt --check`
- 静态检查：暂未启用 clippy 作为强制门禁。
- 高风险验证：
  - archive 路径校验单元测试。
  - storage create/rerun/log 单元测试。
  - 本地 server/agent/source upload smoke test。
  - Windows/macOS 执行器需实机补测。
- 最小人工验证步骤：
  - 准备 server/agent 两份 INI 配置。
  - 本地测试可启动 `buildsvc --config configs/server.test.ini` 和 `buildsvc --config configs/agent.test.ini`。
  - 上传包含顶层目录和 `run-build.sh` 的 `tar.gz`。
  - 在 Web UI 确认 agent online、run success、日志实时展示。
  - 已验证 run：`run_d27d216d7ddf4394a94789f90b1a546b`，日志包含 `smoke-start` 和 `smoke-ok`。

## 7. 发布与回滚

- 产物：`buildsvc` 单二进制。
- Linux 包产物：
  - deb：`target/package/buildsvc_<version>-1_<arch>.deb`。
  - rpm：`target/package/buildsvc-<version>-1.<arch>.rpm`。
  - Gentoo：`target/package/buildsvc-<version>-gentoo-overlay.tar.gz`，包含 `app-admin/buildsvc` ebuild overlay，可通过 `PORTDIR_OVERLAY=<overlay> emerge -av app-admin/buildsvc` 使用。
  - Gentoo ebuild 默认使用当前稳定架构 keyword（如 `amd64`、`arm64`）；如需 unstable keyword，可通过 `GENTOO_KEYWORDS='~amd64' make emerge` 覆盖。
- 安装/部署方式：可将二进制和 INI 配置手动放到目标机器，也可使用 deb/rpm/Gentoo overlay 包安装；Linux 包内置 systemd unit。
- 远程升级方式：在 server/agent 均启用 `upgrade_enabled=true` 后，通过 Web UI 上传 deb/rpm/Gentoo overlay 包并选择在线 agent；agent 下载校验后执行 apt/dpkg、dnf/yum/rpm 或 emerge 安装，并请求 `systemctl restart buildsvc`。
- 配置变更：修改 INI 后重启对应进程。
- 升级步骤：停止进程、替换二进制、启动进程。
- 回滚步骤：停止进程、换回旧二进制、启动进程。
- 止损条件：server 无法调度或 agent 异常执行时停止对应进程；未提交代码可直接回退工作区变更。

## 8. 观测与排障

- 关键日志：
  - 进程自身日志：stdout/stderr tracing 输出。
  - run 日志：server data dir 下 `logs/<run_id>.log`。
- 指标/告警：第一版无指标系统。
- 常见故障：
  - agent unknown/invalid token：检查 server `[agent.<name>]` 和 agent `[agent]` token。
  - agent IP 显示为 `-`：检查机器是否有可识别的有线/无线网卡，或在 agent 配置中设置 `advertise_ip`。
  - run failed：查看 run log 和脚本退出码。
  - source download failed：检查 `server.public_url` 是否对 agent 可达。
  - script missing：检查压缩包顶层目录内是否存在固定脚本。
- 排障入口：Web UI run detail、server/agent 进程日志、SQLite `runs` 表。

## 9. 文档索引

- 需求与任务索引：docs/dev/README.md
- 产品概览：docs/overview-product.md
- 架构笔记：docs/architecture.md
- 按需片段模板：.dj-agent/fragments/
- 关键任务文档：
  - docs/dev/1-research-buildsvc-mvp.md
  - docs/dev/1-plan-buildsvc-mvp.md

## 10. 变更记录

| 日期 | 变更 | 影响 | 关联文档 |
|------|------|------|----------|
| 2026-06-26 | 创建 Rust MVP 开发概览 | 明确模块、接口、数据、验证和部署边界 | docs/dev/1-*.md |
