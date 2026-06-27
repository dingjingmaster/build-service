# buildsvc MVP 开发计划

> 文档元数据
> - 文件编号：1
> - 文档类型：plan
> - 文件路径：docs/dev/1-plan-buildsvc-mvp.md
> - 文档版本：v1.0.0
> - 最后更新：2026-06-26
> - 关联需求：自用 Rust 自动编译分发系统 MVP。
> - 关联调研：[docs/dev/1-research-buildsvc-mvp.md](1-research-buildsvc-mvp.md)

## 1. 目标与成功标准

- 任务目标：实现一个可运行的 Rust 单二进制 MVP，启动时读取 INI 配置，根据 role 进入 server 或 agent 模式，支持 Web UI 上传源码、选择 agent、下发执行、实时查看状态和日志。
- 成功标准：
  - 无命令行参数即可启动，能发现 `./buildsvc.ini` 作为开发 fallback；也可通过 `-c`/`--config` 指定配置文件。
  - server 能初始化数据目录和 SQLite schema。
  - agent 能通过 WebSocket hello/heartbeat 出现在 Web UI。
  - Web UI 能上传 `tar.gz`/`zip`，选择 agent 或 labels 创建 run。
  - agent 能下载源码包、校验安全路径、定位固定脚本、运行固定脚本、回传日志和退出码。
  - failed/lost/canceled run 支持手动 rerun。
  - 日志保留策略代码存在，默认 7 天。
- 前置条件：Cargo 可以解析依赖；如依赖下载被沙箱网络限制，需要按权限规则请求用户确认。
- 非目标：artifact 管理、断点续传、Web UI 登录、多 server 高可用、复杂 workflow。

## 2. 修改边界

- 最大修改范围：
  - Rust 项目骨架和源码。
  - 配置样例。
  - 架构/产品/开发文档。
- 禁止触碰范围：
  - Git 历史、提交、推送。
  - 系统 service 安装和系统目录写入。
  - 无关 `.dj-agent` 规则文件。
- 影响模块/文件：
  - `Cargo.toml`
  - `src/**`
  - `buildsvc.ini.example`
  - `configs/*.ini`
  - `docs/**`
- 依赖关系：
  - `tokio` 异步运行时。
  - `axum` HTTP/WebSocket/Web UI。
  - `rusqlite` SQLite。
  - `serde`/`serde_json` 协议与 API。
  - `tokio-tungstenite` agent WebSocket client。
  - `reqwest` agent HTTP download。
  - `tar`/`flate2`/`zip` 解包。

## 3. 安全门禁摘要

| 项 | 结论 |
|----|------|
| 风险矩阵结论 | L3 |
| 命令权限 | C0/C1；依赖下载失败后重试需用户确认 |
| 高风险开发门禁 | 是：异步并发、协议、配置、持久化、进程管理 |
| 破坏性操作 | 否 |
| 用户确认事项 | 无 |
| 止损/回滚方案 | 未提交前可直接撤销本次新增文件；不执行破坏性 Git 命令 |

## 4. Bug 修复计划

不适用。

## 5. 执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 创建 Rust 项目骨架、依赖、模块结构和配置样例 | `cargo check` 到达依赖/语法阶段 | 完成 |
| 2 | 实现 INI 配置发现与解析、role 分派、基础错误处理 | 单元测试覆盖 INI 解析 | 完成 |
| 3 | 实现 SQLite schema、数据目录、server 状态模型和基础 API | storage 单元测试、`cargo check` | 完成 |
| 4 | 实现 agent WebSocket hello/heartbeat、server 认证、状态广播、Web UI agent 列表 | 本地 server/agent smoke test | 完成 |
| 5 | 实现上传源码、选择 agent/labels、创建 build/run、HTTP source download | archive format 单元测试、`cargo check`；上传改为流式落盘 | 完成 |
| 6 | 实现 agent 解包、固定脚本执行、日志 JSON 回传、状态/退出码更新 | archive 路径校验单元测试、本地 tar.gz smoke test | 完成 |
| 7 | 实现 cancel/rerun、断线 lost、日志保留清理入口和 Web UI 展示 | storage rerun/log 测试、人工审查状态流 | 完成 |
| 8 | 更新总览文档、Summary、运行格式化和验证命令 | `cargo fmt --check`、`cargo test`、`cargo check` | 完成 |

## 6. 验证计划

- 基础验证：
  - `cargo fmt --check`
  - `cargo test`
  - `cargo check`
  - 本地 server/agent smoke test：启动 server、启动 agent、上传包含 `run-build.sh` 的 `tar.gz`，观察 run success 和日志。
- 高风险验证：
  - 并发：至少检查状态锁不在 `.await` 跨越期间持有过长时间。
  - 协议：WebSocket 消息反序列化失败不应导致 server 崩溃。
  - 错误路径：上传格式错误、token 错误、脚本缺失应进入 failed 或拒绝。
  - 进程管理：当前 Linux 环境验证 Unix 路径；Windows/macOS 记录未覆盖风险。
- 验证环境：当前工作区 Linux。
- 本地 smoke test：
  - server 监听 `127.0.0.1:18080`。
  - 临时 agent `local-linux` 通过 WebSocket 注册。
  - 上传包含 `run-build.sh` 的 `tar.gz`。
  - run `run_d27d216d7ddf4394a94789f90b1a546b` 成功，exit code 为 `0`，日志包含 `smoke-start` 和 `smoke-ok`。
- 不可执行验证项：Windows/macOS 原生执行器验证。
- 残余风险：第一版不提供强隔离，脚本拥有 agent 进程权限；仅适合可信局域网和可信源码包。

## 7. 变更记录

| 日期 | 变更 | 原因 |
|------|------|------|
| 2026-06-26 | 创建计划 | 启动 MVP 实现 |
