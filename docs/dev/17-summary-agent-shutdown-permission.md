# Agent 关机权限配置实现总结

> 文档元数据
> - 文件编号：17
> - 文档类型：summary
> - 关联调研：[17-research-agent-shutdown-permission.md](17-research-agent-shutdown-permission.md)
> - 关联计划：[17-plan-agent-shutdown-permission.md](17-plan-agent-shutdown-permission.md)

## 1. 最终结果

- 新增 `[agent].shutdown_enabled`，默认 `false`。
- agent 通过 Hello 上报关机能力，server 保存并在 `AgentView` 暴露。
- Agents 表中的关闭按钮仅在 agent 在线且允许关机时启用；未允许或离线时置灰。
- server 关闭 API 对未授权 agent 拒绝请求；agent 收到未授权关机指令时也拒绝执行。
- 已更新中英文 README、产品/开发总文档、配置示例和本地任务索引。

## 2. 验证结果

- 环境：本地 Linux 工作区，Rust/Cargo 当前项目工具链。
- `cargo fmt --check`：通过。
- `cargo test`：通过，26 个测试全部通过。
- `git diff --check`：通过。
- 未执行真实关机；未做浏览器像素级检查，避免系统副作用和环境依赖。

## 3. 审视结论与残余风险

- 安全：默认关闭；UI、server API、agent 三层均有授权约束，未执行破坏性关机。
- 兼容性：Hello 新字段使用 serde 默认值，旧 agent 未上报时按不允许处理。
- 产品：满足配置不允许时按钮置灰且 service 无法关闭该 agent 的要求。
- 残余风险：实际关机仍取决于 agent 进程的系统权限和目标平台策略。

