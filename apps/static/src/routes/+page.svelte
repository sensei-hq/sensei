<script lang="ts">
  import { base } from '$app/paths';
  import { browser } from '$app/environment';
  import { onMount } from 'svelte';
  import * as d3 from 'd3';

  const GITHUB = 'https://github.com/mizukisu/sensei';
  const RELEASES = 'https://github.com/mizukisu/sensei-releases';
  const RELEASE_BASE = `${RELEASES}/releases/latest/download`;

  // ── Platform detection ──────────────────────────────────────────────────
  type Platform = { os: string; svg: string; file: string; label: string };

  // Compact SVG icon paths (viewBox 0 0 24 24)
  const appleSvg = '<path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z" fill="currentColor"/>';
  const windowsSvg = '<path d="M3 12.5V5.81l7.5-1.04V12.5H3zm8.5-7.92L21 3v9.5H11.5V4.58zM3 13.5h7.5v7.73L3 20.19V13.5zm8.5 0H21V21l-9.5-1.42V13.5z" fill="currentColor"/>';
  const linuxSvg = '<path d="M12.5 2C10 2 8.2 4.04 8.2 7.04c0 1.6.5 2.8 1.1 3.96-.9.6-2.7 1.8-3.3 3-1 2 .5 4 2.5 4h7c2 0 3.5-2 2.5-4-.6-1.2-2.4-2.4-3.3-3 .6-1.16 1.1-2.36 1.1-3.96C15.8 4.04 15 2 12.5 2zm-1.25 4a.75.75 0 110 1.5.75.75 0 010-1.5zm2.5 0a.75.75 0 110 1.5.75.75 0 010-1.5zM11 9.5h3s-.5 1.5-1.5 1.5S11 9.5 11 9.5z" fill="currentColor"/>';

  const platforms: Platform[] = [
    { os: 'mac-arm',   svg: appleSvg,   file: 'sensei-cli-macos-arm64.tar.gz',   label: 'macOS Apple Silicon' },
    { os: 'mac-intel', svg: appleSvg,   file: 'sensei-cli-macos-x86_64.tar.gz',  label: 'macOS Intel' },
    { os: 'linux-x64', svg: linuxSvg,   file: 'sensei-cli-linux-x86_64.tar.gz',  label: 'Linux x86_64' },
    { os: 'linux-arm', svg: linuxSvg,   file: 'sensei-cli-linux-arm64.tar.gz',   label: 'Linux ARM64' },
    { os: 'windows',   svg: windowsSvg, file: 'sensei-cli-windows-x86_64.zip',   label: 'Windows x86_64' },
  ];
  function detectPlatform(): Platform {
    if (!browser) return platforms[0];
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('win')) return platforms[4];
    if (ua.includes('linux')) return ua.includes('aarch64') || ua.includes('arm') ? platforms[3] : platforms[2];
    return platforms[0];
  }
  let detected = $derived(detectPlatform());

  // ── Data ────────────────────────────────────────────────────────────────
  const cliGroups = [
    { title: 'Setup', icon: '🔧', commands: [
      { cmd: 'sensei configure', desc: 'Detect AI coding platforms and generate config' },
      { cmd: 'sensei install', desc: 'Install hooks, skills, and MCP server' },
      { cmd: 'sensei install --acp claude-code', desc: 'Install for a specific platform' },
      { cmd: 'sensei uninstall', desc: 'Remove sensei from all platforms' },
    ]},
    { title: 'Indexing', icon: '🔍', commands: [
      { cmd: 'sensei scan ~/Developer', desc: 'Scan a folder and index all repos' },
      { cmd: 'sensei add-lib react', desc: 'Add library docs (auto-discovers llms.txt)' },
    ]},
  ];
  const daemonCommands = [
    { cmd: 'senseid start', desc: 'Start as background daemon' },
    { cmd: 'senseid stop', desc: 'Stop the daemon' },
    { cmd: 'senseid status', desc: 'Show version and PID' },
    { cmd: 'senseid logs', desc: 'Tail the last 50 lines' },
  ];
  const mcpTools = [
    { tool: 'get_session_context', desc: 'Resume from last checkpoint, load repo orientation', icon: '🎯' },
    { tool: 'context_pack', desc: 'Load symbols relevant to a query — token-budgeted', icon: '📦' },
    { tool: 'search', desc: 'Semantic search across the entire codebase', icon: '🔎' },
    { tool: 'load_context', desc: 'Load a specific file with symbol extraction', icon: '📄' },
    { tool: 'take_snapshot', desc: 'Record progress for interruption recovery', icon: '💾' },
    { tool: 'checkpoint', desc: 'Close session with outcome and FTR scoring', icon: '✅' },
  ];

  // ── D3 Architecture diagram ────────────────────────────────────────────
  let graphContainer: HTMLDivElement;
  onMount(() => drawArchGraph());

  function drawArchGraph() {
    const w = graphContainer.clientWidth;
    const h = 420;
    const nodeH = 36, rx = 8;
    const col1 = 80, col2 = w * 0.35, col3 = w * 0.60;
    const col4 = w - 80 - 52;

    const primary   = 'rgb(124,58,237)';
    const secondary = 'rgb(13,148,136)';
    const accent    = 'rgb(79,70,229)';
    const warn      = 'rgb(217,119,6)';
    const plat      = '#a78bfa';
    const stor      = '#64748b';

    interface N { id: string; label: string; x: number; y: number; color: string; w: number; sub?: string }
    interface L { source: string; target: string; label?: string }

    const nodes: N[] = [
      { id: 'claude',   label: 'Claude Code',    x: col1, y: 60,  color: plat, w: 120, sub: 'claude' },
      { id: 'cursor',   label: 'Cursor',         x: col1, y: 110, color: plat, w: 120, sub: 'cursor' },
      { id: 'windsurf', label: 'Windsurf',       x: col1, y: 160, color: plat, w: 120, sub: 'windsurf' },
      { id: 'mcp',      label: 'sensei-mcp',     x: col2, y: 110, color: secondary, w: 115 },
      { id: 'desktop',  label: 'Sensei Desktop', x: col1, y: 280, color: warn, w: 130 },
      { id: 'cli',      label: 'sensei CLI',     x: col1, y: 340, color: accent, w: 120 },
      { id: 'senseid',  label: 'senseid',        x: col3, y: 210, color: primary, w: 110 },
      { id: 'dotsensei',label: '~/.sensei',      x: col4, y: 150, color: stor, w: 105 },
      { id: 'kuzu',     label: 'Kuzu',           x: col4, y: 210, color: '#f97316', w: 105 },
      { id: 'sqlite',   label: 'SQLite',         x: col4, y: 270, color: '#0f9fd8', w: 105 },
    ];
    const links: L[] = [
      { source: 'claude', target: 'mcp', label: 'MCP' },
      { source: 'cursor', target: 'mcp' },
      { source: 'windsurf', target: 'mcp' },
      { source: 'mcp', target: 'senseid', label: 'HTTP :7744' },
      { source: 'desktop', target: 'senseid', label: 'HTTP' },
      { source: 'cli', target: 'senseid' },
      { source: 'senseid', target: 'dotsensei' },
      { source: 'senseid', target: 'kuzu' },
      { source: 'senseid', target: 'sqlite' },
    ];

    const svg = d3.select(graphContainer).append('svg').attr('width', w).attr('height', h).attr('viewBox', `0 0 ${w} ${h}`);
    svg.append('defs').append('marker').attr('id', 'arrow').attr('viewBox', '0 0 10 6').attr('refX', 10).attr('refY', 3)
      .attr('markerWidth', 8).attr('markerHeight', 6).attr('orient', 'auto')
      .append('path').attr('d', 'M0,0 L10,3 L0,6').attr('fill', '#475569');

    const nm = new Map(nodes.map(n => [n.id, n]));

    links.forEach(l => {
      const s = nm.get(l.source)!, t = nm.get(l.target)!;
      const x1 = s.x + s.w / 2, y1 = s.y, x2 = t.x - t.w / 2, y2 = t.y;
      const dx = x2 - x1, dy = y2 - y1;
      const path = Math.abs(dy) > 30
        ? `M${x1},${y1} C${x1 + dx * 0.5},${y1} ${x2 - dx * 0.5},${y2} ${x2},${y2}`
        : `M${x1},${y1} L${x2},${y2}`;
      svg.append('path').attr('d', path).attr('fill', 'none').attr('stroke', '#334155')
        .attr('stroke-width', 1.5).attr('stroke-dasharray', '5,3').attr('marker-end', 'url(#arrow)');
      if (l.label) {
        svg.append('text').attr('x', (x1 + x2) / 2).attr('y', (y1 + y2) / 2 - 12)
          .attr('text-anchor', 'middle').attr('fill', '#64748b').attr('font-size', '10px')
          .attr('font-family', 'ui-monospace, monospace').text(l.label);
      }
    });

    nodes.forEach(n => {
      const g = svg.append('g');
      g.append('rect').attr('x', n.x - n.w / 2).attr('y', n.y - nodeH / 2).attr('width', n.w).attr('height', nodeH)
        .attr('rx', rx).attr('fill', 'rgba(15,23,42,0.85)').attr('stroke', n.color).attr('stroke-width', 1.5);
      g.append('text').attr('x', n.x).attr('y', n.y + 4).attr('text-anchor', 'middle')
        .attr('fill', '#e2e8f0').attr('font-size', '11px').attr('font-weight', '600')
        .attr('font-family', 'ui-monospace, monospace').text(n.label);
    });

    const gl = (x: number, y: number, t: string) => svg.append('text').attr('x', x).attr('y', y)
      .attr('text-anchor', 'middle').attr('fill', '#475569').attr('font-size', '9px').attr('font-weight', '600')
      .attr('letter-spacing', '1px').attr('font-family', 'system-ui, sans-serif').text(t);
    gl(col1, 30, 'AI PLATFORMS');
    gl(col1, 255, 'CLIENTS');
    gl(col4, 125, 'STORAGE');
  }
