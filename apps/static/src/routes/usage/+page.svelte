<script lang="ts">
  import { base } from '$app/paths';

  const commands = [
    { cmd: 'sensei configure', desc: 'Detect installed AI coding platforms and generate config' },
    { cmd: 'sensei install', desc: 'Install sensei for all configured platforms (hooks, skills, MCP)' },
    { cmd: 'sensei install --acp claude-code', desc: 'Install for a specific platform only' },
    { cmd: 'sensei uninstall', desc: 'Remove sensei from all configured platforms' },
    { cmd: 'sensei start', desc: 'Start the senseid daemon (default port 7744)' },
    { cmd: 'sensei start --port 8800', desc: 'Start on a custom port' },
    { cmd: 'sensei stop', desc: 'Stop the running daemon' },
    { cmd: 'sensei status', desc: 'Show daemon status (version, PID, uptime)' },
    { cmd: 'sensei scan ~/Developer', desc: 'Scan a folder and index all repos found' },
    { cmd: 'sensei add-lib react', desc: 'Add a library\'s docs (auto-discovers llms.txt)' },
    { cmd: 'sensei add-lib kavach --url https://...', desc: 'Add a library with explicit doc URL' },
  ];

  const daemon = [
    { cmd: 'senseid', desc: 'Start daemon in foreground' },
    { cmd: 'senseid start', desc: 'Start daemon in background' },
    { cmd: 'senseid stop', desc: 'Stop the background daemon' },
    { cmd: 'senseid status', desc: 'Show daemon version and PID' },
    { cmd: 'senseid logs', desc: 'Tail the last 50 lines of the daemon log' },
    { cmd: 'senseid clear-logs', desc: 'Clear the log file' },
  ];

  const mcpTools = [
    { tool: 'get_session_context', desc: 'Resume from last checkpoint, load repo orientation' },
    { tool: 'context_pack', desc: 'Load only the symbols relevant to a query (saves tokens)' },
    { tool: 'search', desc: 'Semantic search across the entire codebase' },
    { tool: 'load_context', desc: 'Load a specific file with symbol extraction' },
    { tool: 'take_snapshot', desc: 'Record progress for interruption recovery' },
    { tool: 'checkpoint', desc: 'Close a session with outcome tracking (FTR scoring)' },
  ];
</script>

