# Build 源码包删除任务记录

> 日期：2026-06-26
> 级别：L2
> 状态：已完成

## 背景

用户需要 Builds 列表也能删除选中的源码包。Build 删除不应触碰 agent 工作区，避免和 Runs 删除语义混在一起。

## 方案

- Builds 表增加复选框多选和表头全选。
- Builds 增加 `Delete Source` 按钮；键盘 `Delete` 会删除当前选中的 Builds 或 Runs。
- server 新增 `DELETE /api/builds/{id}`。
- 删除 build 前要求该 build 下没有 run 记录；仍有关联 run 时拒绝删除，用户需先删除对应 Runs。
- 删除 build 时，server 删除 `<server_data_dir>/sources/<build_id>` 和 `builds` 表记录。
- build id 做路径安全校验，拒绝空值和路径分隔符。

## 验证

- `cargo fmt --check`
- `cargo check`
- `make test`
- `make debug`
- `make`
- 本地 smoke：临时启动 debug server/agent，上传最小源码包；确认 build 在仍有关联 run 时删除失败；删除 run 后再删除 build，确认 `<server_data_dir>/sources/<build_id>` 消失且 server state 不再返回该 build。

## 结果

Builds 已支持选中删除源码包。删除范围限定在 server 源码包目录和 build 记录，不删除 agent 工作区。
