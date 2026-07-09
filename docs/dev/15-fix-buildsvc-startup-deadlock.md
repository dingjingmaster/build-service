# buildsvc server 启动前自锁修复记录

> 文档元数据
> - 文件编号：15
> - 文档类型：fix
> - 文件路径：docs/dev/15-fix-buildsvc-startup-deadlock.md
> - 完成日期：2026-07-09
> - 需求级别：L2

## 1. 问题定义

- 问题现象：`buildsvc.service` 显示 active running，但没有 `buildsvc server listening` 日志，`0.0.0.0:10000` 未监听，访问返回 connection refused。
- 预期行为：配置为 `role = server` 时，server 启动后应完成启动期 run 状态恢复并监听配置端口。
- 影响范围：server 启动和 agent 断开后的 active run 状态清理。
- 不包含：配置文件路径、systemd unit、端口、防火墙、SQLite schema 迁移策略调整。

## 2. 证据与根因

- 复现方式：服务重启后 systemd 进程存在但无监听；日志中重启后没有 `buildsvc server listening`；strace attach 显示线程等待 futex。
- 证据等级：E2
- 关键日志/堆栈/输入：用户提供的 journal 显示 2026-07-09 11:45 后只有 systemd started，无应用启动完成日志；strace 显示 33 个线程等待 futex。
- 根因：`mark_active_runs_lost()` 持有 SQLite `MutexGuard` 收集 active run id 后，在 guard 仍未释放时调用 `update_run_status()`；后者再次请求同一把 mutex，遇到 active run 时自锁。`mark_agent_runs_lost()` 有相同模式。
- 相关代码路径：`src/storage.rs`

## 3. 修复方案

- 最小修复点：在遍历 active run 并调用 `update_run_status()` 前显式释放 SQLite statement 和 mutex guard。
- 代码逻辑改动：`drop(stmt); drop(conn);` 后再循环更新 run 状态。
- 影响的使用场景：server 启动时存在 `assigned`、`preparing`、`running` run；agent 断开时存在对应 active run。
- 不影响的使用场景：无 active run 的启动；普通 run 创建、更新、删除；配置读取路径。

## 4. 修复执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 记录问题和计划 | 文档与索引存在 | 完成 |
| 2 | 修复 storage active-run 标记自锁 | 代码审查 | 完成 |
| 3 | 添加回归测试覆盖 active run 标记 | `cargo test deadlock` | 完成 |
| 4 | 运行相关 Rust 验证 | `cargo fmt --check`、`cargo test`、`cargo build --release` | 完成 |

## 5. 风险摘要

| 项 | 结论 |
|----|------|
| 风险矩阵 | L2 |
| 命令权限 | C1 |
| 高风险门禁 | 是：并发/锁相关修复 |
| 破坏性操作 | 否 |
| 用户已有修改 | 否 |
| 用户确认 | 无 |
| 副作用/风险 | 低；仅提前释放 mutex，不改变状态机语义 |

## 6. 验证

- 验证环境：本地 Linux，Rust/Cargo 项目测试环境。
- 回归/相关验证：`cargo test deadlock`、`cargo fmt --check`、`cargo test`、`cargo build --release`。
- 结果：通过；完整单元测试 26 passed；release 构建完成。
- 未执行验证项：未在 root systemd 服务上安装并重启验证。
- 残余风险：需要将修复后的二进制部署到目标机器后再重启服务确认 `0.0.0.0:10000` 监听恢复。

## 7. 修复总结

- 最终结果：释放 SQLite mutex guard 后再更新 active run 状态，修复 server 启动和 agent 断开路径的持锁重入死锁。
- 计划偏差：无。
- 后续建议：部署修复后二进制，重启 `buildsvc.service`，检查日志出现 `buildsvc server listening addr=0.0.0.0:10000`。
