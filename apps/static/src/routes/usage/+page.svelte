<script lang="ts">
  import { base } from '$app/paths';

  const cliGroups = [
    {
      title: 'Setup',
      icon: '🔧',
      commands: [
        { cmd: 'sensei configure', desc: 'Detect installed AI coding platforms and generate config' },
        { cmd: 'sensei install', desc: 'Install hooks, skills, and MCP server for all platforms' },
        { cmd: 'sensei install --acp claude-code', desc: 'Install for a specific platform only' },
        { cmd: 'sensei uninstall', desc: 'Remove sensei from all configured platforms' },
      ]
    },
    {
      title: 'Daemon',
      icon: '⚡',
      commands: [
        { cmd: 'sensei start', desc: 'Start the senseid daemon (default port 7744)' },
        { cmd: 'sensei start --port 8800', desc: 'Start on a custom port' },
        { cmd: 'sensei stop', desc: 'Stop the running daemon' },
        { cmd: 'sensei status', desc: 'Show daemon status — version, PID, uptime' },
      ]
    },
    {
      title: 'Indexing',
      icon: '🔍',
      commands: [
        { cmd: 'sensei scan ~/Developer', desc: 'Scan a folder and index all repos found' },
        { cmd: 'sensei add-lib react', desc: 'Add a library\'s docs (auto-discovers llms.txt)' },
        { cmd: 'sensei add-lib kavach --url https://...', desc: 'Add a library with explicit doc URL' },
      ]
    }
  ];

  const daemonCommands = [
    { cmd: 'senseid', desc: 'Start in foreground (logs to stdout)' },
    { cmd: 'senseid start', desc: 'Start as background daemon' },
    { cmd: 'senseid stop', desc: 'Stop the background daemon' },
    { cmd: 'senseid status', desc: 'Show version and PID' },
    { cmd: 'senseid logs', desc: 'Tail the last 50 lines' },
    { cmd: 'senseid clear-logs', desc: 'Clear the log file' },
  ];

  const mcpTools = [
    { tool: 'get_session_context', desc: 'Resume from last checkpoint, load repo orientation', icon: '🎯' },
    { tool: 'context_pack', desc: 'Load symbols relevant to a query — token-budgeted', icon: '📦' },
    { tool: 'search', desc: 'Semantic search across the entire codebase', icon: '🔎' },
    { tool: 'load_context', desc: 'Load a specific file with symbol extraction', icon: '📄' },
    { tool: 'take_snapshot', desc: 'Record progress for interruption recovery', icon: '💾' },
    { tool: 'checkpoint', desc: 'Close session with outcome and FTR scoring', icon: '✅' },
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

  <div class="mx-auto max-w-4xl px-8 py-16">

    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Reference</div>
    <h1 class="mb-4 text-4xl font-extrabold tracking-tight">Usage Guide</h1>
    <p class="mb-14 text-lg text-surface-z5">Everything you need to configure, run, and integrate sensei.</p>

    <!-- CLI Commands — grouped cards -->
    <section class="mb-16">
      <h2 class="mb-2 text-2xl font-bold">CLI Commands</h2>
      <p class="mb-8 text-sm text-surface-z5">The <code class="bg-surface-z3 px-1.5 py-0.5 rounded text-xs">sensei</code> binary handles setup, daemon lifecycle, and indexing.</p>

      <div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
        {#each cliGroups as group}
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
            <div class="flex items-center gap-2 mb-4">
              <span class="text-lg">{group.icon}</span>
              <h3 class="font-bold text-sm">{group.title}</h3>
            </div>
            <div class="space-y-3">
              {#each group.commands as c}
                <div>
                  <code class="block text-xs font-semibold text-primary-z6 mb-0.5">{c.cmd}</code>
                  <span class="text-[11px] text-surface-z4 leading-tight">{c.desc}</span>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </section>

    <!-- Daemon Management -->
    <section class="mb-16">
      <h2 class="mb-2 text-2xl font-bold">Daemon Management</h2>
      <p class="mb-8 text-sm text-surface-z5">The <code class="bg-surface-z3 px-1.5 py-0.5 rounded text-xs">senseid</code> daemon indexes repos and serves the local API.</p>

      <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
        {#each daemonCommands as d}
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
            <code class="block text-xs font-semibold text-primary-z6 mb-1">{d.cmd}</code>
            <span class="text-[11px] text-surface-z4">{d.desc}</span>
          </div>
        {/each}
      </div>

      <div class="mt-5 flex items-start gap-3 rounded-xl border border-surface-z3 bg-surface-z0 p-4">
        <span class="text-lg mt-0.5">🍺</span>
        <div>
          <p class="text-xs font-semibold text-surface-z7 mb-1">Homebrew service</p>
          <p class="text-xs text-surface-z4">Use <code class="bg-surface-z3 px-1 rounded">brew services start sensei</code> to run senseid as a login service.</p>
        </div>
      </div>
    </section>

    <!-- MCP Tools -->
    <section class="mb-16">
      <h2 class="mb-2 text-2xl font-bold">MCP Tools</h2>
      <p class="mb-8 text-sm text-surface-z5">Available to your AI coding platform when connected via <code class="bg-surface-z3 px-1.5 py-0.5 rounded text-xs">sensei-mcp</code>.</p>

      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {#each mcpTools as t}
          <div class="flex items-start gap-3 rounded-xl border border-surface-z3 bg-surface-z2 p-4 hover:border-primary-z4 transition-colors">
            <span class="text-lg mt-0.5">{t.icon}</span>
            <div>
              <code class="text-xs font-bold text-primary-z6">{t.tool}</code>
              <p class="text-[11px] text-surface-z4 mt-0.5">{t.desc}</p>
            </div>
          </div>
        {/each}
      </div>
    </section>

    <!-- Architecture -->
    <section class="mb-16">
      <h2 class="mb-6 text-2xl font-bold">Architecture</h2>
      <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-6">
        <pre class="text-xs text-surface-z7 leading-relaxed overflow-x-auto"><code>┌─────────────┐     MCP      ┌────────────┐     HTTP     ┌──────────┐
│ Claude Code │ ──────────── │ sensei-mcp │ ──────────── │ senseid  │
│ Cursor      │  (stdio)     │            │  (:7744)     │ (daemon) │
│ Windsurf    │              └────────────┘              └──────────┘
└─────────────┘                                               │
                                                         ┌────┴─────┐
                                                         │ SQLite   │
                                                         │ Graph DB │
                                                         └──────────┘</code></pre>
      </div>
      <div class="mt-5 grid grid-cols-1 gap-3 sm:grid-cols-3">
        {#each [
          { name: 'senseid', desc: 'Indexer daemon — watches repos, builds code graph, serves HTTP API', color: 'primary' },
          { name: 'sensei-mcp', desc: 'MCP adapter — translates AI platform tool calls to HTTP', color: 'secondary' },
          { name: 'sensei', desc: 'CLI — manages installation, config, scanning over HTTP', color: 'accent' },
        ] as comp}
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
            <code class="text-xs font-bold text-{comp.color}-z6">{comp.name}</code>
            <p class="text-[11px] text-surface-z4 mt-1">{comp.desc}</p>
          </div>
        {/each}
      </div>
    </section>

    <!-- Data storage -->
    <section class="rounded-xl border border-surface-z3 bg-surface-z0 p-6 mb-10">
      <div class="flex items-start gap-3">
        <span class="text-lg">🔒</span>
        <div>
          <h2 class="text-sm font-bold mb-1">Local-first, private by default</h2>
          <p class="text-xs text-surface-z5">All data is stored at <code class="bg-surface-z3 px-1 rounded">~/.sensei/</code>. Nothing is sent to external servers. The daemon stores its PID, logs, SQLite database, and per-repo graph data locally.</p>
        </div>
      </div>
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
