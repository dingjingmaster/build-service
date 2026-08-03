# Agent 关机权限配置实施计划

> 文档元数据
> - 文件编号：17
> - 文档类型：plan
> - 关联调研：[17-research-agent-shutdown-permission.md](17-research-agent-shutdown-permission.md)

## 1. 成功标准

- `[agent].shutdown_enabled` 可解析，缺省为 `false`。
- agent Hello 上报该配置；server 状态向 UI 暴露该能力。
- 未授权或离线 agent 的关闭机器按钮置灰。
- 未授权 agent 调用关闭 API 时被 server 拒绝且不发送关机指令。
- 授权且在线 agent 保持现有关机流程。
- `cargo fmt --check` 和 `cargo test` 通过。

## 2. 修改范围与门禁

- 预计文件：`src/config.rs`、`src/protocol.rs`、`src/agent.rs`、`src/server/mod.rs`、`src/server/ui.rs`、`configs/*.ini`、`README*.md`、产品/开发总文档及本地任务文档。
- 不执行真实关机，不修改数据库 schema、打包脚本和无关功能。
- 高风险项：配置/协议兼容性与系统副作用；旧 agent 缺省为不允许关机。

## 3. 执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 配置字段、示例和文档 | 配置单测、文档检查 | 完成 |
| 2 | Hello/运行时/UI 状态传递与 API 权限校验 | `cargo test`、代码审查 | 完成 |
| 3 | 更新任务进度与总结，执行完整验证 | `cargo fmt --check`、`cargo test`、`git diff --check` | 完成 |

## 4. 未执行验证与残余风险

- 不执行真实关机，避免改变当前开发机状态。
- 无浏览器自动化环境时不做像素级按钮检查，以 HTML 条件和 `AgentView` 数据静态审查替代。
- 关机最终是否成功仍取决于 agent 进程的系统权限。
