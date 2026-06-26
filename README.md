# buildsvc

[English](README.en.md) | 简体中文

## 简体中文

buildsvc 是一个轻量级自用分布式编译服务。它用一个 Rust 单二进制同时支持 server 和 agent 两种角色，角色由 INI 配置中的 `[core].role` 决定。

server 负责 Web UI、源码包上传、任务分发、状态展示和日志保存；agent 主动连接 server，下载源码包，解包后执行源码根目录里的预设脚本，并把执行状态和日志实时回传给 server。

它的目标是替代临时脚本、SSH 手工登录和重型 CI 系统，适合局域网内自用构建环境。第一版不包含登录认证、Git 托管、队列系统、Kubernetes 或 Jenkins 类功能。

### 主要能力

- 单二进制运行：同一个 `buildsvc` 可作为 server 或 agent。
- INI 配置：默认读取系统配置文件，也支持 `--config <path>` 指定配置。
- Web UI：上传源码包、选择目标 agent、查看 agent 状态、run 状态和实时日志。
- 多平台 agent：目标支持 Linux、Windows、macOS。
- 源码包格式：支持 `.tar.gz` 和 `.zip`。
- 固定脚本入口：Linux/macOS 执行 `run-build.sh`，Windows 执行 `run-build.bat`。
- 并发构建：每个 agent 可配置本机并发数。
- 心跳状态：agent 通过 WebSocket 连接和心跳上报在线状态，断开后 server 会及时标记离线。
- 任务删除：删除 run 时会先让在线 agent 删除对应工作区，再删除 server 记录。
- 源码删除：build 无关联 run 时可删除 server 侧源码包。
- 可选 Web 终端：不依赖 SSH，通过 agent 在目标机器上创建 PTY 会话。仅建议在可信局域网内启用。
- 可选远程升级：server 上传 deb/rpm/Gentoo overlay 包，agent 校验后通过系统包管理器安装并重启 service。
- Linux 打包：支持 deb、rpm、Gentoo emerge overlay。

### 构建和测试

需要先安装 Rust 工具链和 `make`。

```bash
make          # release 构建，生成 target/release/buildsvc
make debug    # debug 构建，生成 target/debug/buildsvc
make test     # 运行 cargo test
```

等价的直接命令：

```bash
cargo build --release
cargo build
cargo test
```

### 打包

所有包产物默认输出到 `target/package/`。

#### Debian / Ubuntu

需要 `dpkg-deb`。

```bash
make deb
sudo apt install ./target/package/buildsvc_*.deb
```

包内文件：

- `/usr/bin/buildsvc`
- `/etc/buildsvc/buildsvc.ini`
- `/usr/lib/systemd/system/buildsvc.service`
- `/usr/share/doc/buildsvc/examples/server.ini`
- `/usr/share/doc/buildsvc/examples/agent.ini`

#### RPM

需要 `rpmbuild`。

```bash
make rpm
sudo dnf install ./target/package/buildsvc-*.rpm
```

也可以在不使用 dnf 的系统上用：

```bash
sudo rpm -Uvh ./target/package/buildsvc-*.rpm
```

#### Gentoo emerge

需要 Portage。若本机有 `ebuild` 命令，`make emerge` 会同时生成 Manifest。

```bash
make emerge
sudo env PORTDIR_OVERLAY="$PWD/target/package/gentoo-overlay" emerge -av app-admin/buildsvc
```

`make emerge` 会生成：

- `target/package/gentoo-overlay/`
- `target/package/buildsvc-<version>-gentoo-overlay.tar.gz`

默认 ebuild 会按当前架构生成稳定 keyword，例如 `amd64` 或 `arm64`。如果你希望生成 unstable keyword，可以这样覆盖：

```bash
GENTOO_KEYWORDS='~amd64' make emerge
```

如果安装时遇到类似 `masked by: ~amd64 keyword`，说明当前 ebuild 使用了 unstable keyword，可以重新用默认配置执行 `make emerge`，或在 Gentoo 上手动放行：

```bash
sudo mkdir -p /etc/portage/package.accept_keywords
echo 'app-admin/buildsvc ~amd64' | sudo tee /etc/portage/package.accept_keywords/buildsvc
```

### 快速启动

开发调试时可以用项目内的两份测试配置：

```bash
./target/release/buildsvc --config configs/server.test.ini
./target/release/buildsvc --config configs/agent.test.ini
```

