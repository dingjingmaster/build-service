# 源码包根目录定位放宽任务记录

> 日期：2026-06-26
> 级别：L2
> 状态：已完成

## 背景

用户认为源码包必须包含唯一顶层目录不方便，希望压缩包根目录直接包含 `run-build.sh` 或 `run-build.bat` 时也能执行。

## 实现

- archive 解包后优先在解压目录本身查找 `run-build.sh` 或 `run-build.bat`。
- 如果根目录没有脚本，且压缩包只有一个顶层目录，则进入该目录查找脚本。
- 多个顶层目录但根目录无脚本时仍报错，避免无法确定源码根目录。
- 继续保留 archive entry 的相对路径和 unsafe component 校验。

## 验证

- `cargo fmt --check`
- `cargo test`
