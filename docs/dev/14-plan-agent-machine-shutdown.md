# Agent 远程关机入口开发计划

> 文档元数据
> - 文件编号：14
> - 文档类型：plan
> - 文件路径：docs/dev/14-plan-agent-machine-shutdown.md
> - 文档版本：v1.0.0
> - 最后更新：2026-06-29
> - 关联需求：在右侧 Agents 末尾增加“关闭机器”列，点击后直接关闭远程机器，且不显示横向滚动条。
> - 关联调研：[docs/dev/14-research-agent-machine-shutdown.md](14-research-agent-machine-shutdown.md)

## 1. 目标与成功标准

- 任务目标：在 Agents 表末尾提供在线 agent 机器关机入口，并通过现有 server-agent 通道触发。
- 成功标准：
  - Agents 表末尾有“关闭机器”列。
  - 在线 agent 的按钮点击后直接调用 server API 并向 agent 下发关机指令。
  - 离线 agent 按钮禁用。
  - Agents 表不显示横向滚动条。
  - Rust 格式检查和测试通过。
- 前置条件：server 和 agent 使用同版本协议。
- 非目标：不做权限探测、二次确认、批量操作或真实关机自动化测试。

## 2. 修改边界

- 最大修改范围：`src/protocol.rs`、`src/server/mod.rs`、`src/agent.rs`、`src/server/ui.rs`、总文档和本地任务文档。
- 禁止触碰范围：不执行真实关机命令；不修改存储 schema、打包脚本、配置格式。
- 影响模块/文件：
  - `protocol`：新增 server-to-agent 指令。
  - `server`：新增 REST API 并转发指令。
  - `agent`：新增平台关机命令调度。
  - `server/ui`：新增表格列、按钮点击逻辑和列宽控制。
- 依赖关系：无新增依赖。

## 3. 安全门禁摘要

| 项 | 结论 |
|----|------|
| 风险矩阵结论 | L3：API/协议变更和系统副作用 |
| 命令权限 | C0/C1 |
| 高风险开发门禁 | 是：系统副作用；代码审查和编译验证，不执行真实关机 |
| 破坏性操作 | 无 |
| 用户确认事项 | 无，用户明确要求直接关机 |
| 止损/回滚方案 | 回退本次代码和文档变更即可移除入口和协议 |

## 4. Bug 修复计划（按需）

不适用，需求实现。

## 5. 执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 新增 `ServerToAgent::ShutdownMachine`、`POST /api/agents/{id}/shutdown` 和 agent 关机命令处理 | `cargo fmt --check`、`cargo test` | 完成 |
| 2 | Agents 表末尾新增“关闭机器”列，按钮仅在线可点，点击后禁用避免重复发送 | 代码阅读、`cargo fmt --check`、`cargo test` | 完成 |
| 3 | 收窄 Agents 表列宽并隐藏横向溢出 | 静态检查列宽合计 98%，确认 `overflow-x: hidden` | 完成 |
| 4 | 更新总文档、Summary 和任务索引 | 人工检查链接和状态 | 完成 |

## 6. 验证计划

- 基础验证：`cargo fmt --check`、`cargo test`。
- 高风险验证（按需）：API/协议和错误路径人工审查；真实关机不在当前工作区执行。
- 验证环境：本地 Linux 工作区。
- 不可执行验证项：浏览器像素级横向滚动条检查、真实远程机器关机。
- 残余风险：agent 进程权限不足时系统关机命令可能无法真正关机；旧 agent 收到新指令会因协议不兼容断开。

## 7. 变更记录（按需）

| 日期 | 变更 | 原因 |
|------|------|------|
| 2026-06-29 | 从 L2 升级为 L3 | 实际实现涉及 REST API 和 server-agent 协议变更 |