也可以直接用 cargo：

```bash
cargo run -- --config configs/server.test.ini
cargo run -- --config configs/agent.test.ini
```

Web UI 地址由 server 配置中的 `listen` 和 `public_url` 决定。测试配置默认使用 `http://127.0.0.1:18080`。

`configs/server.ini` 和 `configs/agent.ini` 是发布/打包用配置，会作为示例配置打进 Linux 包；本地调试优先使用 `configs/server.test.ini` 和 `configs/agent.test.ini`。

打开 Web UI 后，在 Builds tab 上传 `.tar.gz` 或 `.zip` 源码包，选择目标 agent 或 labels，server 会创建 run 并分发给在线 agent。

### 作为 service 运行

安装包内置 systemd unit：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now buildsvc
sudo systemctl status buildsvc
journalctl -u buildsvc -f
```

默认 unit 不传命令行参数：

```ini
ExecStart=/usr/bin/buildsvc
```

因此它会自动读取默认配置文件：

- Linux/macOS：`/etc/buildsvc/buildsvc.ini`
- Windows：`C:\ProgramData\buildsvc\buildsvc.ini`
- 开发 fallback：当前目录的 `./buildsvc.ini`

如果同一台机器上要同时运行 server 和 agent，建议准备两份配置文件，并为第二个进程单独创建 service unit，或开发调试时用 `--config` 指定。

### 远程升级

远程升级默认关闭，需要 server 和 agent 两端都配置：

```ini
upgrade_enabled = true
```

Web UI 的 Upgrades tab 支持上传：

- deb：`make deb` 生成的 `.deb`。
- rpm：`make rpm` 生成的 `.rpm`。
- emerge：`make emerge` 生成的 `buildsvc-<version>-gentoo-overlay.tar.gz`。

升级流程：

1. server 保存上传包并计算 sha256。
2. server 通过 agent WebSocket 下发升级指令。
3. agent 下载升级包并校验 sha256。
4. agent 调用对应包管理器安装。
5. agent 执行 `systemctl daemon-reload` 和 `systemctl restart buildsvc`。
6. agent 重连后在 Agents 表显示新版本。

配置文件不会被强制覆盖：

- deb 使用 dpkg conffile，并以 `--force-confold` 安装。
- rpm 使用 `%config(noreplace)`。
- Gentoo 走 Portage 的 `CONFIG_PROTECT`。

限制：

- 当前远程包升级只支持 Linux agent。
- agent 不能有正在运行的 build run。
- agent 进程需要有安装系统包和重启 `buildsvc` service 的权限，通常应作为 root service 运行。

### 源码包规范

源码包必须只有一个顶层目录。脚本必须放在这个顶层目录里。

Linux/macOS 示例：

```text
my-project/
  run-build.sh
  src/
  Makefile
```

Windows 示例：

```text
my-project/
  run-build.bat
  src/
```

agent 解包后会进入源码根目录执行脚本：

- Linux/macOS：执行前会先给 `run-build.sh` 加执行权限。
- Windows：执行 `run-build.bat`。
- 脚本退出码为 `0` 时 run 标记为成功，非 `0` 时标记为失败。
- 脚本 stdout/stderr 会实时回传到 server，并在 Web UI 的 Run Log 中显示。

### 配置文件结构

配置文件使用 INI 格式。`[core].role` 决定当前进程是 server 还是 agent。建议 server 和 agent 使用独立配置文件；如果放在同一个文件里，也要保证相关字段都是有效值。

#### `[core]`

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `role` | 进程角色，`server` 或 `agent` | 必填 |
| `data_dir` | 数据目录 | Linux/macOS: `/var/lib/buildsvc`；Windows: `C:\ProgramData\buildsvc\data` |
| `log_level` | tracing 日志级别 | `info` |

#### `[server]`

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `listen` | HTTP/WebSocket 监听地址 | `0.0.0.0:8080` |
| `public_url` | agent 下载源码包时访问的 server URL，必须对 agent 可达 | `http://127.0.0.1:8080` |
| `db_path` | SQLite 数据库路径 | `<data_dir>/buildsvc.db` |
| `log_retention_days` | run 日志保留天数 | `7` |
| `agent_offline_after_sec` | 多久未收到 agent 心跳后标记离线 | `15` |
| `agent_heartbeat_sec` | server 下发给 agent 的心跳间隔 | `5` |
| `script_timeout_sec` | 默认脚本超时时间 | `7200` |
| `kill_grace_sec` | 取消或超时后优雅终止等待时间 | `10` |
| `max_upload_size_mb` | 最大上传源码包大小 | `2048` |
| `terminal_enabled` | 是否允许 Web UI 打开 agent 终端 | `false` |
| `upgrade_enabled` | 是否允许 Web UI 推送升级包 | `false` |

