# Agent 远程关机入口调研报告

> 文档元数据
> - 文件编号：14
> - 文档类型：research
> - 文件路径：docs/dev/14-research-agent-machine-shutdown.md
> - 文档版本：v1.0.0
> - 最后更新：2026-06-29
> - 关联需求：在右侧 Agents 末尾增加“关闭机器”列，点击后直接关闭远程机器，且不显示横向滚动条。

## 1. 问题与边界

- 问题描述：用户需要从 Web UI 的 Agents 表直接关闭在线 agent 所在机器。
- 调研目的：确认现有 server-agent 控制通道、UI 表格结构和系统调用边界，选择最小实现路径。
- 包含：server REST API、server-to-agent WebSocket 指令、agent 本机系统关机命令、Agents 表格入口和列宽调整。
- 不包含：二次确认弹窗、批量关机、权限配置、关机结果持久化、真实关机实测。
- 非目标：不改变 run、terminal、upgrade 的既有语义。
- 禁止触碰范围：不执行真实关机命令；不修改打包、部署脚本和无关模块。

## 2. 当前证据

- 现有实现/现状：
  - `src/protocol.rs` 已定义 `ServerToAgent` JSON text WebSocket 协议。
  - `src/server/mod.rs` 已通过 REST API 触发 cancel/delete/terminal/upgrade 等操作，并向在线 agent 转发指令。
  - `src/agent.rs` 已有升级时调用系统命令的执行风格。
  - `src/server/ui.rs` 的 Agents 表使用固定表格布局，原列宽合计 98%。
- 已知约束：
  - 关机是系统副作用，只能由用户点击后触发，验证阶段不执行。
  - 离线 agent 无法接收 WebSocket 指令。
  - 用户明确要求“点击后直接关闭”，不加确认弹窗。
- 关键日志/数据/用户反馈：用户要求新增列并避免横向滚动条。
- Bug 证据等级：不适用，需求实现。
- 证据不足项：无浏览器环境，无法做真实像素级滚动条验证。

## 3. 安全门禁摘要

| 项 | 结论 |
|----|------|
| 风险矩阵初判 | L3：新增 REST API 和 server-agent 协议指令，且触发系统副作用 |
| 命令权限 | C0/C1：只读检查、工作区内修改、Rust 验证 |
| 高风险开发门禁 | 是：系统副作用和协议变更；不执行真实关机 |
| 破坏性操作 | 代码修改本身无；真实关机未执行 |
| 用户已有修改 | 无，开始前 `git status --short` 为空 |
| 用户确认事项 | 无，用户已明确要求点击直接关机 |

## 4. 候选方案

| 方案 | 核心思路 | 优点 | 风险/代价 | 适用条件 |
|------|----------|------|-----------|----------|
| A | 复用现有 WebSocket，新增 `shutdown_machine` 指令 | 改动小，和现有控制流一致 | 新旧 agent 版本混用时旧 agent 不认识新指令 | 当前 server/agent 成对升级使用 |
| B | 通过 Web 终端执行关机命令 | 不改协议 | 依赖终端启用，交互路径不直观，无法形成稳定按钮语义 | 临时人工操作 |
| C | server SSH 到 agent 关机 | 不要求 agent 新协议 | 引入 SSH 凭据和平台差异，偏离现有 agent 主动连接模型 | 不适合当前架构 |

## 5. 推荐结论

- 推荐方案：采用方案 A，新增 `ServerToAgent::ShutdownMachine`，server 提供 `POST /api/agents/{id}/shutdown`，agent 按平台调度本机关机命令。
- 取舍理由：最贴合现有 browser -> server -> agent 控制链路，改动集中且不新增依赖。
- 需要进入 Plan 的关键约束：按钮只对在线 agent 启用；点击后直接发送请求；Agents 表列宽总和保持低于 100% 并隐藏横向溢出。
- 需要用户确认的问题：无。
- 后续验证方向：`cargo fmt --check`、`cargo test`、静态检查 UI 列宽；真实关机需在目标测试机手工验证。

## 6. 参考资料（按需）

- 本地源码：`src/protocol.rs`、`src/server/mod.rs`、`src/agent.rs`、`src/server/ui.rs`
