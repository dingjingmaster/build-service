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
      gap: 18px;
      padding: 0 24px;
      border-bottom: 1px solid var(--line);
      background: var(--panel);
    }
    h1 { font-size: 18px; margin: 0; letter-spacing: 0; }
    .tabs {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 4px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: #f8fafc;
    }
    button.tab-button {
      min-height: 30px;
      border: 0;
      border-radius: 4px;
      padding: 5px 14px;
      background: transparent;
      color: var(--muted);
    }
    button.tab-button.active {
      background: var(--accent);
      color: #fff;
    }
    main {
      padding: 18px;
      min-height: calc(100vh - 56px);
    }
    .tab-panel { display: none; }
    .tab-panel.active { display: block; }
    .main-grid {
      display: grid;
      grid-template-columns: minmax(280px, 360px) minmax(0, 1fr);
      gap: 18px;
      align-items: start;
    }
    .upgrade-grid {
      display: grid;
      grid-template-columns: minmax(320px, 420px) minmax(0, 1fr);
      gap: 18px;
      align-items: start;
      min-height: calc(100vh - 92px);
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
    input[type="text"], input[type="file"], select {
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
    button.small {
      min-height: 28px;
      padding: 4px 8px;
      font-size: 12px;
    }
    button:disabled {
      opacity: 0.5;
      cursor: not-allowed;
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
    .runs-scroll { height: 419px; }
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
    tr.selectable.selected { background: #e8f1fc; }
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
    .section-actions {
      display: flex;
      gap: 8px;
      align-items: center;
      flex-wrap: wrap;
      margin-bottom: 10px;
    }
    .build-select { width: 38px; text-align: center; }
    .build-source { width: 42%; }
    .build-time { width: 32%; }
    .build-status { width: 24%; }
    .run-select { width: 38px; text-align: center; }
    .run-agent { width: 62%; }
    .run-status { width: 30%; }
    .agent-computer { width: 20%; }
    .agent-user { width: 10%; }
    .agent-ip { width: 13%; }
    .agent-os { width: 8%; }
    .agent-arch { width: 8%; }
    .agent-version { width: 10%; }
    .agent-running { width: 8%; }
    .agent-status { width: 10%; }
    .agent-terminal { width: 13%; }
    .upgrade-log {
      flex: 1;
      min-height: 520px;
      height: auto;
      margin: 0;
      padding: 10px;
      overflow: auto;
      border-top: 1px solid var(--line);
      background: #111827;
      color: #e5e7eb;
      font: 12px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
    }
    .terminal-panel {
      position: fixed;
      inset: 72px 24px 24px 24px;
      z-index: 10;
      display: flex;
      flex-direction: column;
      min-height: 0;
      border: 1px solid #243244;
      border-radius: 6px;
      background: #0b1220;
      box-shadow: 0 16px 42px rgba(15, 23, 42, 0.35);
    }
    .terminal-panel.hidden { display: none; }
    .terminal-head {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 12px;
      border-bottom: 1px solid #243244;
      color: #e5e7eb;
      background: #111827;
    }
    .terminal-head .spacer { flex: 1; }
    .terminal-output {
      flex: 1;
      min-height: 0;
      margin: 0;
      padding: 12px;
      overflow: auto;
      outline: none;
      color: #d1e7d8;
      background: #050a12;
      font: 13px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      white-space: pre-wrap;
      overflow-wrap: anywhere;
    }
    @media (max-width: 920px) {
      header { flex-wrap: wrap; height: auto; min-height: 56px; padding: 10px 18px; }
      .main-grid, .upgrade-grid { grid-template-columns: 1fr; }
      .two { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <header>
    <h1>buildsvc</h1>
    <nav class="tabs" aria-label="Views">
      <button id="buildTab" class="tab-button active" type="button" role="tab" aria-selected="true" aria-controls="buildPanel">Builds</button>
      <button id="upgradeTab" class="tab-button" type="button" role="tab" aria-selected="false" aria-controls="upgradePanel">Upgrades</button>
    </nav>
    <div id="socketStatus" class="muted">connecting</div>
  </header>
  <main>
    <div id="buildPanel" class="tab-panel active" role="tabpanel" aria-labelledby="buildTab">
      <div class="main-grid">
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
              <div class="table-scroll runs-scroll">
                <table>
                  <thead>
                    <tr>
                      <th class="run-select"><input id="selectAllRuns" type="checkbox"></th>
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
                <div class="section-actions">
                  <button id="deleteBuildsBtn" class="danger" type="button" disabled>Delete Source</button>
                  <span id="selectedBuilds" class="muted"></span>
                </div>
                <div class="table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th class="build-select"><input id="selectAllBuilds" type="checkbox"></th>
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
                        <th class="agent-version">Version</th>
                        <th class="agent-running">Tasks</th>
                        <th class="agent-status">Status</th>
                        <th class="agent-terminal">Terminal</th>
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
                <button id="deleteBtn" class="danger" type="button" disabled>Delete</button>
                <span id="selectedRun" class="muted"></span>
              </div>
            </div>
            <pre id="log"></pre>
          </section>
        </div>
      </div>
    </div>
    <div id="upgradePanel" class="tab-panel" role="tabpanel" aria-labelledby="upgradeTab">
      <div class="upgrade-grid">
        <section>
          <h2>Upgrade Agents</h2>
          <form id="upgradeForm" class="body stack">
            <div>
              <label for="packageKind">Package type</label>
              <select id="packageKind" name="package_kind">
                <option value="deb">deb</option>
                <option value="rpm">rpm</option>
                <option value="emerge">emerge</option>
              </select>
            </div>
            <div>
              <label for="upgradePackage">Package</label>
              <input id="upgradePackage" name="package" type="file" accept=".deb,.rpm,.tgz,.tar.gz" required>
            </div>
            <div>
              <label>Agents</label>
              <div id="upgradeAgentChecks" class="agent-list"></div>
            </div>
            <button type="submit">Push Upgrade</button>
            <div id="upgradeResult" class="muted"></div>
          </form>
        </section>
        <section class="log-section">
          <h2>Upgrade Log</h2>
          <div class="body">
            <span class="muted">Package manager output and upgrade state appear here.</span>
          </div>
          <pre id="upgradeLog" class="upgrade-log"></pre>
        </section>
      </div>
    </div>
  </main>
  <div id="terminalPanel" class="terminal-panel hidden">
    <div class="terminal-head">
      <strong id="terminalTitle">Terminal</strong>
      <span id="terminalStatus" class="muted">closed</span>
      <span class="spacer"></span>
      <button id="terminalCloseBtn" class="secondary small" type="button">Close</button>
    </div>
    <pre id="terminalOutput" class="terminal-output" tabindex="0"></pre>
  </div>
  <script>
    let state = { agents: [], builds: [], runs: [] };
    let selectedRun = null;
    let selectedRuns = new Set();
    let selectedBuilds = new Set();
    let terminalSocket = null;
    let terminalBuffer = "";
    let terminalAgent = null;
    let activeTab = "build";

    const socketStatus = document.getElementById("socketStatus");
    const buildTab = document.getElementById("buildTab");
    const upgradeTab = document.getElementById("upgradeTab");
    const buildPanel = document.getElementById("buildPanel");
    const upgradePanel = document.getElementById("upgradePanel");
    const uploadForm = document.getElementById("uploadForm");
    const submitResult = document.getElementById("submitResult");
    const agentChecks = document.getElementById("agentChecks");
    const upgradeForm = document.getElementById("upgradeForm");
    const upgradeAgentChecks = document.getElementById("upgradeAgentChecks");
    const upgradeResult = document.getElementById("upgradeResult");
    const upgradeLog = document.getElementById("upgradeLog");
    const agentsBody = document.getElementById("agentsBody");
    const buildsBody = document.getElementById("buildsBody");
    const selectAllBuilds = document.getElementById("selectAllBuilds");
    const deleteBuildsBtn = document.getElementById("deleteBuildsBtn");
    const selectedBuildsEl = document.getElementById("selectedBuilds");
    const runsBody = document.getElementById("runsBody");
    const selectAllRuns = document.getElementById("selectAllRuns");
    const logEl = document.getElementById("log");
    const selectedRunEl = document.getElementById("selectedRun");
    const rerunBtn = document.getElementById("rerunBtn");
    const cancelBtn = document.getElementById("cancelBtn");
    const deleteBtn = document.getElementById("deleteBtn");
    const terminalPanel = document.getElementById("terminalPanel");
    const terminalTitle = document.getElementById("terminalTitle");
    const terminalStatus = document.getElementById("terminalStatus");
    const terminalOutput = document.getElementById("terminalOutput");
    const terminalCloseBtn = document.getElementById("terminalCloseBtn");

    function activateTab(tab) {
      activeTab = tab;
      const buildActive = tab === "build";
      buildTab.classList.toggle("active", buildActive);
      upgradeTab.classList.toggle("active", !buildActive);
      buildTab.setAttribute("aria-selected", buildActive ? "true" : "false");
      upgradeTab.setAttribute("aria-selected", buildActive ? "false" : "true");
      buildPanel.classList.toggle("active", buildActive);
      upgradePanel.classList.toggle("active", !buildActive);
    }

    buildTab.addEventListener("click", () => activateTab("build"));
    upgradeTab.addEventListener("click", () => activateTab("upgrade"));

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

      upgradeAgentChecks.innerHTML = state.agents.map(agent => {
        const enabled = agent.upgrade_enabled && agent.status !== "offline" && agent.running === 0;
        const detail = agent.upgrade_status
          ? `${agent.name} · ${display(agent.version)} · ${agent.upgrade_status}`
          : `${agent.name} · ${display(agent.version)}`;
        return `
        <label class="agent-item">
          <input type="checkbox" name="upgradeAgent" value="${escapeHtml(agent.name)}" ${enabled ? "" : "disabled"}>
          <span>
            <span>${escapeHtml(display(agent.computer_name))} ${status(agent.status)}</span>
            <span class="labels">${escapeHtml(detail)}</span>
          </span>
        </label>`;
      }).join("");

      agentsBody.innerHTML = state.agents.map(agent => `
        <tr>
          <td title="${escapeHtml(display(agent.computer_name))}">${escapeHtml(display(agent.computer_name))}</td>
          <td title="${escapeHtml(display(agent.username))}">${escapeHtml(display(agent.username))}</td>
          <td title="${escapeHtml(display(agent.ip))}">${escapeHtml(display(agent.ip))}</td>
          <td>${escapeHtml(formatOs(agent.platform))}</td>
          <td>${escapeHtml(formatArch(agent.arch))}</td>
          <td title="${escapeHtml(display(agent.version))}">${escapeHtml(display(agent.version))}</td>
          <td>${agent.running}/${agent.capacity}</td>
          <td title="${escapeHtml(display(agent.upgrade_status))}">${status(agent.status)}</td>
          <td><button class="secondary small" type="button" data-terminal="${escapeHtml(agent.name)}" ${agent.terminal_enabled && agent.status !== "offline" ? "" : "disabled"}>Open</button></td>
        </tr>`).join("");

      for (const button of agentsBody.querySelectorAll("button[data-terminal]")) {
        button.addEventListener("click", () => openTerminal(button.dataset.terminal));
      }

      const builds = [...state.builds].sort((a, b) => (b.created_at || 0) - (a.created_at || 0));
      const buildIds = new Set(builds.map(build => build.id));
      selectedBuilds = new Set([...selectedBuilds].filter(buildId => buildIds.has(buildId)));
      buildsBody.innerHTML = builds.map(build => `
        <tr class="selectable ${selectedBuilds.has(build.id) ? "selected" : ""}" data-build="${escapeHtml(build.id)}">
          <td class="build-select"><input type="checkbox" name="buildSelect" value="${escapeHtml(build.id)}" ${selectedBuilds.has(build.id) ? "checked" : ""}></td>
          <td title="${escapeHtml(build.source_name)}">${escapeHtml(build.source_name)}</td>
          <td title="${escapeHtml(formatTime(build.created_at))}">${escapeHtml(formatTime(build.created_at))}</td>
          <td>${status(build.status)}</td>
        </tr>`).join("");

      for (const checkbox of buildsBody.querySelectorAll("input[name='buildSelect']")) {
        checkbox.addEventListener("click", event => event.stopPropagation());
        checkbox.addEventListener("change", () => {
          if (checkbox.checked) {
            selectedBuilds.add(checkbox.value);
            clearRunSelection();
          } else {
            selectedBuilds.delete(checkbox.value);
          }
          render();
        });
      }

      for (const row of buildsBody.querySelectorAll("tr[data-build]")) {
        row.addEventListener("click", () => {
          selectedBuilds = new Set([row.dataset.build]);
          clearRunSelection();
          render();
        });
      }

      const runIds = new Set(state.runs.map(run => run.id));
      selectedRuns = new Set([...selectedRuns].filter(runId => runIds.has(runId)));
      if (selectedRun && !runIds.has(selectedRun)) {
        selectedRun = null;
        logEl.textContent = "";
      }

      runsBody.innerHTML = state.runs.map(run => `
        <tr class="selectable ${selectedRuns.has(run.id) ? "selected" : ""}" data-run="${escapeHtml(run.id)}">
          <td class="run-select"><input type="checkbox" name="runSelect" value="${escapeHtml(run.id)}" ${selectedRuns.has(run.id) ? "checked" : ""}></td>
          <td title="${escapeHtml(run.agent_name)}">${escapeHtml(agentComputerIp(run.agent_name))}</td>
          <td>${status(run.status)}</td>
        </tr>`).join("");

      for (const checkbox of runsBody.querySelectorAll("input[name='runSelect']")) {
        checkbox.addEventListener("click", event => event.stopPropagation());
        checkbox.addEventListener("change", () => {
          if (checkbox.checked) {
            selectedBuilds.clear();
            selectedRuns.add(checkbox.value);
            selectedRun = checkbox.value;
          } else {
            selectedRuns.delete(checkbox.value);
          }
          render();
        });
      }

      for (const row of runsBody.querySelectorAll("tr[data-run]")) {
        row.addEventListener("click", () => {
          selectedBuilds.clear();
          selectedRuns = new Set([row.dataset.run]);
          selectRun(row.dataset.run);
        });
      }

      const selectedBuildCount = selectedBuilds.size;
      selectedBuildsEl.textContent = selectedBuildCount > 0 ? `${selectedBuildCount} selected` : "";
      deleteBuildsBtn.disabled = selectedBuildCount === 0;
      selectAllBuilds.checked = builds.length > 0 && builds.every(build => selectedBuilds.has(build.id));
      selectAllBuilds.indeterminate = selectedBuildCount > 0 && !selectAllBuilds.checked;

      const run = state.runs.find(item => item.id === selectedRun);
      const selectedCount = selectedRuns.size;
      selectedRunEl.textContent = run
        ? `${run.id} on ${run.agent_name}${selectedCount > 1 ? ` · ${selectedCount} selected` : ""}`
        : (selectedCount > 0 ? `${selectedCount} selected` : "");
      rerunBtn.disabled = !run || !["success", "failed", "lost", "canceled"].includes(run.status);
      cancelBtn.disabled = !run || !["queued", "assigned", "preparing", "running"].includes(run.status);
      deleteBtn.disabled = selectedCount === 0;
      selectAllRuns.checked = state.runs.length > 0 && state.runs.every(run => selectedRuns.has(run.id));
      selectAllRuns.indeterminate = selectedCount > 0 && !selectAllRuns.checked;
    }

    function clearRunSelection() {
      selectedRun = null;
      selectedRuns.clear();
      logEl.textContent = "";
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

    function appendUpgradeLog(value) {
      upgradeLog.textContent += value;
      if (upgradeLog.textContent.length > 200000) {
        upgradeLog.textContent = upgradeLog.textContent.slice(-160000);
      }
      upgradeLog.scrollTop = upgradeLog.scrollHeight;
    }

    upgradeForm.addEventListener("submit", async event => {
      event.preventDefault();
      const form = new FormData();
      const file = document.getElementById("upgradePackage").files[0];
      if (!file) return;
      const selected = Array.from(document.querySelectorAll("input[name='upgradeAgent']:checked"))
        .map(input => input.value);
      if (selected.length === 0) {
        upgradeResult.textContent = "select at least one upgrade-enabled online agent";
        return;
      }
      form.append("package", file);
      form.append("package_kind", document.getElementById("packageKind").value);
      form.append("target_agents", selected.join(","));
      upgradeResult.textContent = "uploading";
      const response = await fetch("/api/upgrades", { method: "POST", body: form });
      if (!response.ok) {
        upgradeResult.textContent = await response.text();
        return;
      }
      const result = await response.json();
      state = result.state;
      const failed = result.failed.length > 0 ? ` · failed: ${result.failed.join("; ")}` : "";
      upgradeResult.textContent = `${result.upgrade_id} sent to ${result.sent.join(", ")}${failed}`;
      appendUpgradeLog(`[server] ${upgradeResult.textContent}\n`);
      upgradeForm.reset();
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

    async function deleteSelectedRuns() {
      const runIds = [...selectedRuns];
      if (runIds.length === 0) return;
      if (!confirm(`Delete ${runIds.length} selected run${runIds.length === 1 ? "" : "s"}?`)) return;

      deleteBtn.disabled = true;
      const failures = [];
      for (const runId of runIds) {
        const response = await fetch(`/api/runs/${encodeURIComponent(runId)}`, { method: "DELETE" });
        if (!response.ok) {
          failures.push(`${runId}: ${await response.text()}`);
          continue;
        }
        state = await response.json();
        selectedRuns.delete(runId);
      }
      render();
      if (failures.length > 0) {
        alert(failures.join("\n"));
      }
    }

    deleteBtn.addEventListener("click", deleteSelectedRuns);

    async function deleteSelectedBuilds() {
      const buildIds = [...selectedBuilds];
      if (buildIds.length === 0) return;
      if (!confirm(`Delete ${buildIds.length} selected source package${buildIds.length === 1 ? "" : "s"}?`)) return;

      deleteBuildsBtn.disabled = true;
      const failures = [];
      for (const buildId of buildIds) {
        const response = await fetch(`/api/builds/${encodeURIComponent(buildId)}`, { method: "DELETE" });
        if (!response.ok) {
          failures.push(`${buildId}: ${await response.text()}`);
          continue;
        }
        state = await response.json();
        selectedBuilds.delete(buildId);
      }
      render();
      if (failures.length > 0) {
        alert(failures.join("\n"));
      }
    }

    deleteBuildsBtn.addEventListener("click", deleteSelectedBuilds);

    selectAllBuilds.addEventListener("change", () => {
      if (selectAllBuilds.checked) {
        selectedBuilds = new Set(state.builds.map(build => build.id));
        clearRunSelection();
      } else {
        selectedBuilds.clear();
      }
      render();
    });

    selectAllRuns.addEventListener("change", () => {
      if (selectAllRuns.checked) {
        selectedBuilds.clear();
        selectedRuns = new Set(state.runs.map(run => run.id));
      } else {
        selectedRuns.clear();
      }
      render();
    });

    function encodeBase64Text(value) {
      const bytes = new TextEncoder().encode(value);
      let binary = "";
      for (const byte of bytes) binary += String.fromCharCode(byte);
      return btoa(binary);
    }

    function decodeBase64Text(value) {
      const binary = atob(value);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
      return new TextDecoder().decode(bytes);
    }

    function terminalSize() {
      const width = terminalOutput.clientWidth || 960;
      const height = terminalOutput.clientHeight || 480;
      return {
        cols: Math.max(40, Math.min(300, Math.floor(width / 8))),
        rows: Math.max(10, Math.min(200, Math.floor(height / 19)))
      };
    }

    function sendTerminalResize() {
      if (!terminalSocket || terminalSocket.readyState !== WebSocket.OPEN) return;
      terminalSocket.send(JSON.stringify({ type: "resize", ...terminalSize() }));
    }

    function sendTerminalInput(value) {
      if (!terminalSocket || terminalSocket.readyState !== WebSocket.OPEN || value === "") return;
      terminalSocket.send(JSON.stringify({ type: "input", data: encodeBase64Text(value) }));
    }

    function cleanTerminalText(value) {
      if (value.includes("\x1b[2J")) {
        terminalBuffer = "";
      }
      return value
        .replace(/\x1b\][^\x07]*(\x07|\x1b\\)/g, "")
        .replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, "")
        .replace(/\r\n/g, "\n")
        .replace(/\r/g, "\n");
    }

    function appendTerminalText(value) {
      for (const ch of cleanTerminalText(value)) {
        if (ch === "\b" || ch === "\x7f") {
          terminalBuffer = terminalBuffer.slice(0, -1);
        } else {
          terminalBuffer += ch;
        }
      }
      if (terminalBuffer.length > 200000) {
        terminalBuffer = terminalBuffer.slice(-160000);
      }
      terminalOutput.textContent = terminalBuffer;
      terminalOutput.scrollTop = terminalOutput.scrollHeight;
    }

    function keyToTerminalInput(event) {
      if (event.ctrlKey && !event.altKey && !event.metaKey && event.key.length === 1) {
        const key = event.key.toLowerCase();
        if (key === "v") return null;
        return String.fromCharCode(key.toUpperCase().charCodeAt(0) - 64);
      }
      if (event.key === "Enter") return "\r";
      if (event.key === "Backspace") return "\x7f";
      if (event.key === "Tab") return "\t";
      if (event.key === "Escape") return "\x1b";
      if (event.key === "ArrowUp") return "\x1b[A";
      if (event.key === "ArrowDown") return "\x1b[B";
      if (event.key === "ArrowRight") return "\x1b[C";
      if (event.key === "ArrowLeft") return "\x1b[D";
      if (event.key === "Delete") return "\x1b[3~";
      if (event.key === "Home") return "\x1b[H";
      if (event.key === "End") return "\x1b[F";
      if (!event.ctrlKey && !event.altKey && !event.metaKey && event.key.length === 1) return event.key;
      return null;
    }

    function openTerminal(agentName) {
      if (terminalSocket) {
        closeTerminal();
      }
      terminalAgent = agentName;
      terminalBuffer = "";
      terminalOutput.textContent = "";
      terminalTitle.textContent = `Terminal · ${agentName}`;
      terminalStatus.textContent = "connecting";
      terminalPanel.classList.remove("hidden");
      terminalOutput.focus();

      terminalSocket = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/api/agents/${encodeURIComponent(agentName)}/terminal/ws`);
      terminalSocket.onopen = () => {
        terminalStatus.textContent = "connected";
        sendTerminalResize();
      };
      terminalSocket.onclose = () => {
        terminalStatus.textContent = "closed";
        terminalSocket = null;
      };
      terminalSocket.onmessage = event => {
        const message = JSON.parse(event.data);
        if (message.type === "open") {
          terminalStatus.textContent = "open";
        } else if (message.type === "output") {
          appendTerminalText(decodeBase64Text(message.data));
        } else if (message.type === "exit") {
          terminalStatus.textContent = message.exit_code === null || message.exit_code === undefined
            ? "closed"
            : `exit ${message.exit_code}`;
          if (message.message) appendTerminalText(`\n${message.message}\n`);
        } else if (message.type === "error") {
          terminalStatus.textContent = "error";
          appendTerminalText(`\n${message.message}\n`);
        }
      };
    }

    function closeTerminal() {
      if (terminalSocket && terminalSocket.readyState === WebSocket.OPEN) {
        terminalSocket.send(JSON.stringify({ type: "close" }));
        terminalSocket.close();
      } else if (terminalSocket) {
        terminalSocket.close();
      }
      terminalSocket = null;
      terminalAgent = null;
      terminalPanel.classList.add("hidden");
    }

    terminalCloseBtn.addEventListener("click", closeTerminal);
    terminalOutput.addEventListener("keydown", event => {
      const data = keyToTerminalInput(event);
      if (data === null) return;
      event.preventDefault();
      event.stopPropagation();
      sendTerminalInput(data);
    });
    terminalOutput.addEventListener("paste", event => {
      const text = event.clipboardData?.getData("text");
      if (!text) return;
      event.preventDefault();
      sendTerminalInput(text);
    });
    window.addEventListener("resize", sendTerminalResize);

    document.addEventListener("keydown", event => {
      const target = event.target;
      const tag = target && target.tagName ? target.tagName.toLowerCase() : "";
      const inputType = tag === "input" ? (target.getAttribute("type") || "text").toLowerCase() : "";
      const editing = tag === "textarea"
        || tag === "select"
        || target?.isContentEditable
        || (tag === "input" && !["checkbox", "radio", "button", "submit"].includes(inputType));
      if (event.key !== "Delete" || editing) {
        return;
      }
      if (activeTab !== "build") return;
      if (selectedBuilds.size === 0 && selectedRuns.size === 0) return;
      event.preventDefault();
      if (selectedBuilds.size > 0) {
        deleteSelectedBuilds();
      } else {
        deleteSelectedRuns();
      }
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
        } else if (message.type === "upgrade_log") {
          appendUpgradeLog(message.data);
        }
      };
    }

    connect();
  </script>
</body>
</html>
"#;