server 还需要为允许连接的 agent 配置独立 token：

```ini
[agent.local-linux]
token = change-me-local-linux
labels = linux,amd64,local
enabled = true
```

这里的 `local-linux` 必须与对应 agent 配置中的 `[agent].name` 一致。

#### `[agent]`

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `server_url` | server 的 agent WebSocket 地址，通常是 `ws://<server>/api/agent/ws` | 必填 |
| `name` | agent 名称，需要匹配 server 的 `[agent.<name>]` | 必填 |
| `token` | agent token，需要匹配 server 配置 | 必填 |
| `advertise_ip` | agent 上报给 UI 的 IP。多网卡机器建议显式配置 | 自动探测 |
| `labels` | agent 标签，用逗号分隔，上传任务时可按标签选择 | 必填 |
| `work_dir` | agent 工作目录 | `<data_dir>/work` |
| `concurrency` | agent 本机并发 run 数 | `1` |
| `heartbeat_sec` | agent 本地心跳间隔；server 接受连接后会以下发值为准 | `5` |
| `script_timeout_sec` | 脚本超时时间 | `7200` |
| `kill_grace_sec` | 优雅终止等待时间 | `10` |
| `terminal_enabled` | 是否允许此 agent 创建 Web 终端会话 | `false` |
| `terminal_shell` | Web 终端 shell | Linux/macOS 使用 `$SHELL` 或 `/bin/sh`；Windows 使用 `%COMSPEC%` 或 `cmd.exe` |
| `terminal_work_dir` | Web 终端工作目录 | `<work_dir>/terminal` |
| `terminal_max_sessions` | 同时允许的终端会话数 | `1` |
| `upgrade_enabled` | 是否允许此 agent 执行远程包升级 | `false` |
| `upgrade_work_dir` | 远程升级包下载和解包目录 | `<work_dir>/upgrades` |

### 最小配置示例

server：

```ini
[core]
role = server
data_dir = /var/lib/buildsvc
log_level = info

[server]
listen = 0.0.0.0:8080
public_url = http://192.168.1.10:8080
db_path = /var/lib/buildsvc/buildsvc.db
terminal_enabled = false
upgrade_enabled = false

[agent.linux-a]
token = replace-with-a-long-random-token
labels = linux,amd64
enabled = true
```

agent：

```ini
[core]
role = agent
data_dir = /var/lib/buildsvc-agent
log_level = info

[agent]
server_url = ws://192.168.1.10:8080/api/agent/ws
name = linux-a
token = replace-with-a-long-random-token
labels = linux,amd64
work_dir = /var/lib/buildsvc-agent/work
concurrency = 1
upgrade_enabled = false
```

### Web UI 使用流程

1. 启动 server。
2. 启动一个或多个 agent，确认 Web UI 中 Agents 状态为 online。
3. 准备包含顶层目录和固定构建脚本的 `.tar.gz` 或 `.zip`。
4. 在 Web UI 的 Builds tab 上传源码包，并选择目标 agents 或 labels。
5. 在 Runs 中查看执行状态，在 Run Log 中查看实时日志。
6. 如需清理，先删除对应 runs；build 没有关联 runs 后可删除源码包。
7. 如启用了 Web 终端，可以从 Agents 区域打开目标机器终端执行命令。
8. 如启用了远程升级，可以在 Upgrades tab 上传 deb/rpm/emerge 包并推送到在线 agent。

### 安全说明

- 第一版 Web UI 无登录认证，建议只在可信局域网或受控网络内使用。
- agent token 要使用足够长的随机值，不要使用示例里的 `change-me-*`。
- Web 终端等同于在 agent 机器上执行命令，默认关闭；仅在信任 server 和网络边界时启用。
- 远程升级等同于允许 Web UI 在 agent 机器上安装系统包并重启 service，默认关闭；仅在可信网络和可信 server 上启用。
- `public_url` 必须是 agent 可以访问的地址，不要只写 server 本机的 `127.0.0.1`，除非 agent 和 server 在同一台机器上。
