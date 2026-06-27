# buildsvc MVP 开发结论摘要

> 文档元数据
> - 文件编号：1
> - 文档类型：summary
> - 文件路径：docs/dev/1-summary-buildsvc-mvp.md
> - 文档版本：v1.0.0
> - 完成日期：2026-06-26
> - 关联需求：自用 Rust 自动编译分发系统 MVP。
> - 关联调研：[docs/dev/1-research-buildsvc-mvp.md](1-research-buildsvc-mvp.md)
> - 关联计划：[docs/dev/1-plan-buildsvc-mvp.md](1-plan-buildsvc-mvp.md)

## 1. 最终结果

- 原始需求：实现 Rust 自研轻量自动编译系统，单二进制 server/agent，无运行参数，INI 配置，Web UI，WebSocket 实时 agent 状态与日志。
- 最终方案：
  - 一个 `buildsvc` Rust 二进制，根据 INI `[core].role` 进入 server 或 agent。
  - server 使用 axum 提供 Web UI、REST API、agent/UI WebSocket，使用 SQLite 记录 build/run 元数据，文件系统保存源码包和日志。
  - agent 使用 WebSocket 接收 run，通过 HTTP 下载源码包，解包后执行固定脚本并回传日志和退出码。
- 完成状态：完成 MVP 代码、自动化验证和本地跨进程 server/agent smoke test。
- 需求变更：
  - 上传源码包改为流式写入磁盘，避免大包整体进入内存。
  - 支持 `-c` / `--config` 指定配置文件；不指定时配置发现顺序为系统服务路径优先，`./buildsvc.ini` 仅作为开发 fallback。

## 2. 关键改动

- 修改文件：
  - `Cargo.toml`、`Cargo.lock`
  - `.gitignore`
  - `buildsvc.ini.example`
  - `configs/server.ini`、`configs/agent.ini`
  - `src/**`
  - `docs/architecture.md`
  - `docs/overview-product.md`
  - `docs/overview-product-dev.md`
  - `docs/dev/**`
- 代码逻辑改动：
  - 新增 INI parser 和配置模型。
  - 新增轻量 CLI parser，支持 `-c` / `--config`。
  - agent heartbeat 改为使用 server 下发的 `heartbeat_sec`。
  - server 默认 `agent_offline_after_sec` 调整为 15 秒，超时后 Web UI 显示 agent `offline`。
  - agent hello 自动上报计算机名，Agents 表新增计算机名列。
  - Agents 列表排序调整为在线优先，再按计算机名排序。
  - server/UI 支持删除 offline agent。
  - 新增 JSON WebSocket protocol。
  - 新增 SQLite schema 与 storage API。
  - 新增 server Web UI、upload、download、rerun、cancel、agent/ws、ui/ws。
  - 新增 agent WebSocket client、heartbeat、source download、archive extraction、script execution、log streaming、cancel handling。
  - 新增 archive 解包和 unsafe path 校验；后续已放宽为支持脚本位于压缩包根目录或唯一顶层目录。
- 影响的使用场景：
  - 用户可通过 Web UI 上传源码包并选择 agent 或 labels。
  - agent 可执行固定脚本并实时回传日志。
- 不影响的使用场景：
  - 不涉及现有业务代码，因为仓库此前无实现。
- 计划偏差：
  - Windows/macOS 原生执行器仍未实机验证。

## 3. 安全门禁结果

| 项 | 结论 |
|----|------|
| 风险矩阵 | L3 |
| 命令权限 | C0/C1；未执行 C2/C3 操作 |
| 高风险项 | 有：异步并发、协议、持久化、进程管理 |
| 破坏性操作 | 无 |
| 用户已有修改 | 有：保留并扩展前序 `docs/architecture.md` |
| 用户确认事项 | 无 |
| 副作用/风险 | 运行脚本无隔离，仅适合可信局域网和可信源码包 |

## 4. 验证结果

- 验证环境：
  - 当前 Linux 工作区。
  - Rust 1.93.1，Cargo 1.93.1。