<div class="min-h-screen overflow-y-auto bg-surface-z1 text-surface-z8">

  <!-- Nav -->
  <nav class="sticky top-0 z-50 flex items-center justify-between border-b border-surface-z0 bg-surface-z1/95 px-8 h-14">
    <a href="{base}/" class="flex items-center gap-2 font-bold text-base select-none">
      <span class="text-lg">⬡</span>
      <span>sensei</span>
    </a>
    <div class="flex items-center gap-5 text-sm">
      <a href="{base}/setup" class="text-surface-z5 hover:text-surface-z8 transition-colors">Setup</a>
      <a href="{base}/usage" class="text-primary-z6 font-semibold">Usage</a>
      <a href="https://github.com/mizukisu/sensei" target="_blank" rel="noopener"
         class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-xs font-medium hover:bg-surface-z3 transition-colors">
        <span class="i-solar-github-bold-duotone"></span> GitHub
      </a>
    </div>
  </nav>

  <div class="mx-auto max-w-3xl px-8 py-16">

    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Reference</div>
    <h1 class="mb-4 text-4xl font-extrabold tracking-tight">Usage Guide</h1>
    <p class="mb-12 text-lg text-surface-z5">CLI commands, daemon management, and MCP tools.</p>

    <!-- CLI Commands -->
    <section class="mb-14">
      <h2 class="mb-5 text-2xl font-bold">CLI Commands</h2>
      <p class="mb-5 text-sm text-surface-z5">The <code class="bg-surface-z3 px-1 rounded">sensei</code> CLI manages installation, configuration, and scanning.</p>
      <div class="rounded-xl border border-surface-z3 overflow-hidden">
        {#each commands as c, i}
          <div class="flex flex-col gap-1 px-5 py-3 {i % 2 === 0 ? 'bg-surface-z2' : 'bg-surface-z1'}">
            <code class="text-xs font-semibold text-primary-z7">{c.cmd}</code>
            <span class="text-xs text-surface-z5">{c.desc}</span>
          </div>
        {/each}
      </div>
    </section>

    <!-- Daemon -->
    <section class="mb-14">
      <h2 class="mb-5 text-2xl font-bold">Daemon Management</h2>
      <p class="mb-5 text-sm text-surface-z5">The <code class="bg-surface-z3 px-1 rounded">senseid</code> daemon runs the indexer and serves the API on port 7744.</p>
      <div class="rounded-xl border border-surface-z3 overflow-hidden">
        {#each daemon as d, i}
          <div class="flex flex-col gap-1 px-5 py-3 {i % 2 === 0 ? 'bg-surface-z2' : 'bg-surface-z1'}">
            <code class="text-xs font-semibold text-primary-z7">{d.cmd}</code>
            <span class="text-xs text-surface-z5">{d.desc}</span>
          </div>
        {/each}
      </div>
      <div class="mt-4 rounded-lg bg-surface-z0 p-4 text-xs text-surface-z5">
        <strong class="text-surface-z7">Homebrew service:</strong> If installed via Homebrew, you can also use
        <code class="bg-surface-z3 px-1 rounded">brew services start sensei</code> to run senseid as a background service that starts on login.
      </div>
    </section>

    <!-- MCP Tools -->
    <section class="mb-14">
      <h2 class="mb-5 text-2xl font-bold">MCP Tools</h2>
      <p class="mb-5 text-sm text-surface-z5">When connected via <code class="bg-surface-z3 px-1 rounded">sensei-mcp</code>, your AI coding platform gets these tools:</p>
      <div class="rounded-xl border border-surface-z3 overflow-hidden">
        {#each mcpTools as t, i}
          <div class="flex flex-col gap-1 px-5 py-3 {i % 2 === 0 ? 'bg-surface-z2' : 'bg-surface-z1'}">
            <code class="text-xs font-semibold text-primary-z7">{t.tool}</code>
            <span class="text-xs text-surface-z5">{t.desc}</span>
          </div>
        {/each}
      </div>
    </section>

    <!-- Architecture -->
    <section class="mb-14">
      <h2 class="mb-5 text-2xl font-bold">Architecture</h2>
      <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
        <pre class="text-xs text-surface-z7 leading-relaxed"><code>┌─────────────┐     MCP      ┌────────────┐     HTTP     ┌──────────┐
│ Claude Code │ ──────────── │ sensei-mcp │ ──────────── │ senseid  │
│ Cursor      │  (stdio)     │            │  (:7744)     │ (daemon) │
│ Windsurf    │              └────────────┘              └──────────┘
└─────────────┘                                               │
                                                         ┌────┴─────┐
                                                         │ SQLite   │
                                                         │ Graph DB │
                                                         └──────────┘</code></pre>
      </div>
      <div class="mt-4 space-y-2 text-sm text-surface-z5">
        <p><strong class="text-surface-z7">senseid</strong> — The indexer daemon. Watches your repos, builds a code graph in SQLite, and serves an HTTP API.</p>
        <p><strong class="text-surface-z7">sensei-mcp</strong> — A thin MCP adapter. Translates MCP tool calls from your AI platform into senseid HTTP requests.</p>
        <p><strong class="text-surface-z7">sensei</strong> — The CLI. Manages installation, configuration, and scanning. Talks to senseid over HTTP.</p>
      </div>
    </section>

    <!-- Data -->
    <section class="border-t border-surface-z0 pt-10">
      <h2 class="mb-3 text-lg font-bold">Data storage</h2>
      <p class="text-sm text-surface-z5">All data is stored locally at <code class="bg-surface-z3 px-1 rounded">~/.sensei/</code>. Nothing is sent to external servers. The daemon stores its PID, logs, SQLite database, and per-repo graph data in this directory.</p>
    </section>

  </div>

  <!-- Footer -->
  <div class="border-t border-surface-z0 py-8 text-center text-xs text-surface-z4">
    <a href="{base}/" class="hover:text-surface-z6 transition-colors font-bold text-surface-z6">⬡ sensei</a>
    <span class="mx-2">·</span>
    <a href="{base}/setup" class="hover:text-surface-z6 transition-colors">Setup</a>
    <span class="mx-2">·</span>
    <a href="{base}/usage" class="hover:text-surface-z6 transition-colors">Usage</a>
    <span class="mx-2">·</span>
    <a href="https://github.com/mizukisu/sensei" target="_blank" rel="noopener" class="hover:text-surface-z6 transition-colors">GitHub</a>
  </div>

</div>
