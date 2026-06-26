# buildsvc architecture notes

## Goal

`buildsvc` is a lightweight private build dispatcher.

It is not a CI platform. The server only distributes source archives and
records execution state. The agent only unpacks source archives, runs the fixed
build script in the source root, streams logs, and reports the final exit code.

## Binary and roles

The project uses one Rust binary with two runtime roles:

- `server`
- `agent`

The process can start without command-line arguments. It discovers and parses
an INI configuration file, then decides its role from configuration. For local
development or special service layouts, `-c <path>` / `--config <path>` can be
used to read an explicit configuration file.

Configuration discovery:

- Explicit CLI config: `buildsvc --config /path/to/buildsvc.ini`
- Linux service: `/etc/buildsvc/buildsvc.ini`
- Windows service: `C:\ProgramData\buildsvc\buildsvc.ini`
- Development fallback: `./buildsvc.ini`

An explicit CLI config has priority. Without `--config`, service paths have
priority and `./buildsvc.ini` is only a development fallback.

The INI file contains the full configuration. A server only uses server-related
sections, and an agent only uses agent-related sections.

Recommended Linux server configuration:

```ini
[core]
role = server
data_dir = /var/lib/buildsvc
log_level = info

[server]
listen = 0.0.0.0:8080
public_url = http://127.0.0.1:8080
db_path = /var/lib/buildsvc/buildsvc.db
log_retention_days = 7
agent_offline_after_sec = 15
agent_heartbeat_sec = 5
script_timeout_sec = 7200
kill_grace_sec = 10
max_upload_size_mb = 2048

[agent.linux-amd64-01]
enabled = true
```

Recommended Linux agent configuration:

```ini
[core]
role = agent
data_dir = /var/lib/buildsvc-agent
log_level = info

[agent]
server_url = ws://127.0.0.1:8080/api/agent/ws
name = linux-amd64-01
work_dir = /var/lib/buildsvc-agent/work
concurrency = 2
heartbeat_sec = 5
script_timeout_sec = 7200
kill_grace_sec = 10
```

Project-local examples:

```text
configs/server.ini
configs/agent.ini
configs/server.test.ini
configs/agent.test.ini
```

Development startup:

```text
buildsvc --config configs/server.test.ini
buildsvc --config configs/agent.test.ini
```

Suggested defaults:

| Field | Default |
| --- | --- |
| `core.log_level` | `info` |
| `server.listen` | `0.0.0.0:8080` |
| `server.db_path` | `<core.data_dir>/buildsvc.db` |
| `server.log_retention_days` | `7` |
| `server.agent_offline_after_sec` | `15` |
| `server.agent_heartbeat_sec` | `5` |
| `server.script_timeout_sec` | `7200` |
| `server.kill_grace_sec` | `10` |
| `server.max_upload_size_mb` | `2048` |
| `agent.work_dir` | `<core.data_dir>/work` |
| `agent.concurrency` | `1` |
| `agent.heartbeat_sec` | `5` |
| `agent.script_timeout_sec` | `7200` |
| `agent.kill_grace_sec` | `10` |

## Supported platforms

First version targets:

- Linux
- Windows
- macOS

The design should not hard-code assumptions that prevent adding more platforms
later.

## Source archives

Supported archive formats in the first version:

- `tar.gz`
- `zip`

No resumable upload in the first version.

Deferred:

- resumable upload / chunked upload for large source archives

Archive structure rule:

- the archive top level must contain exactly one directory
- the build script is executed from that extracted source root

Each run uses an isolated work directory, for example:

```text
<agent_work_dir>/runs/<run_id>/src
```

After a script finishes, server-side policy can decide whether source archives
and run work data should be kept or removed.

Server data layout:

```text
<server_data_dir>/
  buildsvc.db
  sources/
    <build_id>/
      source.tar.gz
      source.zip
  logs/
    <run_id>.log
  tmp/
  ui/
```

Agent data layout:

```text
<agent_data_dir>/
  work/
    runs/
      <run_id>/
        archive/
        src/
        tmp/
  logs/
  tmp/
```

The server is the source of truth for retained logs. Agent-local logs and work
directories are temporary execution data.

## Build script contract

The build script is fixed by platform. No `build.yml` is required in the first
version.

Linux and macOS:

```text
<source_root>/run-build.sh
```

Before execution, the agent runs:

```text
chmod +x ./run-build.sh
```

Windows:

```text
<source_root>\run-build.bat
```

The script exit code is the build result:

- `0`: success
- non-zero: failed

