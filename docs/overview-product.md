# buildsvc 产品概览

> 文档元数据
> - 文档版本：v1.0.0
> - 最后更新：2026-06-26
> - 更新来源：docs/dev/1-*.md

## 1. 产品定位

- 目标用户：在可信局域网内自用的构建维护者。
- 核心问题：用一台主机器把源码包分发到多台不同平台机器上，执行固定脚本，并集中查看执行过程、日志和结果。
- 核心价值：用一个轻量 Rust 二进制提供 server/agent 两种角色，不依赖 Jenkins、Gitea、K8s 等重型平台。
- 非目标：通用 CI 平台、复杂 workflow 编排、构建产物管理、权限多租户、互联网暴露服务。

## 2. 功能边界

- 核心功能：
  - 启动时读取 INI 配置，根据 `role` 进入 server 或 agent。
  - 支持通过 `-c` / `--config` 指定配置文件；不指定时自动发现默认配置。
  - server 提供 Web UI，上传 `tar.gz`/`zip` 源码包。
  - 用户按 agent 名选择目标 agent。
  - agent 通过 WebSocket 连接 server，启动时自动上报最新计算机名，接收 run，下载源码包，解包并执行固定脚本。
  - server 实时显示 agent 状态、run 状态、日志和退出码。
  - server 支持删除 offline agent。
  - failed/lost/canceled/success run 可手动 rerun。
  - 已结束 run 可从 Web UI 删除；删除前必须由在线 agent 成功删除对应工作区目录。
  - 无关联 run 的 source archive/build 可从 Web UI 删除。
  - Web UI 可从在线 Agents 打开交互式终端；终端通过 agent WebSocket/PTTY 转发，不依赖 SSH。
- 不支持功能：
  - 第一版不支持断点续传。
  - 第一版不管理 artifacts。
  - 第一版 Web UI 不登录。
  - 第一版不支持自定义 workflow 文件。
- 关键对象：agent、build、run、source archive、log。
- 关键状态：`queued`、`assigned`、`preparing`、`running`、`success`、`failed`、`canceled`、`lost`。

## 3. 关键场景

| 场景 | 用户目标 | 成功标准 | 异常/边界 |
|------|----------|----------|-----------|
| 提交构建 | 上传源码包并选择目标 agent | 每个选中 agent 创建一个 run | 未选择目标、包格式不支持、包为空时拒绝 |
| 执行构建 | agent 运行源码根目录固定脚本 | exit code 0 为 success，非 0 为 failed | 脚本缺失、解包失败或超时为 failed |
| 查看状态 | server 实时展示 agent 和 run | Web UI 通过 WebSocket 更新；Agents 在线优先，再按计算机名排序 | agent 断线后显示 offline，运行中 run 标记 lost |
| 重跑任务 | 手动重跑已结束 run | 新建 queued run 并重新调度 | running run 不允许 rerun |
| 删除 Run | 清理已结束 run 的 agent 工作区和界面记录 | agent 在线并成功删除 `<agent_work_dir>/runs/<run_id>` 后，server 删除 run 记录和日志 | agent 离线、run 未结束或工作区删除失败时拒绝删除 |
| 删除源码包 | 清理 server 上已上传源码包 | build 下没有 run 记录时，删除 `<server_data_dir>/sources/<build_id>` 和 build 记录 | 仍有关联 run 时拒绝删除，需先删除对应 Runs |
| 打开 Agent 终端 | 从 Web UI 直接操作 agent 所在机器 | server/agent 均启用终端，agent 在线，PTY 输出实时显示，键盘输入实时转发 | 未启用、agent 离线或会话数达到限制时拒绝 |

## 4. 核心流程

```text
1. server 启动，读取 INI，初始化 SQLite 和数据目录。
2. agent 启动，读取 INI，通过 WebSocket hello/heartbeat 注册到 server。
3. 用户在 Web UI 上传源码包，并选择目标 agent。
4. server 创建 build 和 runs，并向在线且有容量的 agent 下发 run_start。
5. agent 下载源码包，解包顶层唯一目录，执行 run-build.sh 或 run-build.bat。
6. agent 通过 WebSocket 回传日志、状态和最终退出码。
7. server 更新 Web UI，并允许用户对结束 run 手动 rerun 或 delete。
8. 用户可从 Agents 列表打开终端；server 只转发浏览器输入输出，agent 在本机打开 shell/PTTY。
```

## 5. 产品规则

- 权限规则：
  - 每个 agent 自动生成并持久化独立 token。
  - Web UI 第一版不登录，仅适合可信局域网。
- 状态流转：
  - 正常：`queued -> assigned -> preparing -> running -> success/failed`。
  - 取消：`queued/assigned/preparing/running -> canceled`。
  - 断线：运行中 run 标记 `lost`。
- 异常处理：
  - server 启动时将历史 active run 标记 `lost`。
  - agent 按 server 下发的心跳间隔上报 heartbeat。
  - 默认心跳间隔为 5 秒，server 默认 15 秒未收到心跳则将 agent 显示为 `offline`。
  - agent 断线后 server 标记该 agent 的 active run 为 `lost`。
  - agent 连接断开后会取消本地运行任务。
- 兼容约束：
  - 支持 Linux、Windows、macOS。
  - 源码包必须是 `tar.gz`、`tgz` 或 `zip`。
  - 压缩包顶层必须只有一个目录。
- 用户可见行为：
  - 日志实时追加展示。
  - agent 选择只基于 agent 名。
  - Agents 列表显示 agent 名、最新计算机名、状态和容量。
  - Agents 列表排序为在线 agent 优先，离线 agent 靠后，同组内按计算机名排序。
  - offline agent 可从 server 运行时列表删除；删除后该 agent 不再显示，下一次连接会重新登记 token 并出现在列表中。
  - Runs 列表支持多选、全选和 Delete 键删除；删除仅对已结束 run 生效。
  - 删除 run 时，server 必须先请求对应在线 agent 删除本地工作区；agent 确认成功后，server 才删除界面记录和 server 侧日志。
  - Builds 列表支持多选、全选和 Delete 键删除；删除仅清理 server 侧源码包和 build 记录，不触碰 agent 工作区。
  - Agents 列表对在线且启用终端的 agent 显示 `Open` 入口，打开后进入 Web 终端面板。
  - 终端能力默认配置关闭；server 和 agent 侧都启用时才允许打开。

## 6. 非功能要求

- 性能：源码上传流式写入磁盘，不整体读入内存。
- 可用性：agent 主动连接 server，适配局域网和 NAT 场景。
- 安全：无构建隔离；脚本和 Web 终端都以 agent 进程权限运行，只适合可信源码、可信网络和低权限 agent 运行用户。
- 兼容性：跨平台脚本名固定，平台差异由脚本和 agent 执行器处理。
- 可观测性：server 记录 run 状态和日志文件；自身日志使用 tracing。

## 7. 文档索引

- 需求与任务索引：docs/dev/README.md
- 开发概览：docs/overview-product-dev.md
- 架构笔记：docs/architecture.md
- 关键任务文档：
  - docs/dev/1-research-buildsvc-mvp.md
  - docs/dev/1-plan-buildsvc-mvp.md

## 8. 变更记录

| 日期 | 变更 | 影响 | 关联文档 |
|------|------|------|----------|
| 2026-06-26 | 创建 MVP 产品边界 | 明确 server/agent/Web UI/固定脚本执行模型 | docs/dev/1-*.md |