</script>

<!-- Hero -->
<div class="mx-auto max-w-3xl px-8 py-24 text-center">
  <div class="mb-6 inline-flex items-center gap-2 rounded-full border border-primary-z3 bg-primary-z1 px-3 py-1 text-xs font-semibold text-primary-z7">
    🧠 AI Development Intelligence
  </div>
  <h1 class="mb-5 text-5xl font-extrabold leading-tight tracking-tight">
    Ship faster.<br>
    <span class="bg-gradient-to-r from-primary-z6 to-secondary-z5 bg-clip-text text-transparent">Measure everything.</span>
  </h1>
  <p class="mb-9 mx-auto max-w-xl text-lg leading-relaxed text-surface-z5">
    Sensei captures how your team uses AI — context quality, session costs, first-try-right rates — and feeds that intelligence back into your workflow.
  </p>
  <div class="flex flex-wrap items-center justify-center gap-3">
    <a href="#setup" class="rounded-xl bg-primary-z5 px-6 py-3 text-sm font-semibold text-white hover:bg-primary-z6 transition-colors">
      Get started
    </a>
    <a href="#usage" class="rounded-xl border border-surface-z3 bg-surface-z2 px-6 py-3 text-sm font-medium hover:bg-surface-z3 transition-colors">
      Usage guide →
    </a>
  </div>
