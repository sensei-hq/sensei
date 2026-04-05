<script lang="ts">
  import { ThemeSwitcherToggle } from '@rokkit/app';

  type View = 'dev' | 'org' | 'platform';

  let { data } = $props();
  let view = $state<View>('dev');
  let sidebarOpen = $state(false);

  function ftrColor(pct: number) {
    if (pct >= 90) return 'text-success-z6';
    if (pct >= 70) return 'text-warning-z6';
    return 'text-error-z6';
  }

  /** SVG stroke-dashoffset for ring with r=34 (circumference ≈ 213.6) */
  function ringOffset(pct: number) {
    return (213.6 * (100 - pct)) / 100;
  }

  function closeSidebar() { sidebarOpen = false; }
  function switchView(v: View) { view = v; closeSidebar(); }
</script>

<!-- Mobile backdrop -->
{#if sidebarOpen}
  <div
    class="fixed inset-0 z-20 bg-black/60 md:hidden"
    role="presentation"
    onclick={closeSidebar}
  ></div>
{/if}

<!-- View switcher pill (fixed) -->
<div class="fixed bottom-4 right-4 z-50 flex items-center gap-0.5 rounded-full border border-surface-z4 bg-surface-z2 p-1 shadow-xl text-xs">
  {#each (['dev', 'org', 'platform'] as View[]) as v}
    <button
      class="rounded-full px-2.5 py-1 font-medium transition-colors
             {view === v ? 'bg-primary-z6 text-white' : 'text-surface-z5 hover:text-surface-z8'}"
      onclick={() => switchView(v)}
    >
      {v === 'dev' ? 'Dev' : v === 'org' ? 'Org' : 'Platform'}
    </button>
  {/each}
  <div class="pl-1 pr-1.5">
    <ThemeSwitcherToggle />
  </div>
</div>

<!-- App shell -->
<div class="flex h-full overflow-hidden">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════ -->
  <aside class="
    fixed inset-y-0 left-0 z-30 flex w-64 flex-col
    border-r border-surface-z3 bg-surface-z2 overflow-y-auto
    transition-transform duration-200 ease-in-out
    {sidebarOpen ? 'translate-x-0' : '-translate-x-full'}
    md:relative md:w-56 md:shrink-0 md:translate-x-0 md:transition-none
  ">
    <!-- Logo -->
    <div class="flex items-center justify-between border-b border-surface-z3 px-4 py-3.5">
      <div class="flex items-center gap-2.5">
        <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-primary-z6 text-sm font-bold text-white">⬡</div>
        <span class="font-bold text-surface-z8">sensei</span>
      </div>
      <!-- Close button (mobile only) -->
      <button
        class="md:hidden flex h-7 w-7 items-center justify-center rounded-lg text-surface-z5 hover:bg-surface-z3 hover:text-surface-z8"
        onclick={closeSidebar}
        aria-label="Close menu"
      >
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" class="h-4 w-4">
          <path stroke-linecap="round" d="M3 3l10 10M13 3L3 13"/>
        </svg>
      </button>
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
            <button
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {item.active ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
              onclick={closeSidebar}
            >
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
            <button
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {item.active ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
              onclick={closeSidebar}
            >
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
            { icon: '🌐', label: 'All Tenants',     active: true,  badge: null },
            { icon: '📊', label: 'Aggregate Stats', active: false, badge: null },
            { icon: '🔔', label: 'Alerts',          active: false, badge: 3    },
            { icon: '📈', label: 'Benchmarks',      active: false, badge: null },
          ] as item}
            <button
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors
                     {item.active ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
              onclick={closeSidebar}
            >
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

    <div class="border-t border-surface-z3 px-3 py-3">
      <button class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z7">
        <span class="text-base leading-none">⬅</span>
        {view === 'platform' ? 'Switch tenant' : 'Switch account'}
      </button>
    </div>
  </aside>

  <!-- ══ MAIN COLUMN ════════════════════════════════════════════════ -->
  <div class="flex min-w-0 flex-1 flex-col overflow-hidden">

    <!-- Mobile header -->
    <header class="flex h-12 shrink-0 items-center justify-between border-b border-surface-z3 bg-surface-z2 px-4 md:hidden">
      <button
        class="flex h-8 w-8 items-center justify-center rounded-lg text-surface-z5 hover:bg-surface-z3 hover:text-surface-z8"
        onclick={() => sidebarOpen = true}
        aria-label="Open menu"
      >
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" class="h-4 w-4">
          <path stroke-linecap="round" d="M2 4h12M2 8h12M2 12h12"/>
        </svg>
      </button>
      <div class="flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-md bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="font-semibold text-surface-z8 text-sm">sensei</span>
      </div>
      <!-- spacer to balance the hamburger -->
      <div class="w-8"></div>
    </header>

    <!-- Scrollable content -->
    <main class="flex-1 overflow-y-auto">

      <!-- ── Developer view ─────────────────────────────────────── -->
      {#if view === 'dev'}
        <div class="p-4 sm:p-6 lg:p-7">

          <!-- Header -->
          <div class="mb-6 flex flex-wrap items-start justify-between gap-3">
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
          <div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <!-- FTR ring card -->
            <div class="col-span-2 rounded-xl border border-surface-z3 bg-surface-z2 p-4 sm:col-span-1">
              <div class="flex items-center gap-4">
                <div class="relative h-20 w-20 shrink-0">
                  <svg width="80" height="80" viewBox="0 0 80 80" class="-rotate-90">
                    <circle cx="40" cy="40" r="34" fill="none" stroke-width="6" class="stroke-surface-z3"/>
                    <circle cx="40" cy="40" r="34" fill="none" stroke-width="6" stroke-linecap="round"
                            class="stroke-success-z6"
                            stroke-dasharray="213.6"
                            stroke-dashoffset="{ringOffset(data.dev.ftr)}"/>
                  </svg>
                  <div class="absolute inset-0 flex flex-col items-center justify-center">
                    <span class="text-lg font-bold text-surface-z8">{data.dev.ftr}%</span>
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

            {#each data.dev.stats as stat}
              <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
                <p class="text-[10px] uppercase tracking-wide text-surface-z4">{stat.label}</p>
                <p class="mt-1 text-2xl font-bold text-surface-z8">{stat.value}</p>
                <p class="mt-1 text-xs {stat.up === true ? 'text-success-z6' : stat.up === false ? 'text-error-z6' : 'text-surface-z4'}">{stat.delta}</p>
              </div>
            {/each}
          </div>

          <!-- Repos + setup row -->
          <div class="mb-5 grid grid-cols-1 gap-5 md:grid-cols-2">
            <!-- My repos -->
            <div>
              <p class="mb-3 text-sm font-semibold text-surface-z8">My repos</p>
              <div class="space-y-2">
                {#each data.dev.repos as repo}
                  <div class="flex cursor-pointer items-center gap-3 rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3 transition-colors hover:border-surface-z4">
                    <span class="text-xl leading-none">{repo.icon}</span>
                    <div class="min-w-0 flex-1">
                      <p class="text-sm font-semibold text-surface-z8">{repo.name}</p>
                      <p class="truncate text-xs text-surface-z5">{repo.symbols.toLocaleString()} symbols · {repo.indexed} · {repo.lang}</p>
                    </div>
                    <div class="shrink-0 text-right">
                      <p class="text-sm font-bold {ftrColor(repo.ftr)}">{repo.ftr}%</p>
                      <p class="text-[10px] text-surface-z4">FTR</p>
                    </div>
                    <div class="shrink-0 text-right">
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

            <!-- Setup wizard -->
            <div>
              <p class="mb-3 text-sm font-semibold text-surface-z8">Local setup</p>
              <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
                <div class="mb-5 flex items-center">
                  {#each data.dev.setupSteps as step, i}
                    <div class="relative flex flex-1 flex-col items-center gap-1.5">
                      {#if i < data.dev.setupSteps.length - 1}
                        <div class="absolute left-1/2 right-0 top-3.5 h-px {i < data.dev.setupDoneUpTo ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
                      {/if}
                      <div class="relative z-10 flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold
                                  {i < data.dev.setupDoneUpTo
                                    ? 'border-2 border-success-z6 bg-success-z1 text-success-z7'
                                    : i === data.dev.setupDoneUpTo
                                      ? 'border-2 border-primary-z6 bg-primary-z1 text-primary-z7'
                                      : 'border-2 border-surface-z3 bg-surface-z1 text-surface-z4'}">
                        {i < data.dev.setupDoneUpTo ? '✓' : i + 1}
                      </div>
                      <span class="text-center text-[10px] text-surface-z4 {i === data.dev.setupDoneUpTo ? 'text-surface-z6' : ''}">{step}</span>
                    </div>
                  {/each}
                </div>
                <p class="mb-1.5 text-sm font-semibold text-primary-z7">Step 4 — Link your account</p>
                <p class="mb-3 text-xs text-surface-z5">Run this in your terminal to connect local sensei to your Acme Corp account:</p>
                <div class="overflow-x-auto rounded-lg border border-surface-z3 bg-surface-z1 px-4 py-3 font-mono text-xs leading-relaxed">
                  <span class="text-surface-z4"># One-time setup</span><br>
                  <span class="text-success-z6">sensei login --token bk_a4f2e1c8...</span><br>
                  <span class="text-surface-z4"># Sessions sync automatically</span>
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
            <div class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-surface-z3">
                    <th class="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Task</th>
                    <th class="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4 hidden sm:table-cell">Repo</th>
                    <th class="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Status</th>
                    <th class="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4 hidden md:table-cell">Result</th>
                    <th class="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4 hidden md:table-cell">Cost</th>
                    <th class="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">When</th>
                  </tr>
                </thead>
                <tbody>
                  {#each data.dev.sessions as s}
                    <tr class="border-b border-surface-z3 last:border-0 transition-colors hover:bg-surface-z3">
                      <td class="max-w-[180px] px-4 py-3 sm:max-w-xs">
                        <span class="block truncate font-medium text-surface-z8">{s.task}</span>
                      </td>
                      <td class="px-4 py-3 text-xs text-surface-z5 hidden sm:table-cell">{s.repo}</td>
                      <td class="px-4 py-3">
                        <span class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium
                                     {s.status === 'completed' ? 'bg-success-z1 text-success-z7 border-success-z3' : 'bg-warning-z1 text-warning-z7 border-warning-z3'}">
                          {s.status}
                        </span>
                      </td>
                      <td class="px-4 py-3 text-xs hidden md:table-cell {s.result === 'First try' ? 'text-success-z6' : 'text-warning-z6'}">
                        {s.result === 'First try' ? '✓ ' : ''}{s.result}
                      </td>
                      <td class="px-4 py-3 text-xs text-surface-z7 hidden md:table-cell">{s.cost}</td>
                      <td class="px-4 py-3 text-xs text-surface-z4">{s.when}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>

        </div>

      <!-- ── Org admin view ──────────────────────────────────────── -->
      {:else if view === 'org'}
        <div class="p-4 sm:p-6 lg:p-7">

          <!-- Header -->
          <div class="mb-6 flex flex-wrap items-start justify-between gap-3">
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
                <div class="hidden sm:block">
                  <p class="text-sm font-medium text-surface-z8">Alice Chen</p>
                  <p class="text-xs text-surface-z4">Admin</p>
                </div>
              </div>
            </div>
          </div>

          <!-- Stats row -->
          <div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
            {#each data.org.stats as stat}
              <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
                <p class="text-[10px] uppercase tracking-wide text-surface-z4">{stat.label}</p>
                <p class="mt-1 text-2xl font-bold text-surface-z8">{stat.value}</p>
                <p class="mt-1 text-xs {stat.up === true ? 'text-success-z6' : stat.up === false ? 'text-error-z6' : 'text-surface-z4'}">{stat.delta}</p>
              </div>
            {/each}
          </div>

          <!-- Contributors + repos row -->
          <div class="mb-5 grid grid-cols-1 gap-5 md:grid-cols-2">
            <!-- Top contributors -->
            <div class="overflow-hidden rounded-xl border border-surface-z3 bg-surface-z2">
              <div class="border-b border-surface-z3 px-5 py-3.5">
                <p class="text-sm font-semibold text-surface-z8">Top contributors by FTR</p>
              </div>
              <div class="overflow-x-auto">
                <table class="w-full text-sm">
                  <thead>
                    <tr class="border-b border-surface-z3">
                      <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Member</th>
                      <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">FTR</th>
                      <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4 hidden sm:table-cell">Sessions</th>
                      <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4 hidden sm:table-cell">Cost</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each data.org.contributors as m}
                      <tr class="border-b border-surface-z3 last:border-0 transition-colors hover:bg-surface-z3">
                        <td class="px-4 py-3">
                          <div class="flex items-center gap-2.5">
                            <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary-z2 text-xs font-semibold text-primary-z7">
                              {m.initials}
                            </div>
                            <span class="text-surface-z8">{m.name}</span>
                          </div>
                        </td>
                        <td class="px-4 py-3 text-sm font-bold {ftrColor(m.ftr)}">{m.ftr}%</td>
                        <td class="px-4 py-3 text-surface-z7 hidden sm:table-cell">{m.sessions}</td>
                        <td class="px-4 py-3 text-surface-z7 hidden sm:table-cell">{m.cost}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Repos by health -->
            <div class="overflow-hidden rounded-xl border border-surface-z3 bg-surface-z2">
              <div class="border-b border-surface-z3 px-5 py-3.5">
                <p class="text-sm font-semibold text-surface-z8">Repos by health</p>
              </div>
              <div class="space-y-2 p-4">
                {#each data.org.repos as repo}
                  <div class="flex cursor-pointer items-center gap-3 rounded-lg border border-surface-z3 px-4 py-3 transition-colors hover:border-surface-z4">
                    <span class="shrink-0 text-lg leading-none">{repo.icon}</span>
                    <div class="min-w-0 flex-1">
                      <p class="text-sm font-semibold text-surface-z8">{repo.name}</p>
                      <p class="truncate text-xs text-surface-z5">{repo.desc}</p>
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
          <div class="overflow-hidden rounded-xl border border-surface-z3 bg-surface-z2">
            <div class="border-b border-surface-z3 px-5 py-3.5">
              <p class="text-sm font-semibold text-surface-z8">Pending onboarding</p>
            </div>
            <div class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b border-surface-z3">
                    <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">User</th>
                    <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4 hidden sm:table-cell">Invited</th>
                    <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Status</th>
                    <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4"></th>
                  </tr>
                </thead>
                <tbody>
                  {#each data.org.pending as u}
                    <tr class="border-b border-surface-z3 last:border-0 transition-colors hover:bg-surface-z3">
                      <td class="px-4 py-3 text-surface-z7">{u.email}</td>
                      <td class="px-4 py-3 text-xs text-surface-z4 hidden sm:table-cell">{u.invited}</td>
                      <td class="px-4 py-3">
                        <span class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium {u.statusCls}">{u.status}</span>
                      </td>
                      <td class="px-4 py-3">
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

        </div>

      <!-- ── Platform admin view ────────────────────────────────── -->
      {:else}
        <div class="p-4 sm:p-6 lg:p-7">

          <!-- Header -->
          <div class="mb-6 flex flex-wrap items-start justify-between gap-3">
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
          <div class="mb-5 flex items-start gap-2.5 rounded-lg border border-secondary-z3 bg-secondary-z1 px-4 py-3 text-xs text-secondary-z7">
            <span class="shrink-0">🔒</span>
            <span>All tenant data shown here is anonymized. No user PII, no source code, no identifiable symbols are stored or displayed.</span>
          </div>

          <!-- Stats row -->
          <div class="mb-5 grid grid-cols-2 gap-3 lg:grid-cols-4">
            {#each data.platform.stats as stat}
              <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
                <p class="text-[10px] uppercase tracking-wide text-surface-z4">{stat.label}</p>
                <p class="mt-1 text-2xl font-bold text-surface-z8">{stat.value}</p>
                <p class="mt-1 text-xs {stat.up ? 'text-success-z6' : 'text-error-z6'}">{stat.delta}</p>
              </div>
            {/each}
          </div>

          <!-- FTR distribution + MCP tools row -->
          <div class="mb-5 grid grid-cols-1 gap-5 md:grid-cols-2">
            <!-- FTR distribution bar chart -->
            <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
              <p class="mb-5 text-sm font-semibold text-surface-z8">FTR distribution (all tenants)</p>
              <div class="flex h-28 items-end gap-2">
                {#each data.platform.ftrBuckets as b}
                  <div class="flex flex-1 flex-col items-center gap-2">
                    <div class="w-full rounded-t opacity-80 transition-all hover:opacity-100 {b.colorCls}"
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
                {#each data.platform.mcpTools as tool}
                  <div>
                    <div class="mb-1 flex items-center justify-between text-xs">
                      <span class="font-mono text-surface-z7">{tool.name}</span>
                      <span class="text-surface-z4">{tool.pct}%</span>
                    </div>
                    <div class="h-1.5 overflow-hidden rounded-full bg-surface-z3">
                      <div class="h-full rounded-full bg-gradient-to-r from-primary-z6 to-secondary-z5 transition-all" style="width: {tool.pct}%"></div>
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
              {#each data.platform.tenants as t}
                <div class="flex cursor-pointer items-center gap-3 rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-4 transition-colors hover:border-surface-z4 sm:gap-4 sm:px-5">
                  <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-surface-z3 text-xl">
                    {t.icon}
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="text-sm font-semibold text-surface-z8">{t.name}</p>
                    <p class="truncate text-xs text-surface-z5">{t.desc}</p>
                  </div>
                  <div class="flex shrink-0 items-center gap-3 sm:gap-6">
                    <div class="text-right">
                      <p class="text-base font-bold {ftrColor(t.ftr)}">{t.ftr}%</p>
                      <p class="text-[10px] text-surface-z4">FTR</p>
                    </div>
                    <div class="hidden text-right sm:block">
                      <p class="text-base font-bold text-surface-z8">{t.cost}</p>
                      <p class="text-[10px] text-surface-z4">Avg cost</p>
                    </div>
                    <div class="hidden text-right md:block">
                      <p class="text-base font-bold text-surface-z8">{t.sessions.toLocaleString()}</p>
                      <p class="text-[10px] text-surface-z4">Sessions</p>
                    </div>
                    <span class="hidden shrink-0 inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium sm:inline-flex {t.badgeCls}">{t.badge}</span>
                  </div>
                </div>
              {/each}
            </div>
          </div>

        </div>
      {/if}

    </main>
  </div>
</div>
