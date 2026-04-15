<script lang="ts">
  // GitHub release base URL — update when releases exist
  const GITHUB = 'https://github.com/mizukisu/sensei';
  const RELEASE_BASE = `${GITHUB}/releases/latest/download`;

  const downloads = [
    { os: 'macOS (Apple Silicon)', file: 'Sensei_aarch64.dmg',   icon: '🍎' },
    { os: 'macOS (Intel)',         file: 'Sensei_x86_64.dmg',    icon: '🍎' },
    { os: 'Windows',               file: 'Sensei_x86_64-setup.exe', icon: '🪟' },
    { os: 'Linux',                 file: 'sensei_amd64.AppImage', icon: '🐧' },
  ];
</script>

<!-- Landing page -->
<div class="min-h-screen overflow-y-auto bg-surface-z1 text-surface-z8">

  <!-- Nav -->
  <nav class="sticky top-0 z-50 flex items-center justify-between border-b border-surface-z0 bg-surface-z1/95 px-8 h-14">
    <div class="flex items-center gap-2 font-bold text-base select-none">
      <span class="text-lg">⬡</span>
      <span>sensei</span>
    </div>
    <div class="flex items-center gap-5 text-sm">
      <a href="#install" class="text-surface-z5 hover:text-surface-z8 transition-colors">Install</a>
      <a href="#features" class="text-surface-z5 hover:text-surface-z8 transition-colors">Features</a>
      <a href="#sponsor" class="text-surface-z5 hover:text-surface-z8 transition-colors">Sponsor</a>
      <a href={GITHUB} target="_blank" rel="noopener"
         class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-xs font-medium hover:bg-surface-z3 transition-colors">
        <span class="i-solar-github-bold-duotone"></span> GitHub
      </a>
    </div>
  </nav>

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
      <a href="#install"
         class="rounded-xl bg-primary-z5 px-6 py-3 text-sm font-semibold text-white hover:bg-primary-z6 transition-colors">
        Download the app
      </a>
      <a href="#install-cli"
         class="rounded-xl border border-surface-z3 bg-surface-z2 px-6 py-3 text-sm font-medium hover:bg-surface-z3 transition-colors">
        Install CLI →
      </a>
    </div>
  </div>

  <!-- Stats strip -->
  <div class="border-y border-surface-z0 bg-surface-z2 py-10">
    <div class="mx-auto grid max-w-4xl grid-cols-2 gap-8 px-8 text-center sm:grid-cols-4">
      {#each [
        { n: '62%',   l: 'avg FTR improvement'    },
        { n: '$0.18', l: 'median task cost'        },
        { n: '4.2×',  l: 'faster with context'    },
        { n: '38%',   l: 'token savings via cache' },
      ] as stat}
        <div>
          <div class="text-3xl font-extrabold text-surface-z8">{stat.n}</div>
          <div class="mt-1 text-xs font-medium uppercase tracking-wider text-surface-z4">{stat.l}</div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Download the app -->
  <div id="install" class="mx-auto max-w-4xl px-8 py-20">
    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">Download</div>
    <h2 class="mb-3 text-3xl font-bold tracking-tight">Get the Sensei desktop app</h2>
    <p class="mb-10 text-surface-z5">The desktop app includes the project navigator, graph viewer, indexer control, and setup wizard.</p>
    <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {#each downloads as d}
        <a href="{RELEASE_BASE}/{d.file}"
           class="flex flex-col items-start gap-2 rounded-xl border border-surface-z3 bg-surface-z2 p-4 hover:border-primary-z4 hover:bg-primary-z1 transition-all">
          <span class="text-2xl">{d.icon}</span>
          <span class="text-sm font-semibold text-surface-z8">{d.os}</span>
          <span class="font-mono text-[10px] text-surface-z4">{d.file}</span>
        </a>
      {/each}
    </div>
  </div>

  <!-- Install CLI -->
  <div id="install-cli" class="border-t border-surface-z0 bg-surface-z2">
    <div class="mx-auto max-w-4xl px-8 py-20">
      <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">CLI</div>
      <h2 class="mb-3 text-3xl font-bold tracking-tight">Install the Sensei CLI</h2>
      <p class="mb-10 text-surface-z5">One command gets you the CLI and everything you need to initialize repos, run the server, and set up MCP.</p>

      <div class="grid grid-cols-1 gap-6 md:grid-cols-2">
        <!-- Homebrew -->
        <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-6">
          <div class="mb-3 flex items-center gap-2">
            <span class="text-xl">🍺</span>
            <span class="font-semibold text-surface-z8">Homebrew</span>
            <span class="ml-auto rounded bg-surface-z3 px-2 py-0.5 text-[10px] font-medium text-surface-z5">macOS / Linux</span>
          </div>
          <pre class="rounded-lg bg-surface-z0 p-3 text-xs text-surface-z7 overflow-x-auto"><code>brew install mizukisu/tap/sensei</code></pre>
          <p class="mt-2 text-xs text-surface-z4">Updates automatically with <code class="bg-surface-z3 px-1 rounded">brew upgrade</code></p>
        </div>

        <!-- bun / npm -->
        <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-6">
          <div class="mb-3 flex items-center gap-2">
            <span class="text-xl">📦</span>
            <span class="font-semibold text-surface-z8">npm / bun</span>
            <span class="ml-auto rounded bg-surface-z3 px-2 py-0.5 text-[10px] font-medium text-surface-z5">All platforms</span>
          </div>
          <pre class="rounded-lg bg-surface-z0 p-3 text-xs text-surface-z7 overflow-x-auto"><code>bun install -g sensei
# or
npm install -g sensei</code></pre>
        </div>
      </div>

      <!-- Quick start -->
      <div class="mt-8 rounded-xl border border-surface-z3 bg-surface-z1 p-6">
        <p class="mb-3 text-sm font-semibold text-surface-z7">Quick start</p>
        <pre class="rounded-lg bg-surface-z0 p-4 text-xs text-surface-z7 overflow-x-auto"><code>sensei init          # initialize your repo
sensei serve         # start local server + graph indexer
sensei setup --mcp   # register MCP server for Claude Code</code></pre>
      </div>
    </div>
  </div>

  <!-- Features -->
  <div id="features" class="mx-auto max-w-5xl px-8 py-20">
    <div class="mb-3 text-xs font-semibold uppercase tracking-widest text-primary-z6">What sensei does</div>
    <h2 class="mb-3 text-3xl font-bold tracking-tight">The observability layer for AI-assisted development</h2>
    <p class="mb-12 max-w-md text-surface-z5">Like Code Climate for code quality — sensei gives you the metrics to understand and improve how your team uses AI.</p>
    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
      {#each [
        { ico: '🎯', h: 'First-Try-Right Scoring',  p: 'Track how often AI completes tasks on the first attempt. Per-repo, per-agent, per-team. Trending over time.' },
        { ico: '💰', h: 'Cost Intelligence',        p: 'Token-level cost per task, session, and repo. See exactly where money is spent and what drives cache hit rates.' },
        { ico: '📦', h: 'Context Packing',          p: 'Smart token-budgeted context using BM25 + semantic search. Right code, right size, every request.' },
        { ico: '🔄', h: 'Session Continuity',       p: 'Automatic crash recovery and progress snapshots. Never lose context mid-task.' },
        { ico: '📚', h: 'Doc Drift Detection',      p: 'Traceability matrix linking code to docs. Flags stale documentation automatically when code changes.' },
        { ico: '🧩', h: 'Plugin Ecosystem',         p: 'Skills, commands, and hooks that inject guardrails into every Claude Code session. Zero-friction.' },
      ] as f}
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-6">
          <div class="mb-4 text-2xl">{f.ico}</div>
          <h3 class="mb-2 text-sm font-semibold text-surface-z8">{f.h}</h3>
          <p class="text-sm leading-relaxed text-surface-z5">{f.p}</p>
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
        Sensei is open source and free to use. If it saves you time or helps your team ship better software, consider sponsoring the project to keep it going.
      </p>
      <div class="flex flex-wrap justify-center gap-4">
        <a href="{GITHUB}/sponsors"
           target="_blank" rel="noopener"
           class="flex items-center gap-2 rounded-xl bg-pink-500 px-6 py-3 text-sm font-semibold text-white hover:bg-pink-600 transition-colors">
          ❤️ GitHub Sponsors
        </a>
        <a href="https://opencollective.com/sensei-dev"
           target="_blank" rel="noopener"
           class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-6 py-3 text-sm font-medium hover:bg-surface-z2 transition-colors">
          🌐 Open Collective
        </a>
        <a href="https://ko-fi.com/senseidev"
           target="_blank" rel="noopener"
           class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-6 py-3 text-sm font-medium hover:bg-surface-z2 transition-colors">
          ☕ Ko-fi
        </a>
      </div>

      <div class="mt-12 grid grid-cols-1 gap-4 sm:grid-cols-3 text-left">
        {#each [
          { tier: '☕ Coffee',    price: '$5/mo',   perks: ['Supporter badge', 'Thanks in release notes'] },
          { tier: '🚀 Pro',      price: '$25/mo',  perks: ['Priority issues', 'Discord access', 'Early previews'] },
          { tier: '🏢 Team',     price: '$100/mo', perks: ['Logo in README', 'Direct support channel', 'Feature input'] },
        ] as t}
          <div class="rounded-xl border border-surface-z3 bg-surface-z1 p-5">
            <div class="font-semibold text-surface-z8">{t.tier}</div>
            <div class="mt-1 text-2xl font-extrabold text-primary-z6">{t.price}</div>
            <ul class="mt-4 space-y-1">
              {#each t.perks as perk}
                <li class="flex items-center gap-2 text-xs text-surface-z5">
                  <span class="text-success-z5">✓</span> {perk}
                </li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <!-- Footer -->
  <div class="border-t border-surface-z0 py-8 text-center text-xs text-surface-z4">
    <div class="flex flex-wrap justify-center gap-6 mb-4">
      <a href={GITHUB} target="_blank" rel="noopener" class="hover:text-surface-z6 transition-colors">GitHub</a>
      <a href="{GITHUB}/blob/main/LICENSE" target="_blank" rel="noopener" class="hover:text-surface-z6 transition-colors">MIT License</a>
      <a href="{GITHUB}/issues" target="_blank" rel="noopener" class="hover:text-surface-z6 transition-colors">Issues</a>
      <a href="{GITHUB}/releases" target="_blank" rel="noopener" class="hover:text-surface-z6 transition-colors">Releases</a>
    </div>
    <span class="font-bold text-surface-z6">⬡ sensei</span> · AI Development Intelligence · MIT License
  </div>

</div>