</div>

<!-- Stats -->
<div class="border-y border-surface-z0 bg-surface-z2 py-10">
  <div class="mx-auto grid max-w-4xl grid-cols-2 gap-8 px-8 text-center sm:grid-cols-4">
    {#each [
      { n: '62%', l: 'avg FTR improvement' }, { n: '$0.18', l: 'median task cost' },
      { n: '4.2×', l: 'faster with context' }, { n: '38%', l: 'token savings via cache' },
    ] as stat}
      <div>
        <div class="text-3xl font-extrabold text-surface-z8">{stat.n}</div>
        <div class="mt-1 text-xs font-medium uppercase tracking-wider text-surface-z4">{stat.l}</div>
      </div>
    {/each}
  </div>
</div>

<!-- Setup -->
<div id="setup" class="mx-auto max-w-4xl px-8 py-20">
  <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Setup</div>
  <h2 class="mb-3 text-3xl font-bold tracking-tight">Install Sensei</h2>
  <p class="mb-10 text-surface-z5">Get the CLI and daemon running in under two minutes.</p>

  <!-- Platform-detected download -->
  <div class="mb-8 rounded-xl border border-primary-z4 bg-primary-z1 p-5">
    <div class="flex items-center justify-between gap-4 flex-wrap">
      <div class="flex items-center gap-3">
        <svg class="w-7 h-7 text-surface-z7 shrink-0" viewBox="0 0 24 24">{@html detected.svg}</svg>
        <div>
          <p class="text-sm font-semibold text-surface-z8">Download for {detected.label}</p>
          <p class="text-xs text-surface-z5 font-mono">{detected.file}</p>
        </div>
      </div>
      <a href="{RELEASE_BASE}/{detected.file}"
         class="rounded-lg bg-primary-z5 px-5 py-2.5 text-sm font-semibold text-white hover:bg-primary-z6 transition-colors whitespace-nowrap">
        Download
      </a>
    </div>
  </div>

  <div class="grid grid-cols-1 gap-5 md:grid-cols-3">
    <!-- Homebrew -->
    <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
      <div class="flex items-center gap-2 mb-3">
        <span class="text-lg">🍺</span>
        <span class="font-semibold">Homebrew</span>
      </div>
      <pre class="rounded-lg bg-surface-z0 p-3 text-xs text-surface-z7 overflow-x-auto"><code>brew install mizukisu/tap/sensei</code></pre>
    </div>

    <!-- Quick start -->
    <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
      <div class="flex items-center gap-2 mb-3">
        <span class="text-lg">⚡</span>
        <span class="font-semibold">Quick start</span>
      </div>
      <pre class="rounded-lg bg-surface-z0 p-3 text-xs text-surface-z7 overflow-x-auto"><code>sensei configure
