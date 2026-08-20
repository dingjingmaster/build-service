# 取消构建脚本超时限制总结

> 文档元数据
> - 文件编号：23
> - 文档类型：summary
> - 文件路径：docs/dev/23-summary-unlimited-script-runtime.md
> - 完成日期：2026-08-20
> - 需求级别：L3

## 结果

- 移除了 `[server]` 和 `[agent]` 的 `script_timeout_sec` 配置解析字段。
- 移除了 `ServerToAgent::RunStart` 协议中的 `script_timeout_sec` 字段。
- agent 执行 `run-build.sh` / `run-build.bat` 时不再创建脚本超时 sleep 分支，只等待脚本自然退出或 cancel 信号。
- server 创建 run 和调度 run 时不再携带脚本超时值。
- storage 的 `RunRow` / `NewRun` 不再暴露 timeout 业务字段；SQLite `script_timeout_sec` 旧列保留为兼容遗留列，新写入值固定为 0。
- 配置样例、README、架构文档和总产品/开发文档已移除脚本超时配置说明。

## 验证

- TDD 红灯：`cargo test agent::tests::run_script_waits_for_process_without_timeout` 在旧实现下失败，脚本被 1ms timeout 终止并返回 124。
- TDD 绿灯：移除超时分支后，`cargo test agent::tests::run_script_waits_for_process_without_timeout` 通过。
- 完整验证：
  - `cargo fmt --check`：通过。
  - `cargo test`：29 项通过。
  - `git diff --check`：通过。
- 文档/配置检查：用户可见 README、配置样例和架构文档不再出现 `script_timeout_sec`；该字符串仅保留在 storage 旧 SQLite 兼容列和任务历史文档中。

## 风险与后续

- server/agent WebSocket 协议字段变化，部署时应同步升级 server 和 agent。
- 长时间卡住的脚本不会再被 buildsvc 按时间自动杀掉，会持续占用 agent 并发槽；止损方式是 Web UI Cancel、agent 断线取消或脚本自身退出。
- 未执行真实长时间构建实测；本地自动化覆盖了“不因短 timeout 结束”的 agent 执行行为。
- 用户未要求提交，本次未执行 `git add`/`git commit`。
