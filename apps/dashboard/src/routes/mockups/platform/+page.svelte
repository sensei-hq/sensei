<script lang="ts">
  import { ThemeSwitcherToggle } from '@rokkit/app';

  type View = 'dev' | 'org' | 'platform';
  let view = $state<View>('dev');

  // ── helpers ─────────────────────────────────────────────────────
  function ftrColor(pct: number) {
    if (pct >= 90) return 'text-success-z6';
    if (pct >= 70) return 'text-warning-z6';
    return 'text-error-z6';
  }

  function ftrBadgeClass(pct: number) {
    if (pct >= 90) return 'bg-success-z1 text-success-z7 border-success-z3';
    if (pct >= 70) return 'bg-warning-z1 text-warning-z7 border-warning-z3';
    return 'bg-error-z1 text-error-z7 border-error-z3';
  }

  /** SVG stroke-dashoffset for a ring with r=34 (circumference ≈ 213.6) */
  function ringOffset(pct: number) {
    return (213.6 * (100 - pct)) / 100;
  }

  // ── static mock data ─────────────────────────────────────────────
  const devStats = [
    { label: 'Sessions (30d)', value: '412', delta: '↑ 38 vs prev', up: true },
    { label: 'My Cost (30d)',  value: '$58',  delta: '↑ $8 (more sessions)', up: false },
    { label: 'Cache Hit Rate', value: '44%', delta: '↑ 6pp vs prev',        up: true },
  ];

  const devRepos = [
    { icon: '📦', name: 'api-service',    lang: 'TypeScript', symbols: 4201, indexed: '2h ago',  ftr: 89, cost: '$0.11' },
    { icon: '🖥',  name: 'web-dashboard', lang: 'SvelteKit',  symbols: 2840, indexed: '1d ago',  ftr: 74, cost: '$0.19' },
  ];

  const recentSessions = [
    { task: 'Add rate limiting to auth endpoints', repo: 'api-service',   status: 'completed', result: 'First try', cost: '$0.12', when: '2h ago' },
    { task: 'Fix pagination bug in /users list',   repo: 'api-service',   status: 'completed', result: 'First try', cost: '$0.08', when: '5h ago' },
    { task: 'Refactor dashboard chart components', repo: 'web-dashboard', status: 'partial',   result: '2 attempts', cost: '$0.34', when: '1d ago' },
  ];

  const setupSteps = ['Install CLI', 'Init repo', 'MCP setup', 'Link account', 'Plugin'];
  const setupDoneUpTo = 3; // steps 0-2 are done, step 3 is active

  const orgStats = [
    { label: 'Team FTR (30d)',   value: '84%', delta: '↑ 7pp vs last month', up: true  },
    { label: 'Active Members',   value: '9/12', delta: '3 pending invite',    up: null  },
    { label: 'Total Cost (30d)', value: '$327', delta: '↑ $42 vs last month', up: false },
    { label: 'Repos Indexed',    value: '8',    delta: '↑ 2 new this month',  up: true  },
  ];

  const topContributors = [
    { initials: 'BK', name: 'Bob Kim',    ftr: 91, sessions: 412, cost: '$58' },
    { initials: 'CS', name: 'Carol Singh', ftr: 87, sessions: 388, cost: '$62' },
    { initials: 'AC', name: 'Alice Chen',  ftr: 79, sessions: 201, cost: '$31' },
  ];

  const orgRepos = [
    { icon: '📦', name: 'api-service',    desc: '4,201 symbols · indexed 2h ago',  ftr: 89, badge: 'FTR 89%',    badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
    { icon: '🖥',  name: 'web-dashboard', desc: '2,840 symbols · indexed 1d ago',   ftr: 71, badge: 'FTR 71%',    badgeCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
    { icon: '🔧', name: 'data-pipeline',  desc: '1,120 symbols · indexed 5d ago',   ftr:  0, badge: 'Stale index', badgeCls: 'bg-error-z1 text-error-z7 border-error-z3'    },
  ];

  const pendingOnboard = [
    { email: 'dave@acme.com', invited: '2d ago', status: 'Invite sent',                statusCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
    { email: 'eve@acme.com',  invited: '1d ago', status: 'Signed up · setup pending',   statusCls: 'bg-primary-z1 text-primary-z7 border-primary-z3' },
  ];

  const platformStats = [
    { label: 'Active Tenants',  value: '24',     delta: '↑ 3 this month',   up: true  },
    { label: 'Sessions (30d)',  value: '18,432', delta: '↑ 12% vs prev',    up: true  },
    { label: 'Platform Avg FTR', value: '71%',   delta: '↑ 4pp this month', up: true  },
    { label: 'Avg Task Cost',   value: '$0.18',  delta: '↓ $0.02 (cache↑)', up: false },
  ];

  const ftrBuckets = [
    { label: '<40%',  height: 20, colorCls: 'bg-error-z5'   },
    { label: '40–60', height: 45, colorCls: 'bg-warning-z5' },
    { label: '60–80', height: 90, colorCls: 'bg-primary-z5' },
    { label: '80–90', height: 55, colorCls: 'bg-success-z4' },
    { label: '>90%',  height: 25, colorCls: 'bg-success-z6' },
  ];

  const mcpTools = [
    { name: 'context_pack',        pct: 38 },
    { name: 'get_session_context', pct: 24 },
    { name: 'checkpoint',          pct: 15 },
    { name: 'search',              pct: 11 },
    { name: 'record_memory',       pct:  7 },
  ];

  const tenants = [
    { icon: '🏢', name: 'Acme Corp',    desc: 'Team · 12 users · 8 repos',        ftr: 84, cost: '$0.14', sessions: 2341, badge: 'Healthy',  badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
    { icon: '🚀', name: 'DevStudio',    desc: 'Starter · 3 users · 2 repos',      ftr: 61, cost: '$0.31', sessions:  214, badge: 'Improving', badgeCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
    { icon: '🏦', name: 'FinTech Labs', desc: 'Enterprise · 45 users · 23 repos', ftr: 91, cost: '$0.09', sessions: 9812, badge: 'Top 10%',   badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
  ];
</script>

<!-- View switcher pill -->
<div class="fixed bottom-5 right-5 z-50 flex items-center gap-1 rounded-full border border-surface-z4 bg-surface-z2 p-1 shadow-lg text-xs">
  {#each (['dev', 'org', 'platform'] as View[]) as v}
    <button
      class="rounded-full px-3 py-1 font-medium transition-colors
             {view === v ? 'bg-primary-z6 text-white' : 'text-surface-z5 hover:text-surface-z8'}"
      onclick={() => view = v}
    >
      {v === 'dev' ? 'Developer' : v === 'org' ? 'Org Admin' : 'Platform'}
    </button>
  {/each}
  <div class="pl-1 pr-2">
    <ThemeSwitcherToggle />
  </div>
</div>

<!-- Full-screen shell -->
<div class="flex h-full overflow-hidden">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════ -->
  <aside class="flex w-56 shrink-0 flex-col border-r border-surface-z3 bg-surface-z2 overflow-y-auto">

    <!-- Logo -->
    <div class="flex items-center gap-2.5 border-b border-surface-z3 px-4 py-3.5">
      <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary-z6 text-sm font-bold text-white">⬡</div>
      <span class="font-bold text-surface-z8">sensei</span>
    </div>

    <!-- Nav -->
    <nav class="flex-1 space-y-5 px-3 py-4">

      {#if view === 'dev'}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">My workspace</p>
          {#each [
            { icon: '🏠', label: 'Dashboard',    active: true  },
            { icon: '📁', label: 'My Repos',     active: false },
            { icon: '🔄', label: 'Sessions',     active: false },
            { icon: '🧠', label: 'Memory Items', active: false },
            { icon: '💰', label: 'Cost Tracker', active: false },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                           {item.active
                             ? 'bg-primary-z2 font-medium text-primary-z7'
                             : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}">
              <span class="text-base leading-none">{item.icon}</span>
              {item.label}
            </button>
          {/each}
        </div>
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Local setup</p>
          {#each [
            { icon: '⚙️', label: 'CLI Config'    },
            { icon: '🧩', label: 'Plugin Status' },
            { icon: '📡', label: 'MCP Server'    },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
              <span class="text-base leading-none">{item.icon}</span>
              {item.label}
            </button>
          {/each}
        </div>

      {:else if view === 'org'}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Acme Corp</p>
          {#each [
            { icon: '🏠', label: 'Overview',        active: true  },
            { icon: '👥', label: 'Members',         active: false },
            { icon: '📁', label: 'Repos',           active: false },
            { icon: '📊', label: 'Analytics',       active: false },
            { icon: '🧩', label: 'Skills & Config', active: false },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                           {item.active
                             ? 'bg-primary-z2 font-medium text-primary-z7'
                             : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}">
              <span class="text-base leading-none">{item.icon}</span>
              {item.label}
            </button>
          {/each}
        </div>
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Settings</p>
          {#each [
            { icon: '🔑', label: 'API Keys'    },
            { icon: '💳', label: 'Billing'     },
            { icon: '⚙️', label: 'Preferences' },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
              <span class="text-base leading-none">{item.icon}</span>
              {item.label}
            </button>
          {/each}
        </div>

      {:else}
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Platform</p>
          {#each [
            { icon: '🌐', label: 'All Tenants',      active: true,  badge: null },
            { icon: '📊', label: 'Aggregate Stats',  active: false, badge: null },
            { icon: '🔔', label: 'Alerts',           active: false, badge: 3    },
            { icon: '📈', label: 'Benchmarks',       active: false, badge: null },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                           {item.active
                             ? 'bg-primary-z2 font-medium text-primary-z7'
                             : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}">
              <span class="text-base leading-none">{item.icon}</span>
              <span class="flex-1 text-left">{item.label}</span>
              {#if item.badge}
                <span class="rounded-full bg-primary-z6 px-1.5 py-0.5 text-[10px] font-semibold text-white">{item.badge}</span>
              {/if}
            </button>
          {/each}
        </div>
        <div>
          <p class="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-widest text-surface-z4">System</p>
          {#each [
            { icon: '⚙️', label: 'Config'    },
            { icon: '🔑', label: 'API Keys'  },
            { icon: '📋', label: 'Audit Log' },
          ] as item}
            <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
              <span class="text-base leading-none">{item.icon}</span>
              {item.label}
            </button>
          {/each}
        </div>
      {/if}

    </nav>

    <!-- Bottom: user / switch -->
    <div class="border-t border-surface-z3 px-3 py-3">
      <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
        <span class="text-base leading-none">⬅</span>
        {view === 'platform' ? 'Switch tenant' : 'Switch account'}
      </button>
    </div>
  </aside>

  <!-- ══ MAIN CONTENT ═══════════════════════════════════════════════ -->
  <main class="flex-1 overflow-y-auto">

    <!-- ── Developer view ──────────────────────────────────────────── -->
    {#if view === 'dev'}
      <div class="p-7 max-w-5xl">

        <!-- Header -->
        <div class="mb-7 flex items-center justify-between">
          <div>
            <h1 class="text-xl font-bold text-surface-z8">Bob Kim</h1>
            <p class="mt-0.5 text-sm text-surface-z5">Acme Corp · Developer · bob@acme.com</p>
          </div>
          <div class="flex items-center gap-3">
            <div class="flex items-center gap-1.5 text-xs text-success-z6">
              <span class="h-2 w-2 rounded-full bg-success-z5"></span>
              MCP connected
            </div>
            <div class="flex h-8 w-8 items-center justify-center rounded-full bg-primary-z6 text-xs font-bold text-white">BK</div>
          </div>
        </div>

        <!-- FTR + stats row -->
        <div class="mb-6 grid grid-cols-4 gap-4">
          <!-- FTR ring card -->
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
            <div class="flex items-center gap-4">
              <div class="relative h-20 w-20 shrink-0">
                <svg width="80" height="80" viewBox="0 0 80 80" class="-rotate-90">
                  <circle cx="40" cy="40" r="34" fill="none" stroke-width="6" class="stroke-surface-z3"/>
                  <circle cx="40" cy="40" r="34" fill="none" stroke-width="6" stroke-linecap="round"
                          class="stroke-success-z6"
                          stroke-dasharray="213.6"
                          stroke-dashoffset="{ringOffset(91)}"/>
                </svg>
                <div class="absolute inset-0 flex flex-col items-center justify-center">
                  <span class="text-lg font-bold text-surface-z8">91%</span>
                  <span class="text-[9px] uppercase tracking-wide text-surface-z4">FTR</span>
                </div>
              </div>
              <div>
                <p class="text-[10px] uppercase tracking-wide text-surface-z4">First-Try-Right</p>
                <p class="mt-1 text-xs text-surface-z5">Last 30 days</p>
                <p class="mt-1 text-xs text-success-z6">↑ Top of team</p>
              </div>
            </div>
          </div>

          <!-- Stat cards -->
          {#each devStats as stat}
            <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
              <p class="text-[10px] uppercase tracking-wide text-surface-z4">{stat.label}</p>
              <p class="mt-1 text-2xl font-bold text-surface-z8">{stat.value}</p>
              <p class="mt-1 text-xs {stat.up === true ? 'text-success-z6' : stat.up === false ? 'text-error-z6' : 'text-surface-z4'}">
                {stat.delta}
              </p>
            </div>
          {/each}
        </div>

        <!-- Repos + setup row -->
        <div class="mb-6 grid grid-cols-2 gap-5">
          <!-- My repos -->
          <div>
            <p class="mb-3 text-sm font-semibold text-surface-z8">My repos</p>
            <div class="space-y-2">
              {#each devRepos as repo}
                <div class="flex cursor-pointer items-center gap-3 rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3 transition-colors hover:border-surface-z4">
                  <span class="text-xl leading-none">{repo.icon}</span>
                  <div class="flex-1 min-w-0">
                    <p class="text-sm font-semibold text-surface-z8">{repo.name}</p>
                    <p class="text-xs text-surface-z5">{repo.symbols.toLocaleString()} symbols · indexed {repo.indexed} · {repo.lang}</p>
                  </div>
                  <div class="text-right">
                    <p class="text-sm font-bold {ftrColor(repo.ftr)}">{repo.ftr}%</p>
                    <p class="text-[10px] text-surface-z4">FTR</p>
                  </div>
                  <div class="text-right">
                    <p class="text-sm font-bold text-surface-z8">{repo.cost}</p>
                    <p class="text-[10px] text-surface-z4">Avg task</p>
                  </div>
                </div>
              {/each}
              <button class="mt-1 w-full rounded-lg border border-surface-z3 py-2 text-xs text-surface-z5 transition-colors hover:border-surface-z4 hover:text-surface-z7">
                + Add repo
              </button>
            </div>
          </div>

          <!-- Local setup wizard -->
          <div>
            <p class="mb-3 text-sm font-semibold text-surface-z8">Local setup</p>
            <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
              <!-- Step dots -->
              <div class="mb-5 flex items-center">
                {#each setupSteps as step, i}
                  <div class="flex flex-1 flex-col items-center gap-1.5 relative">
                    {#if i < setupSteps.length - 1}
                      <div class="absolute top-3.5 left-1/2 right-0 h-px {i < setupDoneUpTo ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
                    {/if}
                    <div class="relative z-10 flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold
                                {i < setupDoneUpTo
                                  ? 'border-2 border-success-z6 bg-success-z1 text-success-z7'
                                  : i === setupDoneUpTo
                                    ? 'border-2 border-primary-z6 bg-primary-z1 text-primary-z7'
                                    : 'border-2 border-surface-z3 bg-surface-z1 text-surface-z4'}">
                      {i < setupDoneUpTo ? '✓' : i + 1}
                    </div>
                    <span class="text-[10px] text-center text-surface-z4 {i === setupDoneUpTo ? '!text-surface-z6' : ''}">{step}</span>
                  </div>
                {/each}
              </div>

              <p class="mb-1.5 text-sm font-semibold text-primary-z7">Step 4 — Link your account</p>
              <p class="mb-3 text-xs text-surface-z5">Run this in your terminal to connect local sensei to your Acme Corp account:</p>
              <div class="rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-xs leading-relaxed">
                <span class="text-surface-z4"># One-time setup — links this machine to your org</span><br>
                <span class="text-success-z6">sensei login --token bk_a4f2e1c8d9...9c3d</span><br>
                <span class="text-surface-z4"># Sessions will sync automatically after this</span>
              </div>
              <button class="mt-3 rounded-md bg-primary-z6 px-3 py-1.5 text-xs font-semibold text-white transition-colors hover:bg-primary-z7">
                Copy token
              </button>
            </div>
          </div>
        </div>

        <!-- Recent sessions -->
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 overflow-hidden">
          <div class="border-b border-surface-z3 px-5 py-3.5">
            <p class="text-sm font-semibold text-surface-z8">Recent sessions</p>
          </div>
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-surface-z3">
                <th class="px-5 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Task</th>
                <th class="px-5 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Repo</th>
                <th class="px-5 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Status</th>
                <th class="px-5 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Result</th>
                <th class="px-5 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Cost</th>
                <th class="px-5 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">When</th>
              </tr>
            </thead>
            <tbody>
              {#each recentSessions as s}
                <tr class="border-b border-surface-z3 last:border-0 transition-colors hover:bg-surface-z3">
                  <td class="max-w-xs px-5 py-3">
                    <span class="block truncate font-medium text-surface-z8">{s.task}</span>
                  </td>
                  <td class="px-5 py-3 text-xs text-surface-z5">{s.repo}</td>
                  <td class="px-5 py-3">
                    <span class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium
                                 {s.status === 'completed' ? 'bg-success-z1 text-success-z7 border-success-z3' : 'bg-warning-z1 text-warning-z7 border-warning-z3'}">
                      {s.status}
                    </span>
                  </td>
                  <td class="px-5 py-3 text-xs {s.result === 'First try' ? 'text-success-z6' : 'text-warning-z6'}">
                    {s.result === 'First try' ? '✓ ' : ''}{s.result}
                  </td>
                  <td class="px-5 py-3 text-xs text-surface-z7">{s.cost}</td>
                  <td class="px-5 py-3 text-xs text-surface-z4">{s.when}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

      </div>

    <!-- ── Org admin view ───────────────────────────────────────────── -->
    {:else if view === 'org'}
      <div class="p-7 max-w-5xl">

        <!-- Header -->
        <div class="mb-7 flex items-center justify-between">
          <div>
            <h1 class="text-xl font-bold text-surface-z8">Acme Corp</h1>
            <p class="mt-0.5 text-sm text-surface-z5">Organization dashboard · Team plan</p>
          </div>
          <div class="flex items-center gap-3">
            <button class="rounded-lg border border-surface-z3 px-3 py-1.5 text-xs text-surface-z6 transition-colors hover:border-surface-z4 hover:text-surface-z8">
              + Invite member
            </button>
            <div class="flex items-center gap-2">
              <div class="flex h-8 w-8 items-center justify-center rounded-full bg-success-z6 text-xs font-bold text-white">AC</div>
              <div>
                <p class="text-sm font-medium text-surface-z8">Alice Chen</p>
                <p class="text-xs text-surface-z4">Admin</p>
              </div>
            </div>
          </div>
        </div>

        <!-- Stats row -->
        <div class="mb-6 grid grid-cols-4 gap-4">
          {#each orgStats as stat}
            <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
              <p class="text-[10px] uppercase tracking-wide text-surface-z4">{stat.label}</p>
              <p class="mt-1 text-2xl font-bold text-surface-z8">{stat.value}</p>
              <p class="mt-1 text-xs {stat.up === true ? 'text-success-z6' : stat.up === false ? 'text-error-z6' : 'text-surface-z4'}">
                {stat.delta}
              </p>
            </div>
          {/each}
        </div>

        <!-- Contributors + repos row -->
        <div class="mb-6 grid grid-cols-2 gap-5">
          <!-- Top contributors -->
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 overflow-hidden">
            <div class="border-b border-surface-z3 px-5 py-3.5">
              <p class="text-sm font-semibold text-surface-z8">Top contributors by FTR</p>
            </div>
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-surface-z3">
                  <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Member</th>
                  <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">FTR</th>
                  <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Sessions</th>
                  <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Cost</th>
                </tr>
              </thead>
              <tbody>
                {#each topContributors as m}
                  <tr class="border-b border-surface-z3 last:border-0 transition-colors hover:bg-surface-z3">
                    <td class="px-5 py-3">
                      <div class="flex items-center gap-2.5">
                        <div class="flex h-7 w-7 items-center justify-center rounded-full bg-primary-z2 text-xs font-semibold text-primary-z7">
                          {m.initials}
                        </div>
                        <span class="text-surface-z8">{m.name}</span>
                      </div>
                    </td>
                    <td class="px-5 py-3 text-sm font-bold {ftrColor(m.ftr)}">{m.ftr}%</td>
                    <td class="px-5 py-3 text-surface-z7">{m.sessions}</td>
                    <td class="px-5 py-3 text-surface-z7">{m.cost}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          <!-- Repos by health -->
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 overflow-hidden">
            <div class="border-b border-surface-z3 px-5 py-3.5">
              <p class="text-sm font-semibold text-surface-z8">Repos by health</p>
            </div>
            <div class="p-4 space-y-2">
              {#each orgRepos as repo}
                <div class="flex cursor-pointer items-center gap-3 rounded-lg border border-surface-z3 px-4 py-3 transition-colors hover:border-surface-z4">
                  <span class="text-lg leading-none">{repo.icon}</span>
                  <div class="flex-1 min-w-0">
                    <p class="text-sm font-semibold text-surface-z8">{repo.name}</p>
                    <p class="text-xs text-surface-z5">{repo.desc}</p>
                  </div>
                  <span class="shrink-0 inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium {repo.badgeCls}">
                    {repo.badge}
                  </span>
                </div>
              {/each}
            </div>
          </div>
        </div>

        <!-- Pending onboarding -->
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 overflow-hidden">
          <div class="border-b border-surface-z3 px-5 py-3.5">
            <p class="text-sm font-semibold text-surface-z8">Pending onboarding</p>
          </div>
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-surface-z3">
                <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">User</th>
                <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Invited</th>
                <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Status</th>
                <th class="px-5 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4"></th>
              </tr>
            </thead>
            <tbody>
              {#each pendingOnboard as u}
                <tr class="border-b border-surface-z3 last:border-0 transition-colors hover:bg-surface-z3">
                  <td class="px-5 py-3 text-surface-z7">{u.email}</td>
                  <td class="px-5 py-3 text-xs text-surface-z4">{u.invited}</td>
                  <td class="px-5 py-3">
                    <span class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium {u.statusCls}">{u.status}</span>
                  </td>
                  <td class="px-5 py-3">
                    <button class="rounded-md border border-surface-z3 px-2.5 py-1 text-xs text-surface-z5 transition-colors hover:border-surface-z4 hover:text-surface-z8">
                      Resend
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

      </div>

    <!-- ── Platform admin view ──────────────────────────────────────── -->
    {:else}
      <div class="p-7 max-w-5xl">

        <!-- Header -->
        <div class="mb-7 flex items-center justify-between">
          <div>
            <h1 class="text-xl font-bold text-surface-z8">Platform Overview</h1>
            <p class="mt-0.5 text-sm text-surface-z5">Anonymized aggregate · all tenants · last 30 days</p>
          </div>
          <div class="flex items-center gap-2">
            <div class="flex h-8 w-8 items-center justify-center rounded-full bg-primary-z2 text-xs font-bold text-primary-z7">PA</div>
            <div>
              <p class="text-sm font-medium text-surface-z8">Platform Admin</p>
              <span class="rounded bg-primary-z2 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-primary-z7">Platform</span>
            </div>
          </div>
        </div>

        <!-- Anon notice -->
        <div class="mb-6 flex items-center gap-2.5 rounded-lg border border-surface-z3 bg-surface-z2 px-4 py-3 text-xs text-surface-z6">
          <span>🔒</span>
          <span>All tenant data shown here is anonymized. No user PII, no source code, no identifiable symbols are stored or displayed.</span>
        </div>

        <!-- Stats row -->
        <div class="mb-6 grid grid-cols-4 gap-4">
          {#each platformStats as stat}
            <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
              <p class="text-[10px] uppercase tracking-wide text-surface-z4">{stat.label}</p>
              <p class="mt-1 text-2xl font-bold text-surface-z8">{stat.value}</p>
              <p class="mt-1 text-xs {stat.up ? 'text-success-z6' : 'text-error-z6'}">{stat.delta}</p>
            </div>
          {/each}
        </div>

        <!-- FTR distribution + MCP tools row -->
        <div class="mb-6 grid grid-cols-2 gap-5">
          <!-- FTR distribution bar chart -->
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
            <p class="mb-5 text-sm font-semibold text-surface-z8">FTR distribution (all tenants)</p>
            <div class="flex h-28 items-end gap-2">
              {#each ftrBuckets as b}
                <div class="flex flex-1 flex-col items-center gap-2">
                  <div class="w-full rounded-t {b.colorCls} opacity-80 transition-all hover:opacity-100"
                       style="height: {b.height}px"></div>
                  <span class="text-[9px] text-surface-z4">{b.label}</span>
                </div>
              {/each}
            </div>
          </div>

          <!-- Most-used MCP tools -->
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
            <p class="mb-4 text-sm font-semibold text-surface-z8">Most-used MCP tools (platform-wide)</p>
            <div class="space-y-3">
              {#each mcpTools as tool}
                <div>
                  <div class="mb-1 flex items-center justify-between text-xs">
                    <span class="font-mono text-surface-z7">{tool.name}</span>
                    <span class="text-surface-z4">{tool.pct}%</span>
                  </div>
                  <div class="h-1.5 overflow-hidden rounded-full bg-surface-z3">
                    <div class="h-full rounded-full bg-primary-z6 transition-all" style="width: {tool.pct}%"></div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        </div>

        <!-- Tenant list -->
        <div>
          <p class="mb-3 text-sm font-semibold text-surface-z8">Tenants</p>
          <div class="space-y-2">
            {#each tenants as t}
              <div class="flex cursor-pointer items-center gap-4 rounded-xl border border-surface-z3 bg-surface-z2 px-5 py-4 transition-colors hover:border-surface-z4">
                <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-surface-z3 text-xl">
                  {t.icon}
                </div>
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-semibold text-surface-z8">{t.name}</p>
                  <p class="text-xs text-surface-z5">{t.desc}</p>
                </div>
                <div class="flex items-center gap-6">
                  <div class="text-right">
                    <p class="text-base font-bold {ftrColor(t.ftr)}">{t.ftr}%</p>
                    <p class="text-[10px] text-surface-z4">FTR</p>
                  </div>
                  <div class="text-right">
                    <p class="text-base font-bold text-surface-z8">{t.cost}</p>
                    <p class="text-[10px] text-surface-z4">Avg cost</p>
                  </div>
                  <div class="text-right">
                    <p class="text-base font-bold text-surface-z8">{t.sessions.toLocaleString()}</p>
                    <p class="text-[10px] text-surface-z4">Sessions</p>
                  </div>
                  <span class="shrink-0 inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium {t.badgeCls}">{t.badge}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>

      </div>
    {/if}

  </main>
</div>
