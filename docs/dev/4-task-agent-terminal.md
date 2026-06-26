# Agent Web 终端任务记录

> 日期：2026-06-26
> 级别：L3
> 状态：已完成

## 背景

用户希望从 Web UI 的 Agents 列表直接进入 agent 所在机器执行命令，不依赖 SSH，要求实现交互式终端模式。

## 方案

- 新增 `portable-pty` 依赖，由 agent 在本机创建 PTY。
- server 新增 `/api/agents/{name}/terminal/ws`，浏览器通过该 WebSocket 打开终端。
- server 不执行命令，只在 browser WebSocket 和 agent WebSocket 之间转发输入、输出、resize、close。
- agent 支持 `terminal_start`、`terminal_input`、`terminal_resize`、`terminal_close`，并回传 `terminal_started`、`terminal_output`、`terminal_exit`。
- Web UI 在 Agents 表增加 `Open` 按钮，打开后显示内嵌终端面板。
- 终端能力需要 server 和 agent 两端都配置 `terminal_enabled=true`。
- agent 支持 `terminal_shell`、`terminal_work_dir`、`terminal_max_sessions` 配置。

## 安全边界

- 终端 shell 以 agent 进程权限运行。
- Web UI 仍无登录，仅适合可信局域网。
- 建议 agent 使用低权限专用用户运行。
- 终端输出和命令当前不做持久化日志。

## 验证

- `cargo fmt --check`
- `cargo check`
- `make test`
- `make debug`
- `make`
- 本地 smoke：临时启动 debug server/agent，连接 `/api/agents/{name}/terminal/ws`，打开 PTY，发送 `printf smoke-terminal; exit`，确认收到输出和 exit。

## 结果

Agent Web 终端链路已实现。当前前端为内嵌轻量终端面板，能转发常用键盘输入、粘贴、窗口 resize 和关闭事件；复杂全屏 TUI 程序的 ANSI 渲染能力有限。
