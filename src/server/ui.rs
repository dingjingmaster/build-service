pub fn index_html() -> String {
    INDEX_HTML.replace("__BUILDSVC_VERSION__", env!("CARGO_PKG_VERSION"))
}

const INDEX_HTML: &str = r#"<!doctype html>
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
    .server-version {
      margin-left: 6px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 600;
      vertical-align: baseline;
    }
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
      grid-template-columns: minmax(320px, 440px) minmax(0, 1fr);
      gap: 18px;
      align-items: stretch;
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
    .section-count {
      margin-left: 4px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 600;
    }
    .body { padding: 14px; }
    .stack { display: grid; gap: 12px; align-content: start; }
    .field-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
      margin-bottom: 5px;
    }
    .field-head label { margin-bottom: 0; }
    .workspace {
      display: grid;
      gap: 12px;
      grid-template-rows: auto minmax(420px, 1fr);
      min-height: calc(100vh - 92px);
    }
    .two {
      display: grid;
      grid-template-columns: minmax(0, 0.8fr) minmax(540px, 1.4fr);
      gap: 18px;
      align-items: stretch;
    }
    .two > section {
      display: flex;
      flex-direction: column;
    }
    .two > section > .body {
      flex: 1;
    }
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
    button.sort-button {
      min-height: 0;
      margin-left: 4px;
      border: 0;
      border-radius: 3px;
      padding: 1px 4px;
      background: transparent;
      color: var(--muted);
      font-size: 11px;
      line-height: 1;
      vertical-align: middle;
    }
    button.sort-button.active {
      background: #e8f1fc;
      color: var(--accent);
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
      overflow: auto;
      border: 1px solid var(--line);
      border-radius: 5px;
    }
    .agents-table-scroll {
      max-height: 268px;
      overflow-y: auto;
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
      overflow-y: auto;
      overflow-x: hidden;
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
    .agent-item > span { min-width: 0; }
    .item-detail { color: var(--muted); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    #log {
      flex: 1;
      min-height: 420px;
      max-height: 900px;
      height: auto;
      margin: 0;
      padding: 12px;
      overflow-y: auto;
      overflow-x: hidden;
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
    .agent-computer { width: 24%; }
    .agent-ip { width: 15%; }
    .agent-os { width: 8%; }
    .agent-arch { width: 8%; }
    .agent-version { width: 12%; }
    .agent-running { width: 8%; }
    .agent-status { width: 10%; }
    .agent-terminal { width: 13%; }
    .upgrade-log {
      flex: 1;
      min-height: 520px;
      max-height: 900px;
      height: auto;
      margin: 0;
      padding: 10px;
      overflow-y: auto;
      overflow-x: hidden;
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
      white-space: pre;
      overflow-wrap: normal;
      tab-size: 8;
    }
    .terminal-output .ansi-bold { font-weight: 700; }
    .terminal-output .ansi-fg-black { color: #6b7280; }
    .terminal-output .ansi-fg-red { color: #ef4444; }
    .terminal-output .ansi-fg-green { color: #22c55e; }
    .terminal-output .ansi-fg-yellow { color: #eab308; }
    .terminal-output .ansi-fg-blue { color: #60a5fa; }
    .terminal-output .ansi-fg-magenta { color: #e879f9; }
    .terminal-output .ansi-fg-cyan { color: #22d3ee; }
    .terminal-output .ansi-fg-white { color: #e5e7eb; }
    .terminal-output .ansi-bg-black { background: #111827; }
    .terminal-output .ansi-bg-red { background: #7f1d1d; }
    .terminal-output .ansi-bg-green { background: #14532d; }
    .terminal-output .ansi-bg-yellow { background: #713f12; }
    .terminal-output .ansi-bg-blue { background: #1e3a8a; }
    .terminal-output .ansi-bg-magenta { background: #701a75; }
    .terminal-output .ansi-bg-cyan { background: #164e63; }
    .terminal-output .ansi-bg-white { background: #f3f4f6; color: #111827; }
    .terminal-cursor {
      background: #d1e7d8;
      color: #050a12;
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
    <h1>buildsvc <span class="server-version">v__BUILDSVC_VERSION__</span></h1>
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
                <div class="field-head">
                  <label>Agents</label>
                  <button id="selectAllSubmitAgents" class="secondary small" type="button">Select All</button>
                </div>
                <div id="agentChecks" class="agent-list"></div>
              </div>
              <button type="submit">Start</button>
              <div id="submitResult" class="muted"></div>
            </form>
          </section>
          <section>
            <h2>Runs <span id="runsCount" class="section-count">(0/0)</span></h2>
            <div class="body">
              <div class="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th class="run-select"><input id="selectAllRuns" type="checkbox"></th>
                      <th class="run-agent">Computer / IP</th>
                      <th class="run-status">Status <button class="sort-button active" type="button" data-run-sort="status" title="Sort Status">↑</button></th>
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
              <h2>Agents <span id="agentsCount" class="section-count">(0/0)</span></h2>
              <div class="body">
                <div class="table-scroll agents-table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th class="agent-computer">Computer <button class="sort-button active" type="button" data-agent-sort="computer_name" title="Sort Computer">↑</button></th>
                        <th class="agent-ip">IP <button class="sort-button" type="button" data-agent-sort="ip" title="Sort IP">↕</button></th>
                        <th class="agent-os">OS <button class="sort-button" type="button" data-agent-sort="platform" title="Sort OS">↕</button></th>
                        <th class="agent-arch">Arch <button class="sort-button" type="button" data-agent-sort="arch" title="Sort Arch">↕</button></th>
                        <th class="agent-version">Version <button class="sort-button" type="button" data-agent-sort="version" title="Sort Version">↕</button></th>
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
              <div class="field-head">
                <label>Agents</label>
                <button id="selectAllUpgradeAgents" class="secondary small" type="button">Select All</button>
              </div>
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
    let selectedSubmitAgents = new Set();
    let selectedUpgradeAgents = new Set();
    let terminalSocket = null;
    let terminalAgent = null;
    let terminalLines = [[]];
    let terminalFloors = [0];
    let terminalRow = 0;
    let terminalCol = 0;
    let terminalStyle = "";
    let terminalPending = "";
    let terminalSuppressPromptErase = false;
    let terminalSuppressTabEchoUntil = 0;
    let terminalCols = 120;
    let activeTab = "build";
    let agentSort = { key: "computer_name", direction: "asc" };
    let runSort = { key: "status", direction: "asc" };
    let upgradeLogBuffers = new Map();

    const socketStatus = document.getElementById("socketStatus");
    const buildTab = document.getElementById("buildTab");
    const upgradeTab = document.getElementById("upgradeTab");
    const buildPanel = document.getElementById("buildPanel");
    const upgradePanel = document.getElementById("upgradePanel");
    const uploadForm = document.getElementById("uploadForm");
    const submitResult = document.getElementById("submitResult");
    const selectAllSubmitAgents = document.getElementById("selectAllSubmitAgents");
    const agentChecks = document.getElementById("agentChecks");
    const upgradeForm = document.getElementById("upgradeForm");
    const selectAllUpgradeAgents = document.getElementById("selectAllUpgradeAgents");
    const upgradeAgentChecks = document.getElementById("upgradeAgentChecks");
    const upgradeResult = document.getElementById("upgradeResult");
    const upgradeLog = document.getElementById("upgradeLog");
    const agentsCount = document.getElementById("agentsCount");
    const agentsBody = document.getElementById("agentsBody");
    const agentSortButtons = Array.from(document.querySelectorAll("button[data-agent-sort]"));
    const buildsBody = document.getElementById("buildsBody");
    const selectAllBuilds = document.getElementById("selectAllBuilds");
    const deleteBuildsBtn = document.getElementById("deleteBuildsBtn");
    const selectedBuildsEl = document.getElementById("selectedBuilds");
    const runsBody = document.getElementById("runsBody");
    const runsCount = document.getElementById("runsCount");
    const selectAllRuns = document.getElementById("selectAllRuns");
    const runSortButtons = Array.from(document.querySelectorAll("button[data-run-sort]"));
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

    const textCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

    function ipv4Parts(value) {
      const parts = String(value || "").trim().split(".");
      if (parts.length !== 4) return null;
      const numbers = parts.map(part => /^\d+$/.test(part) ? Number(part) : NaN);
      return numbers.every(number => Number.isInteger(number) && number >= 0 && number <= 255)
        ? numbers
        : null;
    }

    function compareNumberArrays(left, right) {
      for (let index = 0; index < Math.max(left.length, right.length); index += 1) {
        const diff = (left[index] ?? 0) - (right[index] ?? 0);
        if (diff !== 0) return diff;
      }
      return 0;
    }

    function agentSortText(agent, key) {
      if (key === "computer_name") return display(agent.computer_name);
      if (key === "ip") return display(agent.ip);
      if (key === "platform") return formatOs(agent.platform);
      if (key === "arch") return formatArch(agent.arch);
      if (key === "version") return display(agent.version);
      return "";
    }

    function compareAgents(left, right) {
      let result = 0;
      if (agentSort.key === "ip") {
        const leftIp = ipv4Parts(left.ip);
        const rightIp = ipv4Parts(right.ip);
        if (leftIp && rightIp) result = compareNumberArrays(leftIp, rightIp);
      }
      if (result === 0) {
        result = textCollator.compare(
          agentSortText(left, agentSort.key),
          agentSortText(right, agentSort.key)
        );
      }
      if (result === 0) {
        result = textCollator.compare(display(left.computer_name), display(right.computer_name));
      }
      return agentSort.direction === "asc" ? result : -result;
    }

    function renderAgentSortButtons() {
      for (const button of agentSortButtons) {
        const active = button.dataset.agentSort === agentSort.key;
        button.classList.toggle("active", active);
        button.textContent = active
          ? (agentSort.direction === "asc" ? "↑" : "↓")
          : "↕";
        button.title = active
          ? `Sort ${button.dataset.agentSort} ${agentSort.direction === "asc" ? "ascending" : "descending"}`
          : `Sort ${button.dataset.agentSort}`;
      }
    }

    const runStatusOrder = new Map([
      ["queued", 0],
      ["assigned", 1],
      ["preparing", 2],
      ["running", 3],
      ["success", 4],
      ["failed", 5],
      ["canceled", 6],
      ["lost", 7]
    ]);

    function compareRuns(left, right) {
      const leftRank = runStatusOrder.has(left.status) ? runStatusOrder.get(left.status) : 99;
      const rightRank = runStatusOrder.has(right.status) ? runStatusOrder.get(right.status) : 99;
      let result = leftRank - rightRank;
      if (result === 0) {
        result = textCollator.compare(display(left.status), display(right.status));
      }
      if (result === 0) {
        result = textCollator.compare(agentComputerIp(left.agent_id), agentComputerIp(right.agent_id));
      }
      return runSort.direction === "asc" ? result : -result;
    }

    function renderRunSortButtons() {
      for (const button of runSortButtons) {
        const active = button.dataset.runSort === runSort.key;
        button.classList.toggle("active", active);
        button.textContent = active
          ? (runSort.direction === "asc" ? "↑" : "↓")
          : "↕";
        button.title = `Sort Status ${runSort.direction === "asc" ? "ascending" : "descending"}`;
      }
    }

    function agentComputerIp(agentId) {
      const agent = state.agents.find(item => item.id === agentId);
      if (!agent) return agentId;
      const computer = display(agent.computer_name);
      const ip = display(agent.ip);
      return ip === "-" ? computer : `${computer} / ${ip}`;
    }

    function compactJoin(values) {
      const visible = values.map(display).filter(value => value !== "-");
      return visible.length > 0 ? visible.join(" · ") : "-";
    }

    function agentDetail(agent) {
      return compactJoin([
        agent.ip,
        formatOs(agent.platform),
        formatArch(agent.arch),
        agent.version
      ]);
    }

    function isActiveRunStatus(value) {
      return ["queued", "assigned", "preparing", "running"].includes(value);
    }

    function isUpgradeAgentEnabled(agent) {
      return agent.upgrade_enabled && agent.status !== "offline" && agent.running === 0;
    }

    function render() {
      const onlineAgents = state.agents.filter(agent => agent.status !== "offline").length;
      const activeRuns = state.runs.filter(run => isActiveRunStatus(run.status)).length;
      const submitAgentIds = new Set(state.agents.map(agent => agent.id));
      const upgradeAgentIds = new Set(state.agents.filter(isUpgradeAgentEnabled).map(agent => agent.id));
      selectedSubmitAgents = new Set([...selectedSubmitAgents].filter(agentId => submitAgentIds.has(agentId)));
      selectedUpgradeAgents = new Set([...selectedUpgradeAgents].filter(agentId => upgradeAgentIds.has(agentId)));
      agentsCount.textContent = `(${onlineAgents}/${state.agents.length})`;
      runsCount.textContent = `(${activeRuns}/${state.runs.length})`;
      selectAllSubmitAgents.disabled = state.agents.length === 0;
      selectAllSubmitAgents.textContent = state.agents.length > 0 && state.agents.every(agent => selectedSubmitAgents.has(agent.id))
        ? "Clear All"
        : "Select All";
      selectAllUpgradeAgents.disabled = upgradeAgentIds.size === 0;
      selectAllUpgradeAgents.textContent = upgradeAgentIds.size > 0 && [...upgradeAgentIds].every(agentId => selectedUpgradeAgents.has(agentId))
        ? "Clear All"
        : "Select All";
      renderAgentSortButtons();
      renderRunSortButtons();

      agentChecks.innerHTML = state.agents.map(agent => `
        <label class="agent-item">
          <input type="checkbox" name="agent" value="${escapeHtml(agent.id)}" ${selectedSubmitAgents.has(agent.id) ? "checked" : ""}>
          <span>
            <span>${escapeHtml(display(agent.computer_name))} ${status(agent.status)}</span>
            <span class="item-detail">${escapeHtml(agentDetail(agent))}</span>
          </span>
        </label>`).join("");

      for (const checkbox of agentChecks.querySelectorAll("input[name='agent']")) {
        checkbox.addEventListener("change", () => {
          if (checkbox.checked) {
            selectedSubmitAgents.add(checkbox.value);
          } else {
            selectedSubmitAgents.delete(checkbox.value);
          }
          render();
        });
      }

      upgradeAgentChecks.innerHTML = state.agents.map(agent => {
        const enabled = isUpgradeAgentEnabled(agent);
        const detail = agent.upgrade_status
          ? `${display(agent.version)} · ${agent.upgrade_status}`
          : `${display(agent.version)}`;
        return `
        <label class="agent-item">
          <input type="checkbox" name="upgradeAgent" value="${escapeHtml(agent.id)}" ${selectedUpgradeAgents.has(agent.id) ? "checked" : ""} ${enabled ? "" : "disabled"}>
          <span>
            <span>${escapeHtml(display(agent.computer_name))} ${status(agent.status)}</span>
            <span class="item-detail">${escapeHtml(detail)}</span>
          </span>
        </label>`;
      }).join("");

      for (const checkbox of upgradeAgentChecks.querySelectorAll("input[name='upgradeAgent']")) {
        checkbox.addEventListener("change", () => {
          if (checkbox.checked) {
            selectedUpgradeAgents.add(checkbox.value);
          } else {
            selectedUpgradeAgents.delete(checkbox.value);
          }
          render();
        });
      }

      const sortedAgents = [...state.agents].sort(compareAgents);
      agentsBody.innerHTML = sortedAgents.map(agent => `
        <tr>
          <td title="${escapeHtml(display(agent.computer_name))}">${escapeHtml(display(agent.computer_name))}</td>
          <td title="${escapeHtml(display(agent.ip))}">${escapeHtml(display(agent.ip))}</td>
          <td>${escapeHtml(formatOs(agent.platform))}</td>
          <td>${escapeHtml(formatArch(agent.arch))}</td>
          <td title="${escapeHtml(display(agent.version))}">${escapeHtml(display(agent.version))}</td>
          <td>${agent.running}/${agent.capacity}</td>
          <td title="${escapeHtml(display(agent.upgrade_status))}">${status(agent.status)}</td>
          <td><button class="secondary small" type="button" data-terminal="${escapeHtml(agent.id)}" ${agent.terminal_enabled && agent.status !== "offline" ? "" : "disabled"}>Open</button></td>
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

      const sortedRuns = [...state.runs].sort(compareRuns);
      runsBody.innerHTML = sortedRuns.map(run => `
        <tr class="selectable ${selectedRuns.has(run.id) ? "selected" : ""}" data-run="${escapeHtml(run.id)}">
          <td class="run-select"><input type="checkbox" name="runSelect" value="${escapeHtml(run.id)}" ${selectedRuns.has(run.id) ? "checked" : ""}></td>
          <td title="${escapeHtml(agentComputerIp(run.agent_id))}">${escapeHtml(agentComputerIp(run.agent_id))}</td>
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
        ? `${run.id} on ${agentComputerIp(run.agent_id)}${selectedCount > 1 ? ` · ${selectedCount} selected` : ""}`
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
      const selected = [...selectedSubmitAgents];
      form.append("source", file);
      form.append("target_agents", selected.join(","));
      submitResult.textContent = "uploading";
      const response = await fetch("/api/builds", { method: "POST", body: form });
      if (!response.ok) {
        submitResult.textContent = await response.text();
        return;
      }
      state = await response.json();
      submitResult.textContent = "submitted";
      selectedSubmitAgents.clear();
      uploadForm.reset();
      render();
    });

    selectAllSubmitAgents.addEventListener("click", () => {
      const agentIds = state.agents.map(agent => agent.id);
      const allSelected = agentIds.length > 0 && agentIds.every(agentId => selectedSubmitAgents.has(agentId));
      selectedSubmitAgents = allSelected ? new Set() : new Set(agentIds);
      render();
    });

    for (const button of agentSortButtons) {
      button.addEventListener("click", () => {
        const key = button.dataset.agentSort;
        if (agentSort.key === key) {
          agentSort.direction = agentSort.direction === "asc" ? "desc" : "asc";
        } else {
          agentSort = { key, direction: "asc" };
        }
        render();
      });
    }

    for (const button of runSortButtons) {
      button.addEventListener("click", () => {
        const key = button.dataset.runSort;
        if (runSort.key === key) {
          runSort.direction = runSort.direction === "asc" ? "desc" : "asc";
        } else {
          runSort = { key, direction: "asc" };
        }
        render();
      });
    }

    function appendUpgradeLogText(value) {
      upgradeLog.textContent += value;
      if (upgradeLog.textContent.length > 200000) {
        upgradeLog.textContent = upgradeLog.textContent.slice(-160000);
      }
      upgradeLog.scrollTop = upgradeLog.scrollHeight;
    }

    function upgradeLogPrefix(message) {
      return message.stream === "stderr"
        ? `[${message.agent_id} stderr] `
        : `[${message.agent_id}] `;
    }

    function flushUpgradeLogBuffers(filter = {}) {
      for (const [key, entry] of upgradeLogBuffers) {
        if (filter.agent_id && entry.agent_id !== filter.agent_id) continue;
        if (filter.upgrade_id && entry.upgrade_id !== filter.upgrade_id) continue;
        if (entry.text !== "") appendUpgradeLogText(`${entry.prefix}${entry.text}\n`);
        upgradeLogBuffers.delete(key);
      }
    }

    function appendUpgradeLog(message) {
      if (typeof message === "string") {
        appendUpgradeLogText(message);
        return;
      }
      if (!message.stream) {
        flushUpgradeLogBuffers({ agent_id: message.agent_id, upgrade_id: message.upgrade_id });
        appendUpgradeLogText(String(message.data || "").endsWith("\n")
          ? String(message.data || "")
          : `${String(message.data || "")}\n`);
        return;
      }

      const key = `${message.upgrade_id}\u0001${message.agent_id}\u0001${message.stream}`;
      const entry = upgradeLogBuffers.get(key) || {
        agent_id: message.agent_id,
        upgrade_id: message.upgrade_id,
        prefix: upgradeLogPrefix(message),
        text: ""
      };
      entry.text += String(message.data || "").replace(/\r\n/g, "\n").replace(/\r/g, "\n");
      const lines = entry.text.split("\n");
      entry.text = lines.pop() || "";
      for (const line of lines) {
        appendUpgradeLogText(`${entry.prefix}${line}\n`);
      }
      if (entry.text === "") {
        upgradeLogBuffers.delete(key);
      } else {
        upgradeLogBuffers.set(key, entry);
      }
    }

    upgradeForm.addEventListener("submit", async event => {
      event.preventDefault();
      const form = new FormData();
      const file = document.getElementById("upgradePackage").files[0];
      if (!file) return;
      const selected = [...selectedUpgradeAgents];
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
      selectedUpgradeAgents.clear();
      upgradeForm.reset();
      render();
    });

    selectAllUpgradeAgents.addEventListener("click", () => {
      const agentIds = state.agents.filter(isUpgradeAgentEnabled).map(agent => agent.id);
      const allSelected = agentIds.length > 0 && agentIds.every(agentId => selectedUpgradeAgents.has(agentId));
      selectedUpgradeAgents = allSelected ? new Set() : new Set(agentIds);
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
      const size = terminalSize();
      terminalCols = size.cols;
      terminalSocket.send(JSON.stringify({ type: "resize", ...size }));
    }

    function sendTerminalInput(value) {
      if (!terminalSocket || terminalSocket.readyState !== WebSocket.OPEN || value === "") return;
      terminalSocket.send(JSON.stringify({ type: "input", data: encodeBase64Text(value) }));
    }

    function resetTerminalBuffer() {
      terminalLines = [[]];
      terminalFloors = [0];
      terminalRow = 0;
      terminalCol = 0;
      terminalStyle = "";
      terminalPending = "";
      terminalSuppressPromptErase = false;
      terminalSuppressTabEchoUntil = 0;
      terminalCols = terminalSize().cols;
      terminalOutput.innerHTML = "";
    }

    function ensureTerminalLine(row = terminalRow) {
      while (terminalLines.length <= row) {
        terminalLines.push([]);
        terminalFloors.push(0);
      }
    }

    function trimTerminalScrollback() {
      const maxLines = 2000;
      if (terminalLines.length <= maxLines) return;
      const extra = terminalLines.length - maxLines;
      terminalLines.splice(0, extra);
      terminalFloors.splice(0, extra);
      terminalRow = Math.max(0, terminalRow - extra);
    }

    function terminalLineText(row = terminalRow) {
      ensureTerminalLine(row);
      return terminalLines[row].map(cell => cell ? cell.ch : " ").join("");
    }

    function updateTerminalPromptFloor() {
      const text = terminalLineText();
      if (/[$#>] $/.test(text)) {
        terminalFloors[terminalRow] = Math.max(terminalFloors[terminalRow] || 0, terminalCol);
        if (terminalSocket && terminalSocket.readyState === WebSocket.OPEN) {
          terminalStatus.textContent = "ready";
        }
      }
    }

    function terminalStyleClass() {
      return terminalStyle;
    }

    function sgrColorClass(offset, bright) {
      const colors = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
      const name = colors[offset] || "white";
      return bright ? `ansi-fg-${name} ansi-bold` : `ansi-fg-${name}`;
    }

    function sgrBgClass(offset) {
      const colors = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
      return `ansi-bg-${colors[offset] || "black"}`;
    }

    function applyTerminalSgr(params) {
      const values = params.length === 0 ? [0] : params.map(value => value === "" ? 0 : Number(value));
      let bold = terminalStyle.includes("ansi-bold");
      let fg = (terminalStyle.match(/ansi-fg-\w+/) || [""])[0];
      let bg = (terminalStyle.match(/ansi-bg-\w+/) || [""])[0];

      for (let index = 0; index < values.length; index += 1) {
        const value = values[index];
        if (value === 0) {
          bold = false; fg = ""; bg = "";
        } else if (value === 1) {
          bold = true;
        } else if (value === 22) {
          bold = false;
        } else if (value === 39) {
          fg = "";
        } else if (value === 49) {
          bg = "";
        } else if (value >= 30 && value <= 37) {
          fg = `ansi-fg-${["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"][value - 30]}`;
        } else if (value >= 90 && value <= 97) {
          fg = `ansi-fg-${["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"][value - 90]}`;
          bold = true;
        } else if (value >= 40 && value <= 47) {
          bg = sgrBgClass(value - 40);
        } else if (value === 38 && values[index + 1] === 5) {
          const color = values[index + 2];
          if (color >= 0 && color <= 7) fg = sgrColorClass(color, false);
          if (color >= 8 && color <= 15) fg = sgrColorClass(color - 8, true);
          index += 2;
        } else if (value === 48 && values[index + 1] === 5) {
          const color = values[index + 2];
          if (color >= 0 && color <= 7) bg = sgrBgClass(color);
          if (color >= 8 && color <= 15) bg = sgrBgClass(color - 8);
          index += 2;
        }
      }

      terminalStyle = [bold ? "ansi-bold" : "", fg, bg].filter(Boolean).join(" ");
    }

    function clearTerminalScreen() {
      terminalLines = [[]];
      terminalFloors = [0];
      terminalRow = 0;
      terminalCol = 0;
    }

    function handleTerminalCsi(rawParams, final) {
      const cleanParams = rawParams.replace(/^[?=>]/, "");
      const params = cleanParams === "" ? [] : cleanParams.split(";");
      const nums = params.map(value => value === "" ? 0 : Number(value));
      ensureTerminalLine();

      if (final === "m") {
        applyTerminalSgr(params);
      } else if (final === "A") {
        terminalRow = Math.max(0, terminalRow - (nums[0] || 1));
      } else if (final === "B") {
        terminalRow += nums[0] || 1;
        ensureTerminalLine();
      } else if (final === "C") {
        terminalCol = Math.min(terminalCols - 1, terminalCol + (nums[0] || 1));
      } else if (final === "D") {
        terminalCol = Math.max(terminalFloors[terminalRow] || 0, terminalCol - (nums[0] || 1));
      } else if (final === "G") {
        terminalCol = Math.max(0, Math.min(terminalCols - 1, (nums[0] || 1) - 1));
      } else if (final === "H" || final === "f") {
        terminalRow = Math.max(0, (nums[0] || 1) - 1);
        terminalCol = Math.max(0, Math.min(terminalCols - 1, (nums[1] || 1) - 1));
        ensureTerminalLine();
      } else if (final === "J") {
        if ((nums[0] || 0) === 2 || (nums[0] || 0) === 3) clearTerminalScreen();
      } else if (final === "K") {
        const mode = nums[0] || 0;
        const line = terminalLines[terminalRow];
        if (mode === 0) {
          line.length = terminalCol;
        } else if (mode === 1) {
          for (let col = 0; col <= terminalCol; col += 1) line[col] = { ch: " ", style: terminalStyleClass() };
        } else if (mode === 2) {
          terminalLines[terminalRow] = [];
          terminalFloors[terminalRow] = 0;
          terminalCol = 0;
        }
      }
    }

    function terminalNewLine() {
      terminalRow += 1;
      terminalCol = 0;
      ensureTerminalLine();
      trimTerminalScrollback();
    }

    function writeTerminalChar(ch) {
      ensureTerminalLine();
      if (ch === "\n") {
        terminalNewLine();
        return;
      }
      if (ch === "\r") {
        terminalCol = 0;
        return;
      }
      if (ch === "\b") {
        const floor = terminalFloors[terminalRow] || 0;
        if (terminalCol > floor) {
          terminalCol -= 1;
        } else {
          terminalSuppressPromptErase = true;
        }
        return;
      }
      if (ch === "\x7f") return;
      if (ch === "\t") {
        if (Date.now() <= terminalSuppressTabEchoUntil) {
          terminalSuppressTabEchoUntil = 0;
          return;
        }
        const spaces = 8 - (terminalCol % 8);
        for (let index = 0; index < spaces; index += 1) writeTerminalChar(" ");
        return;
      }
      if (ch < " ") return;
      const floor = terminalFloors[terminalRow] || 0;
      if (terminalSuppressPromptErase && ch === " " && terminalCol <= floor) {
        terminalSuppressPromptErase = false;
        return;
      }
      terminalSuppressPromptErase = false;
      if (terminalCol >= terminalCols) terminalNewLine();
      terminalLines[terminalRow][terminalCol] = { ch, style: terminalStyleClass() };
      terminalCol += 1;
      updateTerminalPromptFloor();
    }

    function renderTerminal() {
      const showCursor = terminalSocket && terminalSocket.readyState === WebSocket.OPEN;
      const html = terminalLines.map((line, rowIndex) => {
        let output = "";
        let currentStyle = null;
        let chunk = "";
        const flush = () => {
          if (chunk === "") return;
          const escaped = escapeHtml(chunk);
          output += currentStyle ? `<span class="${escapeHtml(currentStyle)}">${escaped}</span>` : escaped;
          chunk = "";
        };
        const width = showCursor && rowIndex === terminalRow
          ? Math.max(line.length, terminalCol + 1)
          : line.length;
        for (let col = 0; col < width; col += 1) {
          const cell = line[col];
          const isCursor = showCursor && rowIndex === terminalRow && col === terminalCol;
          const style = cell ? cell.style : "";
          if (isCursor) {
            flush();
            const classes = ["terminal-cursor", style].filter(Boolean).join(" ");
            output += `<span class="${escapeHtml(classes)}">${escapeHtml(cell ? cell.ch : " ")}</span>`;
            continue;
          }
          if (style !== currentStyle) {
            flush();
            currentStyle = style;
          }
          chunk += cell ? cell.ch : " ";
        }
        flush();
        return output;
      }).join("\n");
      terminalOutput.innerHTML = html;
      terminalOutput.scrollTop = terminalOutput.scrollHeight;
    }

    function appendTerminalText(value) {
      terminalPending += value;
      let index = 0;
      while (index < terminalPending.length) {
        const ch = terminalPending[index];
        if (ch === "\x1b") {
          if (terminalPending[index + 1] === "]") {
            const bell = terminalPending.indexOf("\x07", index + 2);
            const st = terminalPending.indexOf("\x1b\\", index + 2);
            const end = bell === -1 ? st : (st === -1 ? bell : Math.min(bell, st));
            if (end === -1) break;
            index = end + (terminalPending[end] === "\x1b" ? 2 : 1);
            continue;
          }
          if (terminalPending[index + 1] === "[") {
            let end = index + 2;
            while (end < terminalPending.length && !/[@-~]/.test(terminalPending[end])) end += 1;
            if (end >= terminalPending.length) break;
            handleTerminalCsi(terminalPending.slice(index + 2, end), terminalPending[end]);
            index = end + 1;
            continue;
          }
          if (index + 1 >= terminalPending.length) break;
          index += 2;
          continue;
        }
        writeTerminalChar(ch);
        index += 1;
      }
      terminalPending = terminalPending.slice(index);
      renderTerminal();
    }

    function keyToTerminalInput(event) {
      if (event.ctrlKey && !event.altKey && !event.metaKey && event.key.length === 1) {
        const key = event.key.toLowerCase();
        if (key === "v") return null;
        return String.fromCharCode(key.toUpperCase().charCodeAt(0) - 64);
      }
      if (event.key === "Enter") {
        if (terminalSocket && terminalSocket.readyState === WebSocket.OPEN) {
          terminalStatus.textContent = "running";
        }
        return "\r";
      }
      if (event.key === "Backspace") return "\x7f";
      if (event.key === "Tab") {
        terminalSuppressTabEchoUntil = Date.now() + 500;
        return "\t";
      }
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

    function openTerminal(agentId) {
      if (terminalSocket) {
        closeTerminal();
      }
      terminalAgent = agentId;
      resetTerminalBuffer();
      terminalTitle.textContent = `Terminal · ${agentComputerIp(agentId)}`;
      terminalStatus.textContent = "connecting";
      terminalPanel.classList.remove("hidden");
      terminalOutput.focus();

      terminalSocket = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/api/agents/${encodeURIComponent(agentId)}/terminal/ws`);
      terminalSocket.onopen = () => {
        terminalStatus.textContent = "connected";
        sendTerminalResize();
      };
      terminalSocket.onclose = () => {
        const showDisconnect = terminalAgent !== null && !terminalPanel.classList.contains("hidden");
        terminalStatus.textContent = "closed";
        terminalSocket = null;
        if (showDisconnect) {
          appendTerminalText("\n[terminal disconnected]\n");
        }
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
          closeTerminal(false);
        } else if (message.type === "error") {
          terminalStatus.textContent = "error";
          appendTerminalText(`\n${message.message}\n`);
        }
      };
    }

    function closeTerminal(notifyAgent = true) {
      if (terminalSocket && terminalSocket.readyState === WebSocket.OPEN) {
        if (notifyAgent) {
          terminalSocket.send(JSON.stringify({ type: "close" }));
        }
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
          appendUpgradeLog(message);
        }
      };
    }

    connect();
  </script>
</body>
</html>
"#;
