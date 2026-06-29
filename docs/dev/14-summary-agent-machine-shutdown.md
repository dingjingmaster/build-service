# Agent 远程关机入口开发结论摘要

> 文档元数据
> - 文件编号：14
> - 文档类型：summary
> - 文件路径：docs/dev/14-summary-agent-machine-shutdown.md
> - 文档版本：v1.0.0
> - 完成日期：2026-06-29
> - 关联需求：在右侧 Agents 末尾增加“关闭机器”列，点击后直接关闭远程机器，且不显示横向滚动条。
> - 关联调研：[docs/dev/14-research-agent-machine-shutdown.md](14-research-agent-machine-shutdown.md)
> - 关联计划：[docs/dev/14-plan-agent-machine-shutdown.md](14-plan-agent-machine-shutdown.md)

## 1. 最终结果

- 原始需求：右侧 Agents 末尾增加“关闭机器”列，点击后直接关闭远程机器，不出现横向滚动条。
- 最终方案：新增 browser REST API -> server-agent WebSocket -> agent 本机系统关机命令链路；UI 新增末尾列和按钮。
- 完成状态：完成。
- 需求变更：按项目路由从 L2 升级为 L3，因为涉及 REST API 和 server-agent 协议变更。

## 2. 关键改动

- 修改文件：
  - `src/protocol.rs`
  - `src/server/mod.rs`
  - `src/agent.rs`
  - `src/server/ui.rs`
  - `docs/overview-product.md`
  - `docs/overview-product-dev.md`
  - `docs/dev/README.md`
- 代码逻辑改动：
  - `ServerToAgent` 新增 `ShutdownMachine`。
  - server 新增 `POST /api/agents/{agent_id}/shutdown`，只向在线 agent 下发指令。
  - agent 收到指令后按平台调度关机命令：Linux `systemctl --no-block poweroff` 或 `shutdown -h now`，macOS `shutdown -h now`，Windows `shutdown /s /t 0 /f`。
  - Agents 表新增“关闭机器”列，离线 agent 禁用；点击后直接发送请求并暂时禁用该 agent 按钮，避免重复点击。
  - Agents 表列宽调整为合计 98%，并设置 `overflow-x: hidden`。
- 影响的使用场景：在线 agent 可从 Web UI 直接触发机器关机。
- 不影响的使用场景：构建提交、run 删除/重跑/取消、终端、远程升级、存储 schema。
- 计划偏差：初始按 L2 记录，复核发现协议/API 变更后升级并补齐 L3 文档。

## 3. 安全门禁结果

| 项 | 结论 |
|----|------|
| 风险矩阵 | L3 |
| 命令权限 | C0/C1 |
| 高风险项 | 有：系统副作用和协议变更；未执行真实关机 |
| 破坏性操作 | 无 |
| 用户已有修改 | 无 |
| 用户确认事项 | 无，用户明确要求直接关机 |
| 副作用/风险 | agent 进程权限不足会导致关机命令无法真正生效；旧 agent 不兼容新指令 |

## 4. 验证结果

- 验证环境：本地 Linux 工作区。
- 系统信息（按需）：Rust/Cargo 使用当前项目工具链。
- 执行验证：
  - `cargo fmt --check`
  - `cargo test`
  - 静态 UI 检查：Agents 表列宽合计 98%，`agents-table-scroll` 设置 `overflow-x: hidden`。
- 结果：通过；`cargo test` 24 个测试全部通过。
- 未执行验证项：
  - 未执行真实远程关机，避免破坏当前机器/环境。
  - 本地无 Chromium，未做浏览器像素级横向滚动条检查。
- 残余风险：系统关机命令是否成功取决于 agent 运行权限和目标系统策略。

## 5. Bug 修复验证（按需）

不适用，需求实现。

## 6. 后续事项（按需）

- 技术债：若后续需要跨版本滚动升级，可为 agent hello 增加能力位后再按能力显示关机按钮。
- 后续建议：在专用测试机上手工验证 Linux/macOS/Windows 的真实关机权限和效果。
- 关联文档/提交：未提交，当前任务未要求 git commit。
