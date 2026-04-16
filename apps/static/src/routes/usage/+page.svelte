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
    const h = 420;
    const nodeH = 36, rx = 8;

    // Layout columns
    const col1 = 80;                // AI platforms + clients
    const col2 = w * 0.35;          // sensei-mcp
    const col3 = w * 0.60;          // senseid
    const col4 = w - 70;            // storage

    // Colors
    const purple = '#a78bfa';
    const teal   = '#2dd4bf';
    const indigo = '#818cf8';
    const amber  = '#fbbf24';
    const green  = '#34d399';
    const slate  = '#64748b';

    interface Node { id: string; label: string; x: number; y: number; color: string; w: number; icon?: string }
    interface Link { source: string; target: string; label?: string; side?: 'right'|'bottom'|'top' }

    const nodes: Node[] = [
      // AI Platforms (left, stacked)
      { id: 'claude',   label: 'Claude Code', x: col1, y: 60,  color: purple, w: 105, icon: '🤖' },
      { id: 'cursor',   label: 'Cursor',      x: col1, y: 110, color: purple, w: 105, icon: '📝' },
      { id: 'windsurf', label: 'Windsurf',    x: col1, y: 160, color: purple, w: 105, icon: '🏄' },

      // MCP server
      { id: 'mcp', label: 'sensei-mcp', x: col2, y: 110, color: teal, w: 115 },

      // Clients (below platforms)
      { id: 'desktop', label: 'Sensei Desktop', x: col1, y: 280, color: amber, w: 125, icon: '🖥' },
      { id: 'cli',     label: 'sensei CLI',     x: col1, y: 340, color: green, w: 105, icon: '⌨️' },

      // Daemon (center)
      { id: 'senseid', label: 'senseid', x: col3, y: 210, color: indigo, w: 110 },

      // Storage (right, stacked)
      { id: 'dotsensei', label: '~/.sensei',  x: col4, y: 150, color: slate, w: 90, icon: '📁' },
      { id: 'kuzu',      label: 'Kuzu',       x: col4, y: 210, color: slate, w: 90, icon: '🔗' },
      { id: 'sqlite',    label: 'SQLite',     x: col4, y: 270, color: slate, w: 90, icon: '🗄' },
    ];

    const links: Link[] = [
      // AI platforms → mcp
      { source: 'claude',   target: 'mcp', label: 'MCP' },
      { source: 'cursor',   target: 'mcp' },
      { source: 'windsurf', target: 'mcp' },
      // mcp → senseid
      { source: 'mcp', target: 'senseid', label: 'HTTP :7744' },
      // clients → senseid
      { source: 'desktop', target: 'senseid', label: 'HTTP' },
      { source: 'cli',     target: 'senseid' },
      // senseid → storage
      { source: 'senseid', target: 'dotsensei' },
      { source: 'senseid', target: 'kuzu' },
      { source: 'senseid', target: 'sqlite' },
    ];

    const svg = d3.select(graphContainer)
      .append('svg')
      .attr('width', w)
      .attr('height', h)
      .attr('viewBox', `0 0 ${w} ${h}`);

    // Arrow marker
    svg.append('defs').append('marker')
      .attr('id', 'arrow')
      .attr('viewBox', '0 0 10 6')
      .attr('refX', 10).attr('refY', 3)
      .attr('markerWidth', 8).attr('markerHeight', 6)
      .attr('orient', 'auto')
      .append('path').attr('d', 'M0,0 L10,3 L0,6').attr('fill', '#475569');

    const nodeMap = new Map(nodes.map(n => [n.id, n]));

    // Draw links
    links.forEach(link => {
      const s = nodeMap.get(link.source)!;
      const t = nodeMap.get(link.target)!;
      const x1 = s.x + s.w / 2;
      const y1 = s.y;
      const x2 = t.x - t.w / 2;
      const y2 = t.y;

      // Use a curved path for non-horizontal links
      const dx = x2 - x1, dy = y2 - y1;
      const path = Math.abs(dy) > 30
        ? `M${x1},${y1} C${x1 + dx * 0.5},${y1} ${x2 - dx * 0.5},${y2} ${x2},${y2}`
        : `M${x1},${y1} L${x2},${y2}`;

      svg.append('path')
        .attr('d', path)
        .attr('fill', 'none')
        .attr('stroke', '#334155')
        .attr('stroke-width', 1.5)
        .attr('stroke-dasharray', '5,3')
        .attr('marker-end', 'url(#arrow)');

      if (link.label) {
        const mx = (x1 + x2) / 2;
        const my = (y1 + y2) / 2 - 12;
        svg.append('text')
          .attr('x', mx).attr('y', my)
          .attr('text-anchor', 'middle')
          .attr('fill', '#64748b')
          .attr('font-size', '10px')
          .attr('font-family', 'ui-monospace, monospace')
          .text(link.label);
      }
    });

    // Draw nodes
    nodes.forEach(node => {
      const g = svg.append('g');
      const nw = node.w;

      // Background rect
      g.append('rect')
        .attr('x', node.x - nw / 2)
        .attr('y', node.y - nodeH / 2)
        .attr('width', nw)
        .attr('height', nodeH)
        .attr('rx', rx)
        .attr('fill', 'rgba(15,23,42,0.85)')
        .attr('stroke', node.color)
        .attr('stroke-width', 1.5);

      if (node.icon) {
        g.append('text')
          .attr('x', node.x - nw / 2 + 12)
          .attr('y', node.y + 5)
          .attr('font-size', '13px')
          .text(node.icon);

        g.append('text')
          .attr('x', node.x - nw / 2 + 28)
          .attr('y', node.y + 4)
          .attr('fill', '#e2e8f0')
          .attr('font-size', '11px')
          .attr('font-weight', '500')
          .attr('font-family', 'ui-monospace, monospace')
          .text(node.label);
      } else {
        g.append('text')
          .attr('x', node.x)
          .attr('y', node.y + 4)
          .attr('text-anchor', 'middle')
          .attr('fill', node.color)
          .attr('font-size', '12px')
          .attr('font-weight', '700')
          .attr('font-family', 'ui-monospace, monospace')
          .text(node.label);
      }
    });

    // Group labels
    const groupLabel = (x: number, y: number, text: string) => {
      svg.append('text')
        .attr('x', x).attr('y', y)
        .attr('text-anchor', 'middle')
        .attr('fill', '#475569')
        .attr('font-size', '9px')
        .attr('font-weight', '600')
        .attr('text-transform', 'uppercase')
        .attr('letter-spacing', '1px')
        .attr('font-family', 'system-ui, sans-serif')
        .text(text);
    };
    groupLabel(col1, 30, 'AI PLATFORMS');
    groupLabel(col1, 255, 'CLIENTS');
    groupLabel(col4, 125, 'STORAGE');
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

    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2">
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
    <div class="mt-5 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {#each [
        { name: 'senseid', desc: 'Indexer daemon — watches repos, builds code graph, serves HTTP API', color: 'primary' },
        { name: 'sensei-mcp', desc: 'MCP adapter — translates AI platform tool calls into HTTP requests', color: 'secondary' },
        { name: 'sensei CLI', desc: 'Manages installation, config, scanning — talks to senseid over HTTP', color: 'accent' },
        { name: 'Sensei Desktop', desc: 'Tauri app — project navigator, graph viewer, indexer control', color: 'warning' },
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
