# Run Log 前端缓冲与节流任务记录

> 文档元数据
> - 文件编号：25
> - 文档类型：task
> - 文件路径：docs/dev/25-task-run-log-ui-buffer.md
> - 文档版本：v1.0.0
> - 最后更新：2026-09-02
> - 需求级别：L2
> - 关联需求：将 Web Run Log 显示改为累积 buffer + 分片裁剪/节流刷新，并把显示上限从 5MiB 降到 2MiB。

## 1. 目标

- 要解决的问题：Run Log 实时追加时直接拼接 `logEl.textContent + value`，在高频大日志下会产生重复 DOM 读取、字符串复制和渲染压力。
- 成功标准：前端维护独立 run log buffer，追加时对 buffer 分片裁剪；DOM 更新通过 animation frame 节流；可见日志上限为 2MiB。

## 2. 背景与边界

- 背景：server 侧日志已改到 `/tmp/buildsvc/logs`，浏览器仍需降低长日志实时显示时的内存和 CPU 抖动。
- 包含：Run Log 前端 JS、server Run Log API 尾部读取窗口、相关文档和本地任务索引。
- 不包含：agent 日志发送背压、WebSocket 协议。
- 关键假设：用户接受 Web/API 可见日志窗口从 5MiB 缩小到 2MiB，server 文件仍完整保存。
- 非目标：增加虚拟滚动、分页、搜索或下载完整日志。
- 最大修改范围：`src/server/ui.rs`、`src/storage.rs`、README/总文档、本地任务文档和索引。
- 禁止触碰范围：不修改 server/agent 协议、不删除日志、不修改上一轮 tmpfs 路径逻辑。

## 3. 风险门禁

| 项 | 结论 |
|----|------|
| 风险矩阵 | L2：前端行为和长期显示上限变化 |
| 高风险开发门禁 | 否 |
| 破坏性操作 | 否 |
| 用户已有修改 | 是：上一轮日志目录改动和无关版本号改动保留，不回退 |
| 底层/系统风险 | 否 |
| 命令权限 | C1：工作区内代码/文档修改和本地测试 |
| 用户确认事项 | 已明确要求按指定方案修改 |
| 回滚/止损方式 | 恢复旧的 5MiB 上限和直接刷新逻辑 |

## 4. 方案

- 推荐方案：新增 `runLogBuffer` 保存可见日志窗口；历史加载和实时追加统一写入 buffer；超过 2MiB 时先保留最近 2MiB 分片；DOM 写入通过 `requestAnimationFrame` 合并刷新。
- 取舍理由：改动集中，不引入依赖；减少 DOM 读取和高频渲染；保持现有 UI 和 WebSocket 数据流。
- 风险与应对：刷新节流可能让日志显示延迟到下一帧；这符合实时 UI 的可接受延迟。

## 5. 执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 增加 UI HTML 行为约束测试 | `cargo test server::ui::tests::run_log_ui_uses_buffered_throttled_two_mib_window` 先失败 | 完成 |
| 2 | 修改 Run Log 前端追加/刷新逻辑 | 上述测试通过 | 完成 |
| 3 | 将 server Run Log API 尾部读取窗口同步降到 2MiB | `cargo test storage::tests::read_log_returns_tail_when_file_exceeds_display_limit` 先失败再通过 | 完成 |
| 4 | 更新文档中的 5MiB 描述为 2MiB | 人工审查、`rg` 检查 | 完成 |
| 5 | 运行格式和测试验证 | `cargo fmt --check`、`cargo test`、`git diff --check` | 完成 |

## 6. 实现记录

- 修改文件：`src/server/ui.rs`、`src/storage.rs`、`docs/overview-product.md`、`docs/overview-product-dev.md`、`docs/dev/README.md`。
- 关键决策：前端不再从 `logEl.textContent` 读取旧内容进行拼接，而是维护 `runLogBuffer`；每次追加后裁剪到 2MiB；DOM 写入通过 `requestAnimationFrame` 合并。server API 读取窗口同步降到 2MiB，避免初次加载时浏览器先接收 5MiB 再裁剪。
- 计划偏差：原计划只写前端；为降低历史加载瞬时内存和网络传输，同步调整 `Storage::read_log` 尾部读取窗口。
- 安全门禁执行结果：未执行破坏性操作；未修改协议；未删除日志；保留用户已有改动。

## 7. 验证记录

- 验证环境：本地 Linux 工作区
- 系统信息（OS/内核/架构/编译器/运行时，按需）：Linux 7.1.4-gentoo-dingjing x86_64 GNU/Linux；rustc 1.96.1；cargo 1.96.1。

| 验证项 | 命令/步骤 | 结果 | 备注 |
|--------|-----------|------|------|
| UI 行为约束测试 | `cargo test server::ui::tests::run_log_ui_uses_buffered_throttled_two_mib_window` | 通过 | 先失败，显示旧 UI 仍为 5MiB 且缺少 buffer/节流；实现后通过 |
| API 尾部窗口测试 | `cargo test storage::tests::read_log_returns_tail_when_file_exceeds_display_limit` | 通过 | 先失败，显示旧常量仍返回 5MiB；实现后通过 |
| 全量 Rust 测试 | `cargo test` | 通过 | 32 项通过 |
| 格式检查 | `cargo fmt --check` | 通过 |  |
| 空白检查 | `git diff --check` | 通过 |  |

- 未执行验证项：未启动浏览器做真实页面内存曲线验证。
- 残余风险：未做浏览器真实性能曲线采集；代码级约束已覆盖 buffer、2MiB 上限和节流刷新。

## 8. 总结

- 最终结果：Web Run Log 前端改为 `runLogBuffer` 累积和裁剪，DOM 刷新通过 `requestAnimationFrame` 节流；Web/API 可见日志窗口降为 2MiB。
- 遗留风险：极端高频日志仍会带来 WebSocket 消息解析成本；agent 侧发送队列背压不在本次范围。
- 后续建议：如仍出现 agent 或 server 侧内存增长，再处理 agent 到 server 的日志发送背压。
