# Run log 过量导致 Web 卡顿修复记录

> 文档元数据
> - 文件编号：22
> - 文档类型：fix
> - 文件路径：docs/dev/22-fix-run-log-tail.md
> - 完成日期：2026-08-20
> - 需求级别：L2

## 1. 问题定义

- 问题现象：编译过程中 Web 页面疑似崩溃或卡死，buildsvc 进程本身未见明确崩溃证据。
- 预期行为：Run log 页面只展示有限日志内容，长时间构建或大量输出时浏览器仍可用。
- 影响范围：Web UI 的 Run Log 初次加载和实时追加显示；server 日志文件仍完整保留。
- 不包含：不修改构建脚本、不改变 run 执行/取消/超时策略、不处理 Upgrade log 或 Terminal。

## 2. 证据与根因

- 复现方式：现有运行数据中存在 4.6MB / 84707 行 Run log；当前代码选择 run 时全量读取并显示，实时日志持续追加到同一个 `<pre>`。
- 证据等级：E2
- 关键日志/堆栈/输入：
  - `/var/lib/buildsvc/logs/run_676941e4e6e04dfbbdc4fd46a73ba825.log` 为 4617979 字节、84707 行。
  - `src/server/ui.rs` 使用 `fetch(.../log)` 后 `logEl.textContent = await response.text()`。
  - WebSocket log 消息使用 `logEl.textContent += message.data`，没有上限。
  - `src/storage.rs` 的 `read_log` 对日志文件执行整文件读取。
- 根因：Run log 历史读取和实时追加都没有可见内容上限，大日志会导致浏览器内存、字符串复制和 `<pre>` 重排压力持续增长。
- 相关代码路径：`src/storage.rs`、`src/server/ui.rs`、`src/server/mod.rs`。

## 3. 修复方案

- 最小修复点：Run log 历史读取只返回文件尾部最多 5MiB；前端实时追加后把可见 Run log 限制到最多 5MiB。
- 代码逻辑改动：
  - `Storage::read_log` 改为 seek 到日志文件尾部窗口再读取。
  - Web UI 增加 Run log 追加函数，加载和 WebSocket 追加路径统一走 5MiB 上限。
- 影响的使用场景：大日志 run 的选择、刷新、实时显示。
- 不影响的使用场景：server 侧日志文件完整性、run 状态、构建执行和超时策略。

## 4. 修复执行计划

| 步骤 | 修改内容 | 验证方式 | 状态 |
|------|----------|----------|------|
| 1 | 增加超过 5MiB 日志只返回尾部的回归测试 | `cargo test storage::tests::read_log_returns_tail_when_file_exceeds_display_limit` 先失败 | 完成 |
| 2 | 修改后端 `read_log` 为尾部读取 | 上述单测通过 | 完成 |
| 3 | 修改前端 Run log 实时追加截断 | 代码审查、`cargo test`、`git diff --check` | 完成 |
| 4 | 分析 7200 秒超时归因 | 数据库 run 记录、日志末尾和配置比对 | 完成 |

## 5. 风险摘要

| 项 | 结论 |
|----|------|
| 风险矩阵 | L2 |
| 命令权限 | C1（工作区内代码、测试和本地文档修改） |
| 高风险门禁 | 否 |
| 破坏性操作 | 否 |
| 用户已有修改 | 否 |
| 用户确认 | 用户已要求先修复页面全量拉取日志问题 |
| 副作用/风险 | 页面只展示尾部日志，完整日志仍在 server 文件中；如果旧日志中间部分需要查看，当前 UI 不再展示。 |

## 6. 验证

- 验证环境：本地 Linux 工作区；运行数据来自 `/var/lib/buildsvc`。
- 回归/相关验证：
  - `cargo test storage::tests::read_log_returns_tail_when_file_exceeds_display_limit`：先失败，显示旧实现返回 5MiB+32 字节完整日志；修复后通过。
  - `cargo fmt --check`：通过。
  - `cargo test`：28 项通过。
  - `git diff --check`：通过。
- 结果：后端历史 Run log 读取限制为文件尾部最多 5MiB；前端实时追加后可见 Run log 限制为最多 5MiB。
- 未执行验证项：未启动浏览器做真实页面内存曲线验证。
- 残余风险：前端按 JavaScript 字符长度截断，非 ASCII 日志的字节数可能与 5MiB 不完全一致；完整日志仍需通过 server 文件系统查看。

## 7. 修复总结

- 最终结果：修复 Run log 全量拉取和无限追加导致 Web 页面被大日志拖垮的问题；总产品/开发文档已记录“Web/API 默认只显示尾部最多 5MiB，日志文件完整保留”。
- 计划偏差：按长期行为变更要求追加更新 `docs/overview-product.md` 和 `docs/overview-product-dev.md`。
- 超时归因：`run_676941e4e6e04dfbbdc4fd46a73ba825` 运行 7201 秒，配置 `script_timeout_sec=7200`，退出码 124；日志末尾 `Terminated/Error 143` 与 buildsvc 超时后终止进程组一致。safe_fs x86 脚本没有自身两小时超时逻辑，而是在第 5 个 x86 rootfs `/data/ubuntu20.04-6` 构建期间尚未完成，触发 buildsvc 超时策略。
- 多角色审视：L2 裁剪项通过；修改范围限定在 Run log 读取/显示、本地任务文档和总文档；未引入依赖、配置或协议变更；完整日志文件未截断。
- 未提交说明：用户未要求提交，本次未执行 `git add`/`git commit`。
- 后续建议：如果 x86 全量 rootfs 构建本身需要超过 2 小时，应提高 `[server].script_timeout_sec` 或拆分 x86 rootfs 构建批次；这属于运行策略/构建脚本调整，不在本次代码修改范围内。
