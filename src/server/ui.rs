pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>buildsvc</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f6f7f9;
      --panel: #ffffff;
      --line: #d9dee7;
      --text: #1e232b;
      --muted: #647084;
      --accent: #1464c0;
      --bad: #b42318;
      --good: #067647;
      --warn: #b54708;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font: 14px/1.45 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    header {
      height: 56px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 24px;
      border-bottom: 1px solid var(--line);
      background: var(--panel);
    }
    h1 { font-size: 18px; margin: 0; letter-spacing: 0; }
    main {
      display: grid;
      grid-template-columns: minmax(280px, 360px) minmax(0, 1fr);
      gap: 18px;
      padding: 18px;
      align-items: start;
      min-height: calc(100vh - 56px);
    }
    section {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 6px;
      min-width: 0;
    }
    section h2 {
      margin: 0;
      padding: 12px 14px;
      border-bottom: 1px solid var(--line);
      font-size: 14px;
    }
    .body { padding: 14px; }
    .stack { display: grid; gap: 12px; align-content: start; }
    .workspace {
      display: grid;
      gap: 12px;
      grid-template-rows: auto minmax(420px, 1fr);
      min-height: calc(100vh - 92px);
    }
    .two { display: grid; grid-template-columns: minmax(0, 0.8fr) minmax(540px, 1.4fr); gap: 18px; }
    label { display: block; font-weight: 600; margin-bottom: 5px; }
    input[type="text"], input[type="file"] {
      width: 100%;
      border: 1px solid var(--line);
      border-radius: 5px;
      padding: 8px;
      background: #fff;
      color: var(--text);
    }
    button {
      min-height: 34px;
      border: 1px solid #0f57a6;
      border-radius: 5px;
      padding: 7px 12px;
      background: var(--accent);
      color: white;
      font-weight: 600;
      cursor: pointer;
    }
    button.secondary {
      background: #fff;
      color: var(--accent);
    }
    button.danger {
      border-color: var(--bad);
      background: var(--bad);
    }
    table {
      width: 100%;
      border-collapse: collapse;
      table-layout: fixed;
    }
    .table-scroll {
      height: 229px;
      overflow: auto;
      border: 1px solid var(--line);
      border-radius: 5px;
    }
    .table-scroll table { border: 0; }
    .table-scroll thead th {
      position: sticky;
      top: 0;
      z-index: 1;
      background: var(--panel);
    }
    th, td {
      padding: 8px 10px;
      border-bottom: 1px solid var(--line);
      text-align: left;
      vertical-align: middle;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      height: 38px;
    }
    th { color: var(--muted); font-size: 12px; font-weight: 700; }
    tr.selectable { cursor: pointer; }
    tr.selectable:hover { background: #f0f5fb; }
    .status {
      display: inline-flex;
      align-items: center;
      min-width: 72px;
      justify-content: center;
      border-radius: 999px;
      padding: 2px 8px;
      font-size: 12px;
      font-weight: 700;
      background: #edf2f7;
      color: #344054;
    }
    .status.success, .status.idle { background: #e7f6ee; color: var(--good); }
    .status.failed, .status.lost, .status.offline { background: #fde8e7; color: var(--bad); }
    .status.running, .status.busy, .status.assigned, .status.preparing { background: #e8f1fc; color: var(--accent); }
    .status.canceled { background: #fff1e5; color: var(--warn); }
    .muted { color: var(--muted); }
    .agent-list {
      display: grid;
      gap: 8px;
      max-height: 240px;
      overflow: auto;
      border: 1px solid var(--line);
      border-radius: 5px;
      padding: 8px;
    }
    .agent-item {
      display: grid;
      grid-template-columns: 22px minmax(0, 1fr);
      align-items: center;
      gap: 8px;
    }
    .labels { color: var(--muted); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    #log {
      flex: 1;
      min-height: 420px;
      height: auto;
      margin: 0;
      padding: 12px;
      overflow: auto;
      border-top: 1px solid var(--line);
      background: #111827;
      color: #e5e7eb;
      font: 12px/1.5 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
    }
    .log-section {
      display: flex;
      min-height: 0;
      flex-direction: column;
    }
    .actions { display: flex; gap: 8px; flex-wrap: wrap; }
    .build-source { width: 48%; }
    .build-time { width: 30%; }
    .build-status { width: 22%; }
    .run-agent { width: 68%; }
    .run-status { width: 32%; }
    .agent-computer { width: 22%; }
    .agent-user { width: 14%; }
    .agent-ip { width: 16%; }
    .agent-os { width: 12%; }
    .agent-arch { width: 13%; }
    .agent-running { width: 11%; }
    .agent-status { width: 12%; }
    @media (max-width: 920px) {
      main { grid-template-columns: 1fr; }
      .two { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <header>
    <h1>buildsvc</h1>
    <div id="socketStatus" class="muted">connecting</div>
  </header>
  <main>
    <div class="stack">
      <section>
        <h2>Submit Build</h2>
        <form id="uploadForm" class="body stack">
          <div>
            <label for="source">Source archive</label>
            <input id="source" name="source" type="file" accept=".zip,.tgz,.tar.gz" required>
          </div>
          <div>
            <label>Agents</label>
            <div id="agentChecks" class="agent-list"></div>
          </div>
          <div>
            <label for="targetLabels">Labels</label>
            <input id="targetLabels" name="target_labels" type="text" placeholder="linux,amd64">
          </div>
          <button type="submit">Start</button>
          <div id="submitResult" class="muted"></div>
        </form>
      </section>
      <section>
        <h2>Runs</h2>
        <div class="body">
          <div class="table-scroll">
            <table>
              <thead>
                <tr>
                  <th class="run-agent">Computer / IP</th>
                  <th class="run-status">Status</th>
                </tr>
              </thead>
              <tbody id="runsBody"></tbody>
            </table>
          </div>
        </div>
      </section>
    </div>
    <div class="workspace">
      <div class="two">
        <section>
          <h2>Builds</h2>
          <div class="body">
            <div class="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th class="build-source">Source</th>
                    <th class="build-time">Uploaded</th>
                    <th class="build-status">Status</th>
                  </tr>
                </thead>
                <tbody id="buildsBody"></tbody>
              </table>
            </div>
          </div>
        </section>
        <section>
          <h2>Agents</h2>
          <div class="body">
            <div class="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th class="agent-computer">Computer</th>
                    <th class="agent-user">User</th>
                    <th class="agent-ip">IP</th>
                    <th class="agent-os">OS</th>
                    <th class="agent-arch">Arch</th>
                    <th class="agent-running">Tasks</th>
                    <th class="agent-status">Status</th>
                  </tr>
                </thead>
                <tbody id="agentsBody"></tbody>
              </table>
            </div>
          </div>
        </section>
      </div>
      <section class="log-section">
        <h2>Run Log</h2>
        <div class="body">
          <div class="actions">
            <button id="rerunBtn" class="secondary" type="button" disabled>Rerun</button>
            <button id="cancelBtn" class="danger" type="button" disabled>Cancel</button>
            <span id="selectedRun" class="muted"></span>
          </div>
        </div>
        <pre id="log"></pre>
      </section>
    </div>
  </main>
  <script>
    let state = { agents: [], builds: [], runs: [] };
    let selectedRun = null;

    const socketStatus = document.getElementById("socketStatus");
    const uploadForm = document.getElementById("uploadForm");
    const submitResult = document.getElementById("submitResult");
    const agentChecks = document.getElementById("agentChecks");
    const agentsBody = document.getElementById("agentsBody");
    const buildsBody = document.getElementById("buildsBody");
    const runsBody = document.getElementById("runsBody");
    const logEl = document.getElementById("log");
    const selectedRunEl = document.getElementById("selectedRun");
    const rerunBtn = document.getElementById("rerunBtn");
    const cancelBtn = document.getElementById("cancelBtn");

    function status(value) {
      return `<span class="status ${escapeHtml(value)}">${escapeHtml(value)}</span>`;
    }

    function escapeHtml(value) {
      return String(value ?? "").replace(/[&<>"']/g, ch => ({
        "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;"
      }[ch]));
    }

    function display(value) {
      return value === null || value === undefined || value === "" ? "-" : value;
    }

    function formatTime(epochSeconds) {
      if (!epochSeconds) return "-";
      return new Date(epochSeconds * 1000).toLocaleString();
    }

    function formatOs(value) {
      const os = String(value || "").toLowerCase();
      if (os === "linux") return "Linux";
      if (os === "windows") return "Windows";
      if (os === "macos" || os === "darwin") return "Mac";
      return display(value);
    }

    function formatArch(value) {
      const arch = String(value || "").toLowerCase();
      if (arch === "aarch64") return "arm64";
      if (arch === "x86_64" || arch === "amd64") return "x86_64";
      return display(value);
    }

    function agentComputerIp(agentName) {
      const agent = state.agents.find(item => item.name === agentName);
      if (!agent) return agentName;
      const computer = display(agent.computer_name);
      const ip = display(agent.ip);
      return ip === "-" ? computer : `${computer} / ${ip}`;
    }

    function render() {
      agentChecks.innerHTML = state.agents.map(agent => `
        <label class="agent-item">
          <input type="checkbox" name="agent" value="${escapeHtml(agent.name)}" ${agent.enabled ? "" : "disabled"}>
          <span>
            <span>${escapeHtml(display(agent.computer_name))} ${status(agent.status)}</span>
            <span class="labels">${escapeHtml(agent.name)} · ${escapeHtml(agent.labels.join(","))}</span>
          </span>
        </label>`).join("");

      agentsBody.innerHTML = state.agents.map(agent => `
        <tr>
          <td title="${escapeHtml(display(agent.computer_name))}">${escapeHtml(display(agent.computer_name))}</td>
          <td title="${escapeHtml(display(agent.username))}">${escapeHtml(display(agent.username))}</td>
          <td title="${escapeHtml(display(agent.ip))}">${escapeHtml(display(agent.ip))}</td>
          <td>${escapeHtml(formatOs(agent.platform))}</td>
          <td>${escapeHtml(formatArch(agent.arch))}</td>
          <td>${agent.running}/${agent.capacity}</td>
          <td>${status(agent.status)}</td>
        </tr>`).join("");

      const builds = [...state.builds].sort((a, b) => (b.created_at || 0) - (a.created_at || 0));
      buildsBody.innerHTML = builds.map(build => `
        <tr>
          <td title="${escapeHtml(build.source_name)}">${escapeHtml(build.source_name)}</td>
          <td title="${escapeHtml(formatTime(build.created_at))}">${escapeHtml(formatTime(build.created_at))}</td>
          <td>${status(build.status)}</td>
        </tr>`).join("");

      runsBody.innerHTML = state.runs.map(run => `
        <tr class="selectable" data-run="${escapeHtml(run.id)}">
          <td title="${escapeHtml(run.agent_name)}">${escapeHtml(agentComputerIp(run.agent_name))}</td>
          <td>${status(run.status)}</td>
        </tr>`).join("");

      for (const row of runsBody.querySelectorAll("tr[data-run]")) {
        row.addEventListener("click", () => selectRun(row.dataset.run));
      }

      const run = state.runs.find(item => item.id === selectedRun);
      selectedRunEl.textContent = run ? `${run.id} on ${run.agent_name}` : "";
      rerunBtn.disabled = !run || !["success", "failed", "lost", "canceled"].includes(run.status);
      cancelBtn.disabled = !run || !["queued", "assigned", "preparing", "running"].includes(run.status);
    }

    async function selectRun(runId) {
      selectedRun = runId;
      render();
      const response = await fetch(`/api/runs/${encodeURIComponent(runId)}/log`);
      logEl.textContent = await response.text();
      logEl.scrollTop = logEl.scrollHeight;
    }

    uploadForm.addEventListener("submit", async event => {
      event.preventDefault();
      const form = new FormData();
      const file = document.getElementById("source").files[0];
      if (!file) return;
      const selected = Array.from(document.querySelectorAll("input[name='agent']:checked"))
        .map(input => input.value);
      form.append("source", file);
      form.append("target_agents", selected.join(","));
      form.append("target_labels", document.getElementById("targetLabels").value);
      submitResult.textContent = "uploading";
      const response = await fetch("/api/builds", { method: "POST", body: form });
      if (!response.ok) {
        submitResult.textContent = await response.text();
        return;
      }
      state = await response.json();
      submitResult.textContent = "submitted";
      uploadForm.reset();
      render();
    });

    rerunBtn.addEventListener("click", async () => {
      if (!selectedRun) return;
      await fetch(`/api/runs/${encodeURIComponent(selectedRun)}/rerun`, { method: "POST" });
    });

    cancelBtn.addEventListener("click", async () => {
      if (!selectedRun) return;
      await fetch(`/api/runs/${encodeURIComponent(selectedRun)}/cancel`, { method: "POST" });
    });

    function connect() {
      const ws = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/api/ui/ws`);
      ws.onopen = () => { socketStatus.textContent = "live"; };
      ws.onclose = () => {
        socketStatus.textContent = "reconnecting";
        setTimeout(connect, 1500);
      };
      ws.onmessage = event => {
        const message = JSON.parse(event.data);
        if (message.type === "state") {
          state = message.state;
          render();
        } else if (message.type === "log" && message.run_id === selectedRun) {
          logEl.textContent += message.data;
          logEl.scrollTop = logEl.scrollHeight;
        }
      };
    }

    connect();
  </script>
</body>
</html>
"#;
