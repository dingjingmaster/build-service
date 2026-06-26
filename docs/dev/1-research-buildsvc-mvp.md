# buildsvc MVP 调研报告

> 文档元数据
> - 文件编号：1
> - 文档类型：research
> - 文件路径：docs/dev/1-research-buildsvc-mvp.md
> - 文档版本：v1.0.0
> - 最后更新：2026-06-26
> - 关联需求：自用 Rust 自动编译分发系统，单二进制 server/agent，INI 配置，WebSocket 实时状态，Web UI。

## 1. 问题与边界

- 问题描述：需要一套轻量自用构建分发系统，在主机器上传源码包，选择 agent 执行固定脚本，并在 server Web UI 实时查看 agent 状态、日志与执行结果。
- 调研目的：确定第一版最小可实现架构、模块边界、协议与持久化方式。
- 包含：Rust 单二进制、INI 配置自动决定角色、server Web UI、agent WebSocket、源码包上传/下载、固定脚本执行、日志回传、失败/丢失任务手动重跑。
- 不包含：Jenkins/Gitea/K8s/Nomad 等外部 CI 平台、复杂 workflow、artifact 管理、断点续传、Web UI 登录。
- 非目标：server 不理解构建逻辑，agent 不实现平台编译策略，脚本内部操作由用户自行控制。
- 禁止触碰范围：Git 历史、系统级 service 文件安装、系统目录写入、用户未授权的删除或清理。

## 2. 当前证据

- 现有实现/现状：仓库仅有架构笔记，无 Rust 项目骨架。
- 已知约束：
  - 运行可不传参数，读取固定路径或开发 fallback 的 `buildsvc.ini`；也支持 `--config` 指定配置文件。
  - 配置文件为 INI，包含全量配置，按 role 读取相关段。
  - 平台暂定 Linux、Windows、macOS。
  - 源码包支持 `tar.gz` 和 `zip`，顶层必须只有一个目录。
  - Linux/macOS 执行 `run-build.sh`，执行前赋予可执行权限；Windows 执行 `run-build.bat`。
  - agent 可并发构建，每个 run 独立目录。
  - 日志保存一周。
  - agent 断线：server 标记 run 为 `lost`，agent 杀掉本地子进程。
  - 每个 agent 独立 token；Web UI 不登录。
  - 第一版不做断点续传，后续记录。
- 关键日志/数据/用户反馈：不适用，需求实现。
- Bug 证据等级：不适用。
- 证据不足项：真实 Windows/macOS 环境验证暂不可在当前 Linux 工作区完成。

## 3. 安全门禁摘要

| 项 | 结论 |
|----|------|
| 风险矩阵初判 | L3 |
| 命令权限 | C0/C1；如需要下载依赖则按环境审批升级 |
| 高风险开发门禁 | 是：Rust 异步并发、协议、配置、持久化、进程管理 |
| 破坏性操作 | 否 |
| 用户已有修改 | 有：`docs/architecture.md` 为本轮前序设计结果，继续在此基础上编辑 |
| 用户确认事项 | 无，用户已确认关键产品取舍 |

## 4. 候选方案

| 方案 | 核心思路 | 优点 | 风险/代价 | 适用条件 |
|------|----------|------|-----------|----------|
| A | Rust + axum + WebSocket + SQLite + 文件存储 | 单二进制，依赖少，Web UI/API/WS 在同一进程，状态可持久化 | 需要实现基本调度、协议和执行器 | 适合当前自用 MVP |
| B | Rust + SSH 推送执行 | agent 端无需服务 | Windows/macOS 差异大，日志实时性和取消复杂，密钥管理麻烦 | 不适合用户要求的 agent 状态实时展示 |
| C | Rust + 消息队列/外部数据库 | 调度能力强 | 依赖增加，不符合简单自用要求 | 后续规模扩大时再考虑 |

## 5. 推荐结论

- 推荐方案：方案 A。
- 取舍理由：
  - agent 主动 WebSocket 连接 server，避免 server 主动 SSH 和跨平台连接问题。
  - 大文件仍走 HTTP，WebSocket 只承载控制、心跳、状态与日志 JSON。
  - SQLite 保存 build/run 元数据，日志与源码包保存文件系统，降低数据库膨胀。
  - Web UI 直接由 server 提供，避免额外前端构建工具。
- 需要进入 Plan 的关键约束：
  - 纯 JSON WebSocket 消息。
  - 只用 labels 和显式 agent 名选择目标。
  - 第一版产物管理和断点续传延后。
  - 跨平台进程树清理实现要分平台处理，当前 Linux 环境只能完整验证 Unix 路径。
- 需要用户确认的问题：无。
- 后续验证方向：`cargo fmt --check`、`cargo test`、`cargo check`，本地 server/agent smoke test。

## 6. 参考资料

- [docs/architecture.md](../architecture.md)
