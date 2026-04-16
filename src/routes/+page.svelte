<script lang="ts">
  import { base } from '$app/paths';
  import { browser } from '$app/environment';
  import { onMount } from 'svelte';
  import * as d3 from 'd3';

  const GITHUB = 'https://github.com/mizukisu/sensei-dev';
  const RELEASES = 'https://github.com/mizukisu/sensei-releases';
  const RELEASE_BASE = `${RELEASES}/releases/latest/download`;

  // ── Platform detection ──────────────────────────────────────────────────
  type Platform = { os: string; svg: string; vb: string; file: string; label: string };

  // Official SVG icon paths
  // Apple (viewBox 0 0 24 24)
  const appleSvg = '<path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z" fill="currentColor"/>';
  // Windows (viewBox 0 0 24 24)
  const windowsSvg = '<path d="M3 12.5V5.81l7.5-1.04V12.5H3zm8.5-7.92L21 3v9.5H11.5V4.58zM3 13.5h7.5v7.73L3 20.19V13.5zm8.5 0H21V21l-9.5-1.42V13.5z" fill="currentColor"/>';
  // Linux — mdi:linux (viewBox 0 0 24 24)
  const linuxSvg = '<path fill="currentColor" d="M14.62 8.35c-.42.28-1.75 1.04-1.95 1.19c-.39.31-.75.29-1.14-.01c-.2-.16-1.53-.92-1.95-1.19c-.48-.31-.45-.7.08-.92c1.64-.69 3.28-.64 4.91.03c.49.21.51.6.05.9m7.22 7.28c-.93-2.09-2.2-3.99-3.84-5.66a4.3 4.3 0 0 1-1.06-1.88c-.1-.33-.17-.67-.24-1.01c-.2-.88-.29-1.78-.7-2.61c-.73-1.58-2-2.4-3.84-2.47c-1.81.05-3.16.81-3.95 2.4c-.21.43-.36.88-.46 1.34c-.17.76-.32 1.55-.5 2.32c-.15.65-.45 1.21-.96 1.71c-1.61 1.57-2.9 3.37-3.88 5.35c-.14.29-.28.58-.37.88c-.19.66.29 1.12.99.96c.44-.09.88-.18 1.3-.31c.41-.15.57-.05.67.35c.65 2.15 2.07 3.66 4.24 4.5c4.12 1.56 8.93-.66 9.97-4.58c.07-.27.17-.37.47-.27c.46.14.93.24 1.4.35c.49.09.85-.16.92-.64c.03-.26-.06-.49-.16-.73"/>';

  // Desktop app installers — bootstraps daemon, CLI, and MCP on first run
  const platforms: Platform[] = [
    { os: 'mac-arm',   svg: appleSvg,   vb: '0 0 24 24', file: 'Sensei_aarch64.dmg',        label: 'macOS Apple Silicon' },
    { os: 'mac-intel', svg: appleSvg,   vb: '0 0 24 24', file: 'Sensei_x86_64.dmg',         label: 'macOS Intel' },
    { os: 'linux-x64', svg: linuxSvg,   vb: '0 0 24 24', file: 'sensei_amd64.AppImage',     label: 'Linux x86_64' },
    { os: 'linux-arm', svg: linuxSvg,   vb: '0 0 24 24', file: 'sensei_arm64.AppImage',     label: 'Linux ARM64' },
    { os: 'windows',   svg: windowsSvg, vb: '0 0 24 24', file: 'Sensei_x86_64-setup.exe',   label: 'Windows x86_64' },
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
    const col4 = w - 80 - 55;       // right edge at w-80 (110/2=55)

    const primary   = 'rgb(124,58,237)';
    const secondary = 'rgb(13,148,136)';
    const accent    = 'rgb(79,70,229)';
    const warn      = 'rgb(217,119,6)';
    const plat      = '#a78bfa';
    const stor      = '#64748b';

    // Icon SVG data: { d: path, vb: viewBox, fill?: override color }
    type Icon = { d: string; vb: string; fill?: string };
    const ico: Record<string, Icon> = {
      // material-icon-theme:claude
      claude: { d: 'm14.375 6.48l.49.28v.209l-.14.489l-5.937 1.397l-.558-1.387zm0 0M12.155 2.373l.683.143l.182.224l.173.535l-.072.342l-3.983 5.447L7.81 7.737l3.673-4.82zM8.719 1.522l.419-.28l.349.14l.349.49l-.957 5.748l-.65-.441l-.279-.769l.49-4.33zM4.239 1.614l.43-.55L4.95 1l.558.081l.275.216l2.004 4.442l.724 2.11l-.848.471l-3.231-5.864zM2.154 4.665l-.14-.56l.42-.488l.488.07h.14l2.933 2.165l.908.698l1.257.978l-.698 1.187l-.629-.489l-.419-.419l-4.05-2.863zM1.316 8.296L1 7.946v-.31l.316-.108l3.562.21l3.491.279l-.113.695l-6.66-.346zM3.411 11.931h-.698l-.278-.32v-.382l1.186-.838l4.82-3.068l.487.833zM4.738 13.883l-.28.07l-.418-.21l.07-.35l4.12-5.446l.558.768l-3.072 4.05zM8.23 14.581l-.21.28l-.419.14l-.349-.28l-.21-.42L8.09 8.646l.629.07zM11.791 13.045v.558l-.07.21l-.279.14l-.489-.066l-3.356-4.996l1.331-1.014l1.117 2.025l.105.733zM13.398 12.207l.07.349l-.21.279l-.21-.07l-1.187-.838l-1.815-1.606l-1.397-.978l.419-1.326l.698.419l.42.768zM12.49 8.645l1.746.14l.419.28l.279.418v.302l-.768.327l-3.911-.978l-1.606-.07l.419-1.466l1.117.838z', vb: '0 0 16 16', fill: '#ff7043' },
      // devicon-plain:cursor
      cursor: { d: 'M117.9 30.289L66.664.713a5.32 5.32 0 0 0-5.323 0L10.09 30.29a4.48 4.48 0 0 0-2.234 3.872v59.663c0 1.6.853 3.077 2.24 3.878l51.24 29.586a5.33 5.33 0 0 0 5.324 0l51.246-29.586a4.48 4.48 0 0 0 2.24-3.878V34.166a4.48 4.48 0 0 0-2.24-3.872zm-3.216 6.272l-49.47 85.681c-.337.576-1.217.341-1.217-.325V65.81a3.15 3.15 0 0 0-1.573-2.72l-48.59-28.055c-.571-.331-.336-1.216.33-1.216h98.94c1.409 0 2.284 1.525 1.58 2.741', vb: '0 0 128 128' },
      // simple-icons:windsurf
      windsurf: { d: 'M23.55 5.067a2.177 2.177 0 0 0-2.18 2.177v4.867a1.77 1.77 0 0 1-1.76 1.76a1.82 1.82 0 0 1-1.472-.766l-4.971-7.1a2.2 2.2 0 0 0-1.81-.942c-1.134 0-2.154.964-2.154 2.153v4.896c0 .972-.797 1.76-1.76 1.76c-.57 0-1.136-.287-1.472-.766L.408 5.16A.224.224 0 0 0 0 5.288v4.245c0 .215.066.423.188.6l5.475 7.818c.324.462.8.805 1.351.93a2.164 2.164 0 0 0 2.645-2.098V11.89c0-.972.787-1.76 1.76-1.76h.002a1.8 1.8 0 0 1 1.472.766l4.972 7.1a2.172 2.172 0 0 0 3.96-1.212v-4.895a1.76 1.76 0 0 1 1.76-1.76h.195a.22.22 0 0 0 .22-.22V5.287a.22.22 0 0 0-.22-.22Z', vb: '0 0 24 24', fill: '#00b4d8' },
      // Sensei hexagon (from desktop icon)
      sensei: { d: 'M12 2L3 7v10l9 5l9-5V7z', vb: '0 0 24 24', fill: warn },
      // Folder
      folder: { d: 'M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v8a2 2 0 01-2 2H4a2 2 0 01-2-2V6z', vb: '0 0 20 20' },
      // Kuzu — simplified graph icon (no official SVG available)
      kuzu: { d: 'M12 2a10 10 0 100 20 10 10 0 000-20zm0 3a7 7 0 110 14 7 7 0 010-14zm0 3a4 4 0 100 8 4 4 0 010-8z', vb: '0 0 24 24', fill: '#f97316' },
      // SQLite — database icon
      sqlite: { d: 'M4 4a2 2 0 00-2 2v12a2 2 0 002 2h16a2 2 0 002-2V6a2 2 0 00-2-2H4zm0 2h16v3c-1.5 1-5 1.5-8 1.5S5.5 10 4 9V6zm0 5c1.5 1 5 1.5 8 1.5s6.5-.5 8-1.5v3c-1.5 1-5 1.5-8 1.5S5.5 15 4 14v-3zm0 5c1.5 1 5 1.5 8 1.5s6.5-.5 8-1.5v2H4v-2z', vb: '0 0 24 24', fill: '#0f9fd8' },
    };

    interface N { id: string; label: string; x: number; y: number; color: string; w: number; icon?: string }
    interface L { source: string; target: string; label?: string }

    const nodes: N[] = [
      { id: 'claude',   label: 'Claude Code',    x: col1, y: 60,  color: plat, w: 125, icon: 'claude' },
      { id: 'cursor',   label: 'Cursor',         x: col1, y: 110, color: plat, w: 125, icon: 'cursor' },
      { id: 'windsurf', label: 'Windsurf',       x: col1, y: 160, color: plat, w: 125, icon: 'windsurf' },
      { id: 'mcp',      label: 'sensei-mcp',     x: col2, y: 110, color: secondary, w: 115 },
      { id: 'desktop',  label: 'Sensei Desktop', x: col1, y: 280, color: warn, w: 135, icon: 'sensei' },
      { id: 'cli',      label: 'sensei CLI',     x: col1, y: 340, color: accent, w: 125 },
      { id: 'senseid',  label: 'senseid',        x: col3, y: 210, color: primary, w: 110 },
      { id: 'dotsensei',label: '~/.sensei',      x: col4, y: 150, color: stor, w: 110, icon: 'folder' },
      { id: 'kuzu',     label: 'Kuzu',           x: col4, y: 210, color: '#f97316', w: 110, icon: 'kuzu' },
      { id: 'sqlite',   label: 'SQLite',         x: col4, y: 270, color: '#0f9fd8', w: 110, icon: 'sqlite' },
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

      const hasIcon = n.icon && ico[n.icon];
      const textX = hasIcon ? n.x - n.w / 2 + 30 : n.x;
      const anchor = hasIcon ? 'start' : 'middle';

      if (hasIcon) {
        const i = ico[n.icon!];
        const iSize = 16;
        const iX = n.x - n.w / 2 + 8;
        const iY = n.y - iSize / 2;
        const ig = g.append('svg').attr('x', iX).attr('y', iY).attr('width', iSize).attr('height', iSize).attr('viewBox', i.vb);
        // Handle multi-path SVGs (Claude icon has multiple paths)
        i.d.split(/(?=M)/).forEach(segment => {
          if (segment.trim()) ig.append('path').attr('d', segment).attr('fill', i.fill || n.color);
        });
      }

      g.append('text').attr('x', textX).attr('y', n.y + 4).attr('text-anchor', anchor)
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
  <div class="mx-auto grid max-w-5xl grid-cols-2 gap-8 px-8 text-center sm:grid-cols-4">
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
<div id="setup" class="mx-auto max-w-5xl px-8 py-20">
  <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Setup</div>
  <h2 class="mb-3 text-3xl font-bold tracking-tight">Install Sensei</h2>
  <p class="mb-10 text-surface-z5">Download the desktop app — it installs the daemon, CLI, and MCP server on first run.</p>

  <!-- Platform-detected download -->
  <div class="mb-8 rounded-xl border border-primary-z4 bg-primary-z1 p-5">
    <div class="flex items-center justify-between gap-4 flex-wrap">
      <div class="flex items-center gap-3">
        <svg class="w-7 h-7 text-surface-z7 shrink-0" viewBox="{detected.vb}">{@html detected.svg}</svg>
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
        <span class="ml-auto rounded bg-surface-z3 px-2 py-0.5 text-[10px] font-medium text-surface-z5">macOS / Linux</span>
      </div>
      <pre class="rounded-lg bg-surface-z0 p-3 text-xs text-surface-z7 overflow-x-auto"><code>brew install --cask mizukisu/tap/sensei-app</code></pre>
      <p class="mt-2 text-xs text-surface-z5">Or CLI only: <code class="bg-surface-z3 px-1 rounded">brew install mizukisu/tap/sensei</code></p>
    </div>

    <!-- What's included -->
    <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
      <div class="flex items-center gap-2 mb-3">
        <span class="text-lg">⚡</span>
        <span class="font-semibold">Included</span>
      </div>
      <ul class="space-y-1.5 text-xs text-surface-z5">
        <li class="flex items-center gap-2"><span class="text-success-z5">✓</span> Desktop app with setup wizard</li>
        <li class="flex items-center gap-2"><span class="text-success-z5">✓</span> <code class="bg-surface-z3 px-1 rounded">senseid</code> indexer daemon</li>
        <li class="flex items-center gap-2"><span class="text-success-z5">✓</span> <code class="bg-surface-z3 px-1 rounded">sensei</code> CLI</li>
        <li class="flex items-center gap-2"><span class="text-success-z5">✓</span> <code class="bg-surface-z3 px-1 rounded">sensei-mcp</code> server</li>
      </ul>
    </div>

    <!-- All platforms -->
    <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
      <div class="flex items-center gap-2 mb-3">
        <span class="text-lg">📦</span>
        <span class="font-semibold">All downloads</span>
      </div>
      <div class="space-y-1.5 text-xs">
        {#each platforms as p}
          <a href="{RELEASE_BASE}/{p.file}"
             class="flex items-center gap-2 {p.os === detected.os ? 'text-primary-z6 font-semibold' : 'text-surface-z5'} hover:text-primary-z6 transition-colors">
            <svg class="w-3.5 h-3.5 shrink-0" viewBox="{p.vb}">{@html p.svg}</svg> {p.label}
            {#if p.os === detected.os}
              <span class="ml-auto rounded bg-primary-z2 px-1.5 py-0.5 text-[9px] font-bold text-primary-z7">detected</span>
            {/if}
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

<!-- Architecture -->
<div id="architecture" class="mx-auto max-w-5xl px-8 py-20">
  <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">How it works</div>
  <h2 class="mb-3 text-3xl font-bold tracking-tight">Architecture</h2>
  <p class="mb-10 text-surface-z5">AI platforms connect via MCP. Desktop and CLI talk directly over HTTP. All data stays local.</p>

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

<!-- Usage / Reference -->
<div id="usage" class="border-t border-surface-z0 bg-surface-z2">
  <div class="mx-auto max-w-5xl px-8 py-20">
    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Reference</div>
    <h2 class="mb-3 text-3xl font-bold tracking-tight">Usage Guide</h2>
    <p class="mb-10 text-surface-z5">CLI commands, daemon management, and MCP tools.</p>

    <!-- CLI + Daemon side by side -->
    <div class="grid grid-cols-1 gap-5 mb-10 lg:grid-cols-2">
      <div>
        <h3 class="mb-4 text-lg font-bold">CLI Commands</h3>
        {#each cliGroups as group}
          <div class="mb-4 rounded-xl border border-surface-z3 bg-surface-z1 p-4">
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
      <div>
        <h3 class="mb-4 text-lg font-bold">Daemon Management</h3>
        <div class="grid grid-cols-2 gap-3">
          {#each daemonCommands as d}
            <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-3">
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
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {#each mcpTools as t}
        <div class="flex items-start gap-3 rounded-xl border border-surface-z3 bg-surface-z1 p-4">
          <span class="text-lg">{t.icon}</span>
          <div>
            <code class="text-xs font-bold text-primary-z6">{t.tool}</code>
            <p class="text-xs text-surface-z5 mt-0.5">{t.desc}</p>
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<!-- Sponsor -->
<div id="sponsor">
  <div class="mx-auto max-w-5xl px-8 py-20 text-center">
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
  </div>
</div>
