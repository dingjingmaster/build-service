# buildsvc

English | [简体中文](README.md)

## Overview

buildsvc is a lightweight self-hosted distributed build service. A single Rust binary can run as either a server or an agent, selected by `[core].role` in an INI configuration file.

The server provides the Web UI, accepts source archive uploads, dispatches runs, stores logs, and shows live state. Agents connect to the server, download assigned source archives, unpack them, run the preset build script from the source root, and stream status and logs back to the server.

It is designed for trusted LAN and personal build environments. It is not a Git hosting service, Jenkins replacement, Kubernetes system, or full CI platform.

## Features

- Single binary for both server and agent.
- INI-based configuration, with default service paths and optional `--config <path>`.
- Web UI for uploads, target selection, agent status, run status, and live logs.
- Linux, Windows, and macOS agent targets.
- Source archive support for `.tar.gz` and `.zip`.
- Fixed script entrypoint: `run-build.sh` on Linux/macOS and `run-build.bat` on Windows.
- Per-agent concurrency.
- WebSocket agent connection and heartbeat based online/offline state.
- Run deletion with agent workspace cleanup confirmation.
- Build deletion when no runs are attached.
- Optional Web terminal through the agent, without SSH. Enable only on trusted networks.
- Optional remote upgrade: upload deb/rpm/Gentoo overlay packages on the server and let agents install them through the local package manager.
- Linux packaging for deb, rpm, and Gentoo emerge overlay.

## Build And Test

Install Rust and `make` first.

```bash
make          # release build: target/release/buildsvc
make debug    # debug build: target/debug/buildsvc
make test     # run cargo test
```

Equivalent Cargo commands:

```bash
cargo build --release
cargo build
cargo test
```

## Packaging And Installation

Linux package artifacts are written under `target/package/`. deb, rpm, and Gentoo packages install a systemd unit and automatically run `daemon-reload`, `enable`, and `restart` after install or upgrade. On uninstall, package scripts stop and disable the service, then reload systemd.

Generated packages install:

- `/usr/bin/buildsvc`
- `/etc/buildsvc/buildsvc.ini`
- `/usr/lib/systemd/system/buildsvc.service`
- `/usr/share/doc/buildsvc/examples/buildsvc.ini`
- If present: `/usr/share/doc/buildsvc/examples/server.ini`
- If present: `/usr/share/doc/buildsvc/examples/agent.ini`

The `/etc/buildsvc/buildsvc.ini` package config is selected from non-empty `configs/buildsvc.ini` first, then falls back to `packaging/buildsvc.ini`.

The default packaged `[core].role` is `agent`, so a typical agent host usually only needs edits to fields such as `server_url` and `name`. The agent token is generated on first start and saved to `<data_dir>/agent.token`.

### Debian / Ubuntu / Linux Mint

```bash
make deb
sudo apt install ./target/package/buildsvc_*.deb
```

Edit config and restart after installation:

```bash
sudoedit /etc/buildsvc/buildsvc.ini
sudo systemctl restart buildsvc
sudo systemctl status buildsvc
```

Upgrade and uninstall:

```bash
sudo apt install ./target/package/buildsvc_*.deb
sudo apt remove buildsvc
```

### Fedora / RHEL / Rocky / AlmaLinux / openSUSE

```bash
make rpm
sudo dnf install ./target/package/buildsvc-*.rpm
```

Other rpm-based systems can use:

```bash
sudo yum localinstall ./target/package/buildsvc-*.rpm
sudo zypper install ./target/package/buildsvc-*.rpm
sudo rpm -Uvh ./target/package/buildsvc-*.rpm
```

Edit config and restart after installation:

```bash
sudoedit /etc/buildsvc/buildsvc.ini
sudo systemctl restart buildsvc
sudo systemctl status buildsvc
```

Upgrade and uninstall:

```bash
sudo dnf install ./target/package/buildsvc-*.rpm
sudo dnf remove buildsvc
```

### Gentoo

```bash
make emerge
```

`make emerge` generates:

- `target/package/gentoo-overlay/`
- `target/package/buildsvc-<version>-gentoo-overlay.tar.gz`

Install using the local overlay temporarily:

```bash
sudo env PORTDIR_OVERLAY="$PWD/target/package/gentoo-overlay" emerge -av app-admin/buildsvc
```

Or unpack the overlay into a fixed path and install from there:

```bash
sudo mkdir -p /var/local/overlays/buildsvc
sudo tar -xf target/package/buildsvc-*-gentoo-overlay.tar.gz -C /var/local/overlays/buildsvc --strip-components=1
sudo env PORTDIR_OVERLAY="/var/local/overlays/buildsvc" emerge -av app-admin/buildsvc
```

