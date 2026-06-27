# Agent 脚本权限处理任务记录

> 日期：2026-06-27
> 级别：L2
> 状态：已完成

## 背景

zip 等源码包格式可能丢失 Unix 执行位。用户的 `run-build.sh` 内部调用 `scripts/build-all.sh` 或 `scripts/scp.sh` 时，入口脚本已被 agent 加权限，但子脚本仍可能因为权限不足失败。

## 实现

- Unix agent 执行前仍要求源码根目录存在 `run-build.sh`。
- 执行前递归扫描源码根目录内普通文件。
- 自动给 `run-build.sh`、`*.sh` 和带 shebang 的脚本文件增加执行位。
- 其他普通文件不修改权限。
- 继续关闭脚本 stdin，避免 `ssh`、`scp` 等命令等待交互输入导致 run 卡住。

## 验证

- `cargo fmt --check`
- `cargo test`
