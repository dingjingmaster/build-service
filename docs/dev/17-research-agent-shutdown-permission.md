# Agent 关机权限配置调研

> 文档元数据
> - 文件编号：17
> - 文档类型：research
> - 关联需求：agent 配置是否允许 service 关闭 agent 所在机器；未允许时 UI 按钮置灰且 server 拒绝请求。

## 1. 现状与边界

- 现有链路为 Web UI -> `POST /api/agents/{id}/shutdown` -> server-agent WebSocket `shutdown_machine` -> agent 本机系统关机命令。
- 当前 `AgentConfig` 没有关机权限字段，server 只按在线状态发送关机指令，UI 只按在线状态启用按钮。
- 本次包含：agent INI 配置、Hello 能力上报、server 运行时权限状态、REST 防绕过校验、Agents 表按钮状态、配置示例和说明。
- 本次不包含：真实关机测试、权限提升、二次确认、批量关机、存储 schema 变更。

## 2. 推荐方案

- 新增 `[agent].shutdown_enabled`，默认 `false`。
- agent 在 Hello 中上报 `shutdown_enabled`；server 保存到在线运行时状态并映射到 `AgentView`。
- UI 仅当 agent 在线且 `shutdown_enabled=true` 时启用按钮。
- `shutdown_agent` API 在发送指令前再次检查该状态，未授权时返回错误，避免直接调用 API 绕过 UI。
- 旧 agent 未上报该字段时按 serde 默认值 `false` 处理，保持安全兼容。

## 3. 风险与验证

- 命中配置格式、WebSocket 协议和系统副作用门禁，按 L3 执行。
- 不执行真实关机命令；通过配置解析、Rust 测试、格式检查和代码审查验证拒绝路径。
- 回滚方式为回退本次配置字段、协议字段、server/UI 校验和文档变更。