Gentoo ebuilds default to a stable keyword for the host architecture, such as `amd64` or `arm64`. To generate an unstable keyword intentionally:

```bash
GENTOO_KEYWORDS='~amd64' make emerge
```

If emerge reports a keyword mask such as `masked by: ~amd64 keyword`, either regenerate with the default stable keyword or allow it explicitly:

```bash
sudo mkdir -p /etc/portage/package.accept_keywords
echo 'app-admin/buildsvc ~amd64' | sudo tee /etc/portage/package.accept_keywords/buildsvc
sudo env PORTDIR_OVERLAY="$PWD/target/package/gentoo-overlay" emerge -av app-admin/buildsvc
```

Edit config after installation. The default packaged role is `agent`, so a normal agent host usually only needs `server_url` and `name`:

```bash
sudoedit /etc/buildsvc/buildsvc.ini
```

On systemd hosts, package scripts automatically run `daemon-reload`, `enable`, and `restart`. You can also check manually:

```bash
sudo systemctl restart buildsvc
sudo systemctl status buildsvc
```

On OpenRC hosts, this package does not yet install an OpenRC init script. Run it directly for now:

```bash
sudo /usr/bin/buildsvc
```

Uninstall:

```bash
sudo emerge -C app-admin/buildsvc
```

### Manual Linux Install

For distributions that do not use the package formats above, install the binary directly:

```bash
make
sudo install -Dm755 target/release/buildsvc /usr/local/bin/buildsvc
sudo install -Dm644 configs/buildsvc.ini /etc/buildsvc/buildsvc.ini
sudoedit /etc/buildsvc/buildsvc.ini
/usr/local/bin/buildsvc
```

To run it at boot with systemd:

```bash
sudo tee /etc/systemd/system/buildsvc.service >/dev/null <<'EOF'
[Unit]
Description=buildsvc
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/buildsvc
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now buildsvc
```

### Manual Windows Install

There is no native Windows package yet. A Windows agent can run `buildsvc.exe` directly. The default config path is `C:\ProgramData\buildsvc\buildsvc.ini`.

Build on Windows:

```powershell
cargo build --release
```

Install an agent from an Administrator PowerShell:

```powershell
New-Item -ItemType Directory -Force "C:\Program Files\buildsvc", "C:\ProgramData\buildsvc" | Out-Null
Copy-Item .\target\release\buildsvc.exe "C:\Program Files\buildsvc\buildsvc.exe" -Force
@'
[core]
role = agent
data_dir = C:\ProgramData\buildsvc\data
log_level = info

[agent]
server_url = ws://SERVER_IP:8080/api/agent/ws
name = windows-agent-1
work_dir = C:\ProgramData\buildsvc\work
concurrency = 1
'@ | Set-Content -Encoding UTF8 "C:\ProgramData\buildsvc\buildsvc.ini"
notepad "C:\ProgramData\buildsvc\buildsvc.ini"
& "C:\Program Files\buildsvc\buildsvc.exe"
```

For dependency-free startup, use Task Scheduler to wrap the normal process:

```powershell
schtasks /Create /TN buildsvc /SC ONSTART /RL HIGHEST /RU SYSTEM /TR "`"C:\Program Files\buildsvc\buildsvc.exe`""
schtasks /Run /TN buildsvc
```

Stop and uninstall:

```powershell
taskkill /IM buildsvc.exe /F
schtasks /Delete /TN buildsvc /F
Remove-Item "C:\Program Files\buildsvc" -Recurse -Force
```

### Manual macOS Install

There is no native macOS package yet. A macOS agent can run the binary directly. The default config path is `/etc/buildsvc/buildsvc.ini`.

Build and install on macOS:

```bash
cargo build --release
sudo install -d /usr/local/bin /etc/buildsvc /var/lib/buildsvc
sudo install -m 0755 target/release/buildsvc /usr/local/bin/buildsvc
sudo install -m 0644 configs/buildsvc.ini /etc/buildsvc/buildsvc.ini
sudoedit /etc/buildsvc/buildsvc.ini
/usr/local/bin/buildsvc
```

To run it at boot with launchd:

```bash
sudo tee /Library/LaunchDaemons/com.local.buildsvc.plist >/dev/null <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.local.buildsvc</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/buildsvc</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/var/log/buildsvc.log</string>
  <key>StandardErrorPath</key>
  <string>/var/log/buildsvc.err</string>
</dict>
</plist>
EOF

