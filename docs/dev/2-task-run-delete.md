# Run 删除功能任务记录

> 日期：2026-06-26
> 级别：L3
> 状态：已完成

## 背景

用户需要在 Web UI 中删除选中的 Runs，支持 Delete 键和全选删除。删除语义不是单纯隐藏界面记录，而是必须先由对应在线 agent 删除本地工作区目录，例如 `<agent_work_dir>/runs/<run_id>/`，成功后 server 才能删除 run 记录。

## 方案

- Runs UI 保持在左侧边栏，表格高度调整为 10 条 run 数据。
- Runs 增加复选框多选和表头全选；Delete 键和 Delete 按钮触发删除。
- server 新增 `DELETE /api/runs/{id}`。
- server 只允许删除 `success`、`failed`、`canceled`、`lost` 状态的 run。
- server 要求对应 agent 当前在线，发送 `run_delete`，等待 `run_deleted` 确认。
- agent 删除 `<agent_work_dir>/runs/<run_id>`；目录不存在视为成功，run 正在执行时拒绝删除。
- agent 确认成功后，server 删除 SQLite run 记录和 server 侧 run 日志。

## 验证

- `cargo fmt --check`
- `cargo check`
- `make test`
- `make debug`
- `make`
- 本地 smoke：临时启动 debug server/agent，上传最小 `tar.gz`，等待 run success 后调用 `DELETE /api/runs/{id}`，确认 agent 工作区目录消失且 server state 不再返回该 run。

## 结果

Run 删除链路已打通。agent 离线、run 未结束、agent 工作区删除失败或确认超时时，server 不删除界面记录。