- 执行验证：
  - `cargo fmt --check`
  - `cargo test`
  - `cargo check`
  - `cargo build`
  - `cargo run -- --help`
  - `target/debug/buildsvc --config /tmp/buildsvc-cli-smoke-server-19080.ini`
  - 本地 server/agent/source upload smoke test
  - 本地 heartbeat/offline smoke test
  - 本地 agent computer name/delete smoke test
- 结果：
  - `cargo fmt --check` 通过。
  - `cargo test` 通过，10 个测试全部通过。
  - `cargo check` 通过。
  - `cargo build` 通过。
  - `cargo run -- --help` 输出 `-c, --config <path>` 用法。
  - `target/debug/buildsvc --config /tmp/buildsvc-cli-smoke-server-19080.ini` 成功监听 `127.0.0.1:19080`，`GET /` 和 `/api/state` 返回成功。
  - smoke test 通过：临时 agent `local-linux` online，上传 `source.tar.gz` 后 run `run_d27d216d7ddf4394a94789f90b1a546b` 成功，exit code 为 `0`，日志包含 `smoke-start` 和 `smoke-ok`。
  - heartbeat/offline smoke test 通过：server 配置 `agent_heartbeat_sec=1`、`agent_offline_after_sec=3`，agent 本地配置 `heartbeat_sec=99`，连接后日志显示采用 server 下发的 `heartbeat_sec=1`；停止 agent 后 server `/api/state` 在约 4 秒内显示 `offline`。
  - agent computer name/delete smoke test 通过：agent 以 `HOSTNAME=zz-computer` 启动后 server `/api/state` 显示 `computer_name=zz-computer`；agent 停止并变为 `offline` 后仍保留该计算机名；调用 `DELETE /api/agents/local-linux` 后 Agents 列表为空。
- 未执行验证项：
  - Windows/macOS 原生 agent 执行器验证。
- 残余风险：
  - Windows 取消使用 `taskkill`，未在 Windows 实机验证。
  - macOS Unix process group 行为未在 macOS 实机验证。
  - SQLite schema 第一版未提供迁移框架。
  - Web UI 无登录，必须限制在可信网络。

## 5. Bug 修复验证

不适用。

## 6. 后续事项

- 技术债：
  - 增加跨进程集成测试或可重复 smoke 脚本。
  - Windows/macOS 实机验证进程取消和固定脚本执行。
  - 后续实现断点续传。
  - 后续如需要 artifact 管理，再扩展上传/下载接口。
- 后续建议：
  - 先用当前 MVP 在一台 server + 一台 Linux agent 上做局域网试跑。
  - service 文件可以在 MVP 试跑稳定后再补。
  - 对外暴露前必须增加 Web UI 登录或网络访问控制。

## 7. 多角色审视

- 安全审查：
  - 未执行破坏性命令、Git 提交或系统级 service 修改。
  - 工作区已有 `docs/architecture.md` 被保留并增量更新。
  - 高风险项为异步并发、协议、持久化和进程管理；已通过 `cargo test`、`cargo check` 和人工审查覆盖基础错误路径。
  - 仍需 Windows/macOS 实机验证进程取消行为。
- 产品审查：
  - MVP 聚焦“分发源码包、固定脚本执行、实时日志和结果”，未扩展 workflow、artifact、登录等非目标功能。
  - Web UI 已覆盖 agent 列表、上传、build/run 列表、日志、rerun/cancel。
- 架构审查：
  - server/agent/protocol/storage/archive/config 边界清晰。
  - 大文件上传使用流式落盘，避免内存放大。
  - 单 server + SQLite 是当前自用范围内的可接受单点。
- 工程审查：
  - `cargo fmt --check`、`cargo test`、`cargo check` 和本地 server/agent smoke test 均通过。
  - storage 测试发现并修复了 SQLite mutex 重入死锁。
  - 未引入前端构建链，Web UI 由 server 内嵌提供。
