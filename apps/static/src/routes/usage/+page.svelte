<script lang="ts">
  import { onMount } from 'svelte';
  import * as d3 from 'd3';

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

  let graphContainer: HTMLDivElement;

  onMount(() => {
    drawArchGraph();
  });

  function drawArchGraph() {
    const w = graphContainer.clientWidth;
    const h = 280;
    const nodeW = 120, nodeH = 50, rx = 10;

    const nodes = [
      { id: 'acp',     label: 'Claude Code\nCursor\nWindsurf', x: 80,       y: h/2,     color: '#a78bfa', w: 110 },
      { id: 'mcp',     label: 'sensei-mcp',                    x: w * 0.38, y: h/2,     color: '#2dd4bf', w: nodeW },
      { id: 'senseid', label: 'senseid',                       x: w * 0.68, y: h/2,     color: '#818cf8', w: nodeW },
      { id: 'db',      label: 'SQLite\nGraph DB',              x: w * 0.68, y: h - 30,  color: '#6366f1', w: 100 },
    ];

    const links = [
      { source: 'acp', target: 'mcp',     label: 'MCP (stdio)' },
      { source: 'mcp', target: 'senseid', label: 'HTTP :7744' },
      { source: 'senseid', target: 'db',  label: '' },
    ];

    const svg = d3.select(graphContainer)
      .append('svg')
      .attr('width', w)
      .attr('height', h)
      .attr('viewBox', `0 0 ${w} ${h}`);

    // Defs for arrow marker
    svg.append('defs').append('marker')
      .attr('id', 'arrow')
      .attr('viewBox', '0 0 10 6')
      .attr('refX', 10)
      .attr('refY', 3)
      .attr('markerWidth', 8)
      .attr('markerHeight', 6)
      .attr('orient', 'auto')
      .append('path')
      .attr('d', 'M0,0 L10,3 L0,6')
      .attr('fill', '#64748b');

    const nodeMap = new Map(nodes.map(n => [n.id, n]));

    // Draw links
    links.forEach(link => {
      const s = nodeMap.get(link.source)!;
      const t = nodeMap.get(link.target)!;

      const isVertical = s.id === 'senseid' && t.id === 'db';
      const x1 = isVertical ? s.x : s.x + (s.w / 2);
      const y1 = isVertical ? s.y + nodeH / 2 : s.y;
      const x2 = isVertical ? t.x : t.x - (t.w / 2);
      const y2 = isVertical ? t.y - nodeH / 2 : t.y;

      svg.append('line')
        .attr('x1', x1).attr('y1', y1)
        .attr('x2', x2).attr('y2', y2)
        .attr('stroke', '#475569')
        .attr('stroke-width', 2)
        .attr('stroke-dasharray', '6,3')
        .attr('marker-end', 'url(#arrow)');

      if (link.label) {
        const mx = (x1 + x2) / 2;
        const my = (y1 + y2) / 2 - 10;
        svg.append('text')
          .attr('x', mx).attr('y', my)
          .attr('text-anchor', 'middle')
          .attr('fill', '#94a3b8')
          .attr('font-size', '11px')
          .attr('font-family', 'ui-monospace, monospace')
          .text(link.label);
      }
    });

    // Draw nodes
    nodes.forEach(node => {
      const g = svg.append('g');
      const nw = node.w;

      g.append('rect')
        .attr('x', node.x - nw / 2)
        .attr('y', node.y - nodeH / 2)
        .attr('width', nw)
        .attr('height', nodeH)
        .attr('rx', rx)
        .attr('fill', 'rgba(30,41,59,0.9)')
        .attr('stroke', node.color)
        .attr('stroke-width', 2);

      const lines = node.label.split('\n');
      lines.forEach((line, i) => {
        const yOff = node.y + (i - (lines.length - 1) / 2) * 14;
        g.append('text')
          .attr('x', node.x)
          .attr('y', yOff + 4)
          .attr('text-anchor', 'middle')
          .attr('fill', i === 0 && lines.length === 1 ? node.color : '#e2e8f0')
          .attr('font-size', lines.length > 2 ? '10px' : '12px')
          .attr('font-weight', lines.length === 1 ? '700' : '400')
          .attr('font-family', 'ui-monospace, monospace')
          .text(line);
      });
    });
  }
</script>

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

  <!-- Architecture — D3 graph -->
  <section class="mb-16">
    <h2 class="mb-6 text-2xl font-bold">Architecture</h2>
    <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-6 overflow-hidden" bind:this={graphContainer}></div>
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
