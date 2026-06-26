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
| 构建系统 | Makefile + Cargo | 构建、测试、格式化 | `make`、`make debug`、`make test` |
| 运行平台 | Linux、Windows、macOS | agent/server 目标平台 | 当前仅在 Linux 工作区自动验证 |
| HTTP/WebSocket | axum、tokio、tokio-tungstenite | server API/UI 和 agent WS client | WebSocket 消息为 JSON text |
| 持久化 | rusqlite + SQLite | build/run 元数据 | 日志和源码包放文件系统 |
| 解包 | tar、flate2、zip | `tar.gz`/`zip` source archive | 校验顶层唯一目录和路径安全 |

## 2. 架构边界

- 模块划分：
  - `config`：INI 发现、解析、role 分派配置。
  - `protocol`：agent/server/UI JSON 消息和 UI view model。
  - `storage`：SQLite schema、build/run 状态、日志文件。
  - `server`：HTTP API、Web UI、agent WebSocket、调度。
  - `agent`：WebSocket client、源码下载、解包、脚本执行、日志回传。
  - `archive`：跨格式解包与路径校验。
- 进程/线程边界：
  - server 和每个 agent 是独立进程。
  - server 使用 tokio 异步处理 HTTP/WS；SQLite 操作用 mutex 包装。
  - agent 每个 run 使用独立任务和独立工作目录。
- 客户端/服务端边界：
  - Browser 只连接 server。
  - Agent 主动连接 server，不由 server SSH 推送。
- 数据流：
  - Browser 上传源码包到 server。
  - Agent 通过 HTTP 下载源码包。
  - Agent 通过 WebSocket 回传状态和日志。
- 控制流：
  - server 根据 online agent 容量调度 queued run。
  - cancel/rerun 由 server API 触发。
- 外部依赖：
  - 无外部数据库或消息队列。
  - agent 执行脚本依赖本机 shell 和编译环境。

## 3. 关键接口

| 接口/协议/ABI | 调用方 | 提供方 | 兼容约束 | 说明 |
|---------------|--------|--------|----------|------|
| `/api/agent/ws` | agent | server | JSON text WebSocket | hello、computer_name、heartbeat、run_start、run_log、run_finished、run_cancel |
| `/api/ui/ws` | browser | server | JSON text WebSocket | 推送完整 UI state 和日志增量 |
| `POST /api/builds` | browser | server | multipart form | 上传 source，传 target agents/labels |
| `DELETE /api/agents/{name}` | browser | server | offline agent only | 从 server 运行时 agent 列表删除离线 agent |
| `GET /api/runs/{id}/source` | agent | server | agent name/token header | 下载已分配源码包 |
| `GET /api/runs/{id}/log` | browser | server | text/plain | 读取 run 完整日志 |
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
  - 项目内开发样例：`configs/server.ini`、`configs/agent.ini`。
  - `server.agent_heartbeat_sec` 默认 5 秒，由 server 在 `hello_accepted` 中下发给 agent。
  - `server.agent_offline_after_sec` 默认 15 秒，超时后 server 将 agent 从在线表移除，Web UI 显示 `offline`。
  - `agent.advertise_ip` 可选；多网卡或非标准网卡名机器建议显式配置。
  - 未配置 `agent.advertise_ip` 时，agent 枚举本机网卡，优先选择有线/无线物理网卡地址，并过滤 lo/docker/veth/tun/bridge 等虚拟接口。
- 持久化数据：
  - SQLite：`<server_data_dir>/buildsvc.db`。
  - 源码包：`<server_data_dir>/sources/<build_id>/source.*`。
  - 日志：`<server_data_dir>/logs/<run_id>.log`。
  - agent 工作目录：`<agent_work_dir>/runs/<run_id>/...`。
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
| 权限/系统调用 | agent 运行用户脚本并终止进程组 | Linux 编译验证；Windows/macOS 需后续实机验证 | docs/dev/1-plan-buildsvc-mvp.md |
| 文件系统 | archive 路径穿越和顶层目录约束 | archive 单元测试 | docs/dev/1-plan-buildsvc-mvp.md |
| 数据保留 | 日志保留和运行中任务重启恢复 | storage 测试和人工审查 | docs/dev/1-plan-buildsvc-mvp.md |

## 6. 构建与验证

- Release 构建：`make`
- Debug 构建：`make debug`
- 单元测试：`make test`
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
  - 启动 `buildsvc --config configs/server.ini` 和 `buildsvc --config configs/agent.ini`。
  - 上传包含顶层目录和 `run-build.sh` 的 `tar.gz`。
  - 在 Web UI 确认 agent online、run success、日志实时展示。
  - 已验证 run：`run_d27d216d7ddf4394a94789f90b1a546b`，日志包含 `smoke-start` 和 `smoke-ok`。

## 7. 发布与回滚

- 产物：`buildsvc` 单二进制。
- 安装/部署方式：将二进制和 INI 配置放到目标机器，service 文件由运维侧编写。
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
