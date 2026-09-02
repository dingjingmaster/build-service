# Run log 写入 tmpfs 任务记录

> 文档元数据
> - 文件编号：24
> - 文档类型：task
> - 文件路径：docs/dev/24-task-run-log-tmpfs.md
> - 文档版本：v1.0.0
> - 最后更新：2026-09-02
> - 需求级别：L2
> - 关联需求：将 server 端编译日志输出改到 `/tmp`，避免重复写固态硬盘。

## 1. 目标

- 要解决的问题：server 端 run 日志默认写入 `<core.data_dir>/logs`，生产配置下位于 `/var/lib/buildsvc/logs`，大量编译输出会持续写固态硬盘。
- 成功标准：server run 日志默认写入 `/tmp/buildsvc/logs`；可通过 `server.log_dir` 显式覆盖；数据库、源码包和升级包仍保存在原有数据目录。

## 2. 背景与边界

- 背景：当前 `/var/lib/buildsvc/logs` 下已有编译日志，用户已将 `/tmp` 挂载为内存，希望后续日志落到 tmpfs。
- 包含：配置解析、Storage 日志目录注入、示例配置、文档说明、回归测试。
- 不包含：迁移或删除旧日志；修改前端 5MiB 展示限制；修改 agent 日志传输队列。
- 关键假设：用户接受 `/tmp` 下日志重启后可能丢失；run 记录仍保留在数据库中。
- 非目标：实现日志轮转、压缩或远端日志归档。
- 最大修改范围：`src/config.rs`、`src/storage.rs`、`src/server/mod.rs`、配置示例、README/总文档、本地任务索引。
- 禁止触碰范围：不删除 `/var/lib/buildsvc/logs` 旧日志；不修改 agent 执行逻辑和 WebSocket 协议。

## 3. 风险门禁

| 项 | 结论 |
|----|------|
| 风险矩阵 | L2：单模块配置和存储路径调整，影响日志持久性 |
| 高风险开发门禁 | 是：配置格式和长期运行行为变化，最低 L2 |
| 破坏性操作 | 否：不删除旧日志 |
| 用户已有修改 | 否：编码前 `git status --short` 无输出 |
| 底层/系统风险 | 否：不涉及 unsafe、并发、协议或 ABI |
| 命令权限 | C1：工作区内代码/文档修改和本地测试 |
| 用户确认事项 | 已确认按方案修改 |
| 回滚/止损方式 | 恢复默认日志目录或在配置中设置 `log_dir = /var/lib/buildsvc/logs` |

## 4. 方案

- 推荐方案：新增 `server.log_dir` 配置，默认 `/tmp/buildsvc/logs`；`Storage` 接收显式日志目录，sources/upgrades/tmp 继续来自 `core.data_dir`。
- 取舍理由：满足减少 SSD 写入，同时保留配置覆盖能力，避免把数据库和源码包放入 tmpfs。
- 风险与应对：tmpfs 内容可能随重启丢失；通过 README/总文档明确 run 日志为 tmpfs 默认保存。

## 5. 执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 增加日志目录默认值和覆盖配置测试 | `cargo test storage::tests::open_uses_tmp_log_dir_by_default` 先失败 | 完成 |
| 2 | 实现 `server.log_dir` 并让 Storage 使用该目录 | 相关单测通过 | 完成 |
| 3 | 更新配置示例和长期文档 | 人工审查、`git diff --check` | 完成 |
| 4 | 运行格式和测试验证 | `cargo fmt --check`、`cargo test`、`git diff --check` | 完成 |

## 6. 实现记录

- 修改文件：`src/config.rs`、`src/storage.rs`、`src/server/mod.rs`、`configs/buildsvc.ini`、`configs/server.test.ini`、`README.md`、`docs/overview-product.md`、`docs/overview-product-dev.md`、`docs/dev/README.md`。
- 关键决策：`Storage::open` 默认使用 `/tmp/buildsvc/logs`；新增 `Storage::open_with_log_dir` 供 server 注入 `server.log_dir`，测试使用独立临时日志目录避免共享 `/tmp`。
- 计划偏差：无。
- 安全门禁执行结果：未执行破坏性操作；未删除旧日志；修改范围保持在 run 日志路径配置和文档。

## 7. 验证记录

- 验证环境：本地 Linux 工作区
- 系统信息（OS/内核/架构/编译器/运行时，按需）：Linux 7.1.4-gentoo-dingjing x86_64 GNU/Linux；rustc 1.96.1；cargo 1.96.1。

| 验证项 | 命令/步骤 | 结果 | 备注 |
|--------|-----------|------|------|
| 默认日志目录回归 | `cargo test storage::tests::open_uses_tmp_log_dir_by_default` | 通过 | 先失败，显示旧实现路径为临时 `data_dir/logs`；实现后通过 |
| 配置解析回归 | `cargo test config::tests` | 通过 | 覆盖默认 `log_dir` 和显式覆盖 |
| 全量 Rust 测试 | `cargo test` | 通过 | 31 项通过 |
| 格式检查 | `cargo fmt --check` | 通过 |  |
| 空白检查 | `git diff --check` | 通过 |  |

- 未执行验证项：未启动真实 server/agent 做端到端构建；本次通过配置解析和 Storage 路径单测覆盖核心路径。
- 残余风险：`/tmp` 为 tmpfs 时，重启或 tmp 清理会导致 run 日志文件丢失；数据库中的 run 记录仍会保留。

## 8. 总结

- 最终结果：server run 日志默认写入 `/tmp/buildsvc/logs`，并可通过 `[server].log_dir` 覆盖；源码包、升级包和 SQLite 仍使用 server data dir。
- 遗留风险：旧 `/var/lib/buildsvc/logs` 文件未迁移也未删除；如需释放磁盘空间需用户单独确认后处理。
- 后续建议：如后续仍担心 tmpfs 占用内存，可再增加日志大小上限或 agent 侧日志发送背压。
