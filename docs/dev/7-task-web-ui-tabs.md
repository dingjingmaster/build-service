# Web UI Tab 分区任务记录

> 日期：2026-06-26
> 级别：L2
> 状态：已完成

## 背景

升级功能加入后，原页面左侧同时显示提交构建、升级和 Runs，日常编译主流程被升级功能挤占空间。用户希望把整个页面做成 tab 页：首页显示编译相关主要功能，另一个 tab 显示升级相关功能。

## 实现

- Web UI header 新增 `Builds` 和 `Upgrades` tab。
- `Builds` tab 作为默认首页，保留：
  - Submit Build
  - Runs
  - Builds
  - Agents
  - Run Log
- `Upgrades` tab 显示：
  - 升级包类型选择
  - 升级包上传
  - 可升级 agent 选择
  - Upgrade Log
- Delete 快捷键仅在 Builds tab 生效，避免在升级页误删隐藏选择项。
- Builds tab 中 Runs、Builds、Agents 表格不再按固定条数限制高度；页面整体滚动。Builds 和 Agents 处于同一网格行，section 高度自然对齐。

## 验证

- JavaScript 语法检查
- `cargo fmt --check`
- `cargo test`