All other build behavior belongs inside the user-provided script.

## Server responsibilities

The server is responsible for:

- accepting source archive uploads
- letting the user select target agents
- creating one run per selected agent
- tracking agent status in real time
- assigning runs to connected agents
- receiving status updates, logs, and final exit codes
- showing build/run state in the web UI
- marking interrupted runs as `lost`
- retaining logs for one week
- supporting manual rerun of failed/lost runs

The server does not build, cross-compile, package artifacts, or interpret build
output.

## Agent responsibilities

The agent is responsible for:

- connecting to the server
- authenticating with its generated token
- reporting name, computer name, IP, platform, concurrency, and current state
- receiving assigned runs
- downloading the source archive
- extracting it into an isolated run directory
- running the fixed platform script
- streaming stdout/stderr to the server
- reporting success, failure, cancellation, or lost state
- terminating local child processes when cancellation or connection loss requires
  it

The agent does not decide build policy and does not manage build artifacts for
the first version.

## Agent selection

After uploading a source archive, the user can choose which agents should run
the build.

The UI selects targets by explicit agent names.

Each selected agent creates a separate run.

## Concurrency

Agents may run builds concurrently. Concurrency is configured per agent.

Every concurrent run must use an independent run directory.

## Realtime communication

Agent-to-server communication uses WebSocket, not HTTP polling.

The WebSocket carries:

- agent hello/authentication
- heartbeat
- agent status updates
- run assignment
- run status changes
- stdout/stderr log events
- cancellation commands

All WebSocket messages are JSON text messages in the first version. Binary
frames are not required.

Large source archive transfer should still use HTTP download/upload rather than
putting file payloads into the WebSocket.

The browser UI can also use WebSocket to receive real-time updates from the
server.

Initial agent WebSocket message examples:

```json
{
  "type": "hello",
  "name": "linux-amd64-01",
  "token": "agent-generated-token",
  "platform": "linux",
  "arch": "x86_64",
  "concurrency": 2,
  "version": "0.1.0"
}
```

```json
{
  "type": "heartbeat",
  "running": 1,
  "capacity": 2,
  "runs": [
    {
      "run_id": "run_123",
      "state": "running"
    }
  ]
}
```

```json
{
  "type": "run_log",
  "run_id": "run_123",
  "stream": "stdout",
  "seq": 42,
  "data": "base64-encoded-log-bytes"
}
```

Initial server-to-agent examples:

```json
{
  "type": "run_start",
  "run_id": "run_123",
  "build_id": "build_456",
  "source_url": "http://127.0.0.1:8080/api/runs/run_123/source",
  "archive_format": "tar.gz",
  "script_timeout_sec": 7200
}
```

```json
{
  "type": "run_cancel",
  "run_id": "run_123",
  "reason": "user_requested"
}
```

## Authentication

Each agent has an independent token generated by the agent and stored in its data directory.

The first version does not require Web UI login because it is intended for
private LAN use.

## Web UI

The first version includes a simple Web UI served by the server process.

Required pages:

- agents page: name, computer name, IP, platform, status, capacity, current runs, last seen
- upload page: upload source archive and select target agents by name
- builds page: build list and aggregate run status
- run detail page: run status, selected agent, timestamps, exit code, live log

The UI should receive real-time updates from the server through WebSocket.

## Disconnection behavior

The system is intended for LAN use, so disconnections are expected to be rare.

If an agent disconnects:

- the server marks the affected running runs as `lost`
- the agent kills local running child processes
- no automatic rerun is performed

Failed or lost runs can be rerun manually from the UI.

If the server restarts while runs are active:

- active runs are treated as unknown/lost until agents reconnect
- if recovery is not simple in the first version, mark those runs as `lost`

## Cancellation and process cleanup

Cancellation should first try graceful termination, then force kill after a
grace period.

Linux and macOS:

- start the script in its own process group/session
- send a graceful termination signal to the process group
- after the grace period, force kill the process group

Windows:

- run the script in a process tree controlled by a Job Object if practical
- try graceful termination first
- after the grace period, force terminate the job/process tree

## Retention

Logs are retained for one week.

Source archives and extracted work directories can be removed according to
server policy after runs finish. The exact cleanup timing can be configured
later.

## Run states

Initial run states:

- `queued`
- `assigned`
- `preparing`
- `running`
- `success`
- `failed`
- `canceled`
- `lost`

## Deferred features

- resumable source archive upload
- richer artifact management
- Web UI login
- multi-server/high-availability mode
- build graph or multi-step workflow definitions
- automatic retry policies
