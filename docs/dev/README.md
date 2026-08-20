# 需求与任务索引

> 记录 L1+ 新增任务、问题、调研、计划、总结或评审文档。本索引是本地上下文，默认不要求提交。新需求/问题优先读取本索引，再按相关性最多展开 3-5 个 Summary、task 或 fix 文档，避免加载完整历史上下文。

## 编号规则

- 文件编号使用从 `1` 开始递增的正整数，不要求固定位数。
- 编号全局只在 `docs/dev/` 下递增，不按类型分别编号。
- 新增任务、问题修复、调研、计划、总结、评审或需求变更文档前，先检查本索引和 `docs/dev/` 现有文件名，取最大编号 + 1。
- 同一需求的多份文档使用同一编号，例如 `2-research-xxx.md`、`2-plan-xxx.md`、`2-summary-xxx.md`。
- 编号一旦分配不得复用；取消、废弃、拆分、合并也要在索引中保留记录并标注状态。
- 文件命名格式：`N-[type]-[slug].md`，其中 `type` 可取 `summary`、`task`、`fix`、`research`、`plan`、`review`。

## 索引

| ID | 日期 | 级别 | 类型 | 文档 | 状态 | 摘要 |
|----|------|------|------|------|------|------|
| 1 | 2026-06-26 | L3 | research | [1-research-buildsvc-mvp.md](1-research-buildsvc-mvp.md) | 已完成 | Rust 自研轻量构建分发系统 MVP 调研。 |
| 1 | 2026-06-26 | L3 | plan | [1-plan-buildsvc-mvp.md](1-plan-buildsvc-mvp.md) | 已完成 | Rust 自研轻量构建分发系统 MVP 实施计划。 |
| 1 | 2026-06-26 | L3 | summary | [1-summary-buildsvc-mvp.md](1-summary-buildsvc-mvp.md) | 已完成 | Rust 自研轻量构建分发系统 MVP 实现、验证和残余风险总结。 |
| 2 | 2026-06-26 | L3 | task | [2-task-run-delete.md](2-task-run-delete.md) | 已完成 | Runs 多选删除、agent 工作区清理确认和 server 记录删除。 |
| 3 | 2026-06-26 | L2 | task | [3-task-build-delete.md](3-task-build-delete.md) | 已完成 | Builds 多选删除 server 源码包和 build 记录。 |
| 4 | 2026-06-26 | L3 | task | [4-task-agent-terminal.md](4-task-agent-terminal.md) | 已完成 | Agent Web 终端，基于 PTY 和 WebSocket 转发输入输出。 |
| 5 | 2026-06-26 | L3 | task | [5-task-linux-packaging.md](5-task-linux-packaging.md) | 已完成 | deb、rpm、Gentoo emerge overlay 打包入口。 |
| 6 | 2026-06-26 | L3 | task | [6-task-agent-package-upgrade.md](6-task-agent-package-upgrade.md) | 已完成 | Server 推送 deb/rpm/Gentoo overlay 包，agent 校验并通过系统包管理器升级。 |
| 7 | 2026-06-26 | L2 | task | [7-task-web-ui-tabs.md](7-task-web-ui-tabs.md) | 已完成 | Web UI 拆分 Builds 和 Upgrades 两个 tab，编译和升级功能分区显示。 |
| 8 | 2026-06-26 | L2 | task | [8-task-package-systemd-hooks.md](8-task-package-systemd-hooks.md) | 已完成 | deb/rpm/Gentoo 包安装、升级和卸载时自动管理 buildsvc systemd service。 |
| 9 | 2026-06-26 | L2 | task | [9-task-agent-token-autogen.md](9-task-agent-token-autogen.md) | 已完成 | 移除 agent labels 配置和手工 token 配置，agent 首次启动自动生成 token。 |
| 10 | 2026-06-26 | L3 | fix | [10-fix-buildsvc-self-restart.md](10-fix-buildsvc-self-restart.md) | 已完成 | 修复从 buildsvc Web 终端触发包配置时同步重启自身 service 导致命令挂起的问题。 |
| 11 | 2026-06-26 | L2 | task | [11-task-agent-run-workspace-cleanup.md](11-task-agent-run-workspace-cleanup.md) | 已完成 | Agent 成功 run 自动删除当前工作区，启动时清理历史 run 工作区，失败保留现场。 |
| 12 | 2026-06-26 | L2 | task | [12-task-flexible-archive-root.md](12-task-flexible-archive-root.md) | 已完成 | 源码包支持脚本直接位于压缩包根目录，兼容唯一顶层目录布局。 |
| 13 | 2026-06-27 | L2 | task | [13-task-agent-script-permissions.md](13-task-agent-script-permissions.md) | 已完成 | Agent 执行前递归给 shell/shebang 脚本增加执行权限，并明确 ssh/scp 需非交互配置。 |
| 14 | 2026-06-29 | L3 | research | [14-research-agent-machine-shutdown.md](14-research-agent-machine-shutdown.md) | 已完成 | Agents 表新增关闭机器入口的范围、风险和方案调研。 |
| 14 | 2026-06-29 | L3 | plan | [14-plan-agent-machine-shutdown.md](14-plan-agent-machine-shutdown.md) | 已完成 | 通过 REST API 和 server-agent WebSocket 指令触发远程机器关机的实施计划。 |
| 14 | 2026-06-29 | L3 | summary | [14-summary-agent-machine-shutdown.md](14-summary-agent-machine-shutdown.md) | 已完成 | Agents 表新增关闭机器列，在线 agent 可点击触发本机关机命令，表格隐藏横向溢出。 |
| 15 | 2026-07-09 | L2 | fix | [15-fix-buildsvc-startup-deadlock.md](15-fix-buildsvc-startup-deadlock.md) | 已完成 | 修复 server 启动时 active run 标记 lost 持锁重入导致无端口监听的问题。 |
| 16 | 2026-08-01 | L2 | fix | [16-fix-buildsvc-child-library-path.md](16-fix-buildsvc-child-library-path.md) | 已完成（待部署） | 修复随包动态库通过 LD_LIBRARY_PATH 污染 bash、systemctl 和构建脚本导致段错误的问题。 |
| 17 | 2026-08-03 | L3 | research/plan/summary | [17-research-agent-shutdown-permission.md](17-research-agent-shutdown-permission.md)、[17-plan-agent-shutdown-permission.md](17-plan-agent-shutdown-permission.md)、[17-summary-agent-shutdown-permission.md](17-summary-agent-shutdown-permission.md) | 已完成 | 为 agent 增加远程关机权限配置，并在 UI、server API 和 agent 本地双重防护。 |
| 18 | 2026-08-04 | L1 | summary | [18-summary-agent-shutdown-disabled-ui.md](18-summary-agent-shutdown-disabled-ui.md) | 已完成 | 禁止远程关机的 Agent 在 Agents 列表置顶、淡金色标识且关闭按钮禁用。 |
| 19 | 2026-08-04 | L2 | fix | [19-fix-gentoo-loader-permission.md](19-fix-gentoo-loader-permission.md) | 已完成 | 修复 Gentoo 包动态加载器权限丢失导致 buildsvc 服务启动重启循环。 |
| 20 | 2026-08-04 | L2 | task | [20-task-agent-shutdown-default-enabled.md](20-task-agent-shutdown-default-enabled.md) | 已完成 | Agent 远程关机默认允许，显式配置 false 时禁止。 |
| 21 | 2026-08-05 | L1 | summary | [21-summary-builds-agents-table-height.md](21-summary-builds-agents-table-height.md) | 已完成 | Builds 与 Agents 表格统一为 8 条数据行高度，超出后在表格内滚动。 |
| 22 | 2026-08-20 | L2 | fix | [22-fix-run-log-tail.md](22-fix-run-log-tail.md) | 已完成 | Run log 历史读取和实时显示限制为最多 5MiB，避免大日志拖垮 Web 页面。 |
