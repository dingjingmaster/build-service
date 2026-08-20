# 取消构建脚本超时限制计划

> 文档元数据
> - 文件编号：23
> - 文档类型：plan
> - 文件路径：docs/dev/23-plan-unlimited-script-runtime.md
> - 日期：2026-08-20
> - 需求级别：L3

## 目标

构建脚本不再受 `script_timeout_sec` 限制；agent 仅在脚本自然结束、用户取消或连接断开取消时结束 run。

## 执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 增加 agent 脚本自然完成的回归测试 | 定向 `cargo test` 先失败，旧实现返回 124 | 完成 |
| 2 | 移除配置和协议中的 `script_timeout_sec` | `cargo test` 编译检查 | 完成 |
| 3 | 移除 agent 超时分支，保留 cancel 终止流程 | agent 定向测试、全量测试 | 完成 |
| 4 | storage 不再暴露 timeout 业务字段，写入兼容列常量 0 | storage 现有测试 | 完成 |
| 5 | 更新配置样例、README 和总文档 | `rg script_timeout_sec`、人工审查 | 完成 |
| 6 | 完整验证和总结 | `cargo fmt --check`、`cargo test`、`git diff --check` | 完成 |

## 边界与兼容

- 不删除 SQLite 旧列，不迁移历史数据库。
- 不修改 `kill_grace_sec`：取消 run 时仍需要优雅终止等待时间。
- 不改变 agent concurrency、heartbeat、offline timeout。

## 审视

- 安全：保留 Cancel 的进程组终止能力；不执行破坏性命令。
- 产品：满足“允许运行任何时间长度”，代价是异常卡死任务需人工取消。
- 架构：协议字段移除要求 server/agent 同步升级；SQLite 列保留避免复杂迁移。