sensei install
sensei start</code></pre>
    </div>

    <!-- All platforms -->
    <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
      <div class="flex items-center gap-2 mb-3">
        <span class="text-lg">📦</span>
        <span class="font-semibold">All downloads</span>
      </div>
      <div class="space-y-1 text-xs">
        {#each platforms as p}
          <a href="{RELEASE_BASE}/{p.file}"
             class="flex items-center gap-2 {p.os === detected.os ? 'text-primary-z6 font-semibold' : 'text-surface-z5'} hover:text-primary-z6 transition-colors">
            <svg class="w-3.5 h-3.5 shrink-0" viewBox="0 0 24 24">{@html p.svg}</svg> {p.label}
          </a>
        {/each}
      </div>
    </div>
  </div>
</div>

<!-- Features -->
<div id="features" class="border-t border-surface-z0 bg-surface-z2">
  <div class="mx-auto max-w-5xl px-8 py-20">
    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">What sensei does</div>
    <h2 class="mb-3 text-3xl font-bold tracking-tight">The observability layer for AI-assisted development</h2>
    <p class="mb-12 max-w-md text-surface-z5">Like Code Climate for code quality — sensei gives you the metrics to understand and improve how your team uses AI.</p>
    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
      {#each [
        { ico: '🎯', h: 'First-Try-Right Scoring',  p: 'Track how often AI completes tasks on the first attempt. Per-repo, per-agent, per-team.' },
        { ico: '💰', h: 'Cost Intelligence',        p: 'Token-level cost per task, session, and repo. See exactly where money is spent.' },
        { ico: '📦', h: 'Context Packing',          p: 'Smart token-budgeted context using BM25 + semantic search. Right code, right size.' },
        { ico: '🔄', h: 'Session Continuity',       p: 'Automatic crash recovery and progress snapshots. Never lose context mid-task.' },
        { ico: '📚', h: 'Doc Drift Detection',      p: 'Traceability matrix linking code to docs. Flags stale documentation automatically.' },
        { ico: '🧩', h: 'Plugin Ecosystem',         p: 'Skills, commands, and hooks that inject guardrails into every AI session.' },
      ] as f}
        <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-6">
          <div class="mb-4 text-2xl">{f.ico}</div>
          <h3 class="mb-2 text-sm font-semibold text-surface-z8">{f.h}</h3>
          <p class="text-sm leading-relaxed text-surface-z5">{f.p}</p>
        </div>
      {/each}
    </div>
  </div>
</div>