sudo launchctl bootstrap system /Library/LaunchDaemons/com.local.buildsvc.plist
sudo launchctl enable system/com.local.buildsvc
sudo launchctl kickstart -k system/com.local.buildsvc
```

Stop and uninstall:

```bash
sudo launchctl bootout system /Library/LaunchDaemons/com.local.buildsvc.plist
sudo rm -f /Library/LaunchDaemons/com.local.buildsvc.plist
sudo rm -f /usr/local/bin/buildsvc
```

## Quick Start

Test configs are included for local development:

```bash
./target/release/buildsvc --config configs/server.test.ini
./target/release/buildsvc --config configs/agent.test.ini
```

Or:

```bash
cargo run -- --config configs/server.test.ini
cargo run -- --config configs/agent.test.ini
```

The Web UI address is controlled by `listen` and `public_url` in the server config. The test configs default to `http://127.0.0.1:18080`.

`configs/buildsvc.ini` is the release/package default config and is installed as a package example. Use `configs/server.test.ini` and `configs/agent.test.ini` for local development.

Open the Web UI, use the Builds tab to upload a `.tar.gz` or `.zip` archive, choose target agents, and watch the run state and logs.

## Service Mode

The package includes a systemd unit. After installing or upgrading via deb, rpm, or the Gentoo overlay, package scripts automatically run:

```bash
systemctl daemon-reload
systemctl enable buildsvc.service
systemctl restart buildsvc.service
```

On uninstall, package scripts stop and disable the service, then reload systemd. Check service state and logs with:

```bash
sudo systemctl status buildsvc
journalctl -u buildsvc -f
```

The packaged systemd unit runs without command-line arguments:

```ini
ExecStart=/usr/bin/buildsvc
```

Default config discovery:

- Linux/macOS: `/etc/buildsvc/buildsvc.ini`
- Windows: `C:\ProgramData\buildsvc\buildsvc.ini`
- Development fallback: `./buildsvc.ini`

To run server and agent on the same host, use two config files and either start one process manually with `--config` or create a second service unit.

## Remote Upgrade

Remote upgrade is disabled by default. Enable it on both server and agent:

```ini
upgrade_enabled = true
```

The Upgrades tab in the Web UI accepts:

- deb: the `.deb` generated by `make deb`.
- rpm: the `.rpm` generated by `make rpm`.
- emerge: the `buildsvc-<version>-gentoo-overlay.tar.gz` generated by `make emerge`.

Upgrade flow:

1. The server stores the uploaded package and computes its sha256.
2. The server sends an upgrade command over the agent WebSocket.
3. The agent downloads the package and verifies sha256.
4. The agent installs it with the matching package manager.
5. The agent runs `systemctl daemon-reload` and `systemctl restart buildsvc`.
6. After reconnecting, the new agent version appears in the Agents table.

Configuration files are not forcibly overwritten:

- deb uses dpkg conffiles and installs with `--force-confold`.
- rpm uses `%config(noreplace)`.
- Gentoo uses Portage `CONFIG_PROTECT`.

Limits:

- Remote package upgrade currently supports Linux agents only.
- The agent must have no active build runs.
- The agent process needs permission to install system packages and restart the `buildsvc` service, usually by running as a root service.

## Source Archive Contract

Archives must contain exactly one top-level directory. The script must live in that directory.

Linux/macOS:

```text
my-project/
  run-build.sh
  src/
```

Windows:

```text
my-project/
  run-build.bat
  src/
```

The agent extracts the archive into its run workspace and executes the script from the source root. On Linux/macOS it first makes `run-build.sh` executable. Exit code `0` means success; any non-zero exit code means failure. stdout and stderr are streamed to the server.

## Configuration

Configuration uses INI format. `[core].role` selects the active role. Separate server and agent config files are recommended.

Server example:

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
enabled = true
```

Agent example:

```ini
[core]
role = agent
data_dir = /var/lib/buildsvc-agent
log_level = info

[agent]
server_url = ws://192.168.1.10:8080/api/agent/ws
name = linux-a
work_dir = /var/lib/buildsvc-agent/work
concurrency = 1
upgrade_enabled = false
```

Important fields:

- `public_url` must be reachable from agents, because agents download source archives from it.
- `[agent.<name>]` on the server is optional, but when present it must match `[agent].name` on the agent.
- Agents generate their own token and save it to `<data_dir>/agent.token`; the server records it when the agent connects.
- `advertise_ip` can be set on multi-NIC machines when automatic IP detection is not what you want.
- `terminal_enabled` must be enabled on both server and agent before the Web terminal can be opened.
- `upgrade_enabled` must be enabled on both server and agent before remote package upgrades can run.
- `upgrade_work_dir` sets where agents download and extract upgrade packages.

## Security Notes

- The first version has no Web UI login. Run it only on a trusted LAN or behind your own access control.
- Agent tokens are generated automatically and stored in `<data_dir>/agent.token`; keep that file private.
- The Web terminal can execute commands on the agent machine. It is disabled by default and should stay disabled unless the network and server are trusted.
- Remote upgrade can install system packages and restart services on the agent machine. It is disabled by default and should only be enabled on trusted networks and servers.