<!-- Usage -->
<div id="usage" class="mx-auto max-w-4xl px-8 py-20">
  <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Reference</div>
  <h2 class="mb-3 text-3xl font-bold tracking-tight">Usage Guide</h2>
  <p class="mb-10 text-surface-z5">CLI commands, daemon management, and MCP tools.</p>

  <!-- CLI + Daemon side by side -->
  <div class="grid grid-cols-1 gap-5 mb-10 lg:grid-cols-2">
    <!-- CLI Commands -->
    <div>
      <h3 class="mb-4 text-lg font-bold">CLI Commands</h3>
      {#each cliGroups as group}
        <div class="mb-4 rounded-xl border border-surface-z3 bg-surface-z2 p-4">
          <div class="flex items-center gap-2 mb-3">
            <span>{group.icon}</span>
            <span class="font-bold text-sm">{group.title}</span>
          </div>
          <div class="space-y-2">
            {#each group.commands as c}
              <div>
                <code class="text-xs font-semibold text-primary-z6">{c.cmd}</code>
                <span class="text-xs text-surface-z5 ml-2">{c.desc}</span>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    <!-- Daemon Management -->
    <div>
      <h3 class="mb-4 text-lg font-bold">Daemon Management</h3>
      <div class="grid grid-cols-2 gap-3">
        {#each daemonCommands as d}
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-3">
            <code class="block text-xs font-semibold text-primary-z6 mb-0.5">{d.cmd}</code>
            <span class="text-xs text-surface-z5">{d.desc}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <!-- MCP Tools -->
  <h3 class="mb-4 text-lg font-bold">MCP Tools</h3>
  <p class="mb-5 text-sm text-surface-z5">Available to your AI coding platform when connected via <code class="bg-surface-z3 px-1.5 py-0.5 rounded text-xs">sensei-mcp</code>.</p>
  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 mb-10">
    {#each mcpTools as t}
      <div class="flex items-start gap-3 rounded-xl border border-surface-z3 bg-surface-z2 p-4">
        <span class="text-lg">{t.icon}</span>
        <div>
          <code class="text-xs font-bold text-primary-z6">{t.tool}</code>
          <p class="text-xs text-surface-z5 mt-0.5">{t.desc}</p>
        </div>
      </div>
    {/each}
  </div>

  <!-- Architecture -->
  <h3 class="mb-4 text-lg font-bold">Architecture</h3>
  <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-6 overflow-hidden" bind:this={graphContainer}></div>
  <div class="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
    {#each [
      { name: 'senseid', desc: 'Indexer daemon — watches repos, builds code graph, serves HTTP API', color: 'primary' },
      { name: 'sensei-mcp', desc: 'MCP adapter — translates AI tool calls into HTTP', color: 'secondary' },
      { name: 'sensei CLI', desc: 'Manages installation, config, scanning', color: 'accent' },
      { name: 'Desktop', desc: 'Project navigator, graph viewer, indexer control', color: 'warning' },
    ] as comp}
      <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
        <code class="text-xs font-bold text-{comp.color}-z6">{comp.name}</code>
        <p class="text-xs text-surface-z6 mt-1">{comp.desc}</p>
      </div>
    {/each}
  </div>
</div>

<!-- Sponsor -->
<div id="sponsor" class="border-t border-surface-z0 bg-surface-z2">
  <div class="mx-auto max-w-3xl px-8 py-20 text-center">
    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Support</div>
    <h2 class="mb-4 text-3xl font-bold tracking-tight">Sponsor Sensei</h2>
    <p class="mb-10 mx-auto max-w-xl text-surface-z5 leading-relaxed">
      Sensei is open source and free to use. If it saves you time, consider sponsoring the project.
    </p>
    <div class="flex flex-wrap justify-center gap-4">
      <a href="{GITHUB}/sponsors" target="_blank" rel="noopener"
         class="flex items-center gap-2 rounded-xl bg-pink-500 px-6 py-3 text-sm font-semibold text-white hover:bg-pink-600 transition-colors">
        ❤️ GitHub Sponsors
      </a>
      <a href="https://opencollective.com/sensei-dev" target="_blank" rel="noopener"
         class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-6 py-3 text-sm font-medium hover:bg-surface-z2 transition-colors">
        🌐 Open Collective
      </a>
      <a href="https://ko-fi.com/senseidev" target="_blank" rel="noopener"
         class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-6 py-3 text-sm font-medium hover:bg-surface-z2 transition-colors">
        ☕ Ko-fi
      </a>
    </div>
    <div class="mt-12 grid grid-cols-1 gap-4 sm:grid-cols-3 text-left">
      {#each [
        { tier: '☕ Coffee', price: '$5/mo', perks: ['Supporter badge', 'Thanks in release notes'] },
        { tier: '🚀 Pro', price: '$25/mo', perks: ['Priority issues', 'Discord access', 'Early previews'] },
        { tier: '🏢 Team', price: '$100/mo', perks: ['Logo in README', 'Direct support channel', 'Feature input'] },
      ] as t}
        <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-5">
          <div class="font-semibold text-surface-z8">{t.tier}</div>
          <div class="mt-1 text-2xl font-extrabold text-primary-z6">{t.price}</div>
          <ul class="mt-4 space-y-1">
            {#each t.perks as perk}
              <li class="flex items-center gap-2 text-xs text-surface-z5"><span class="text-success-z5">✓</span> {perk}</li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  </div>
</div>
