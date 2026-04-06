<script lang="ts">
  let { data } = $props();

  function ftrColor(pct: number) {
    if (pct >= 90) return 'text-success-z6';
    if (pct >= 70) return 'text-warning-z6';
    return 'text-error-z6';
  }

  /** SVG stroke-dashoffset for ring with r=34 (circumference ≈ 213.6) */
  function ringOffset(pct: number) {
    return (213.6 * (100 - pct)) / 100;
  }
</script>

<div class="p-4 sm:p-6 lg:p-7">

  <!-- Header -->
  <div class="mb-6 flex flex-wrap items-start justify-between gap-3">
    <div>
      <h1 class="text-xl font-bold text-surface-z8">{data.userName}</h1>
      <p class="mt-0.5 text-sm text-surface-z5">{data.orgName} · Developer · {data.email}</p>
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
                    stroke-dashoffset="{ringOffset(data.ftr)}"/>
          </svg>
          <div class="absolute inset-0 flex flex-col items-center justify-center">
            <span class="text-lg font-bold text-surface-z8">{data.ftr}%</span>
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

    {#each data.stats as stat}
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
        {#each data.repos as repo}
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
          {#each data.setupSteps as step, i}
            <div class="relative flex flex-1 flex-col items-center gap-1.5">
              {#if i < data.setupSteps.length - 1}
                <div class="absolute left-1/2 right-0 top-3.5 h-px {i < data.setupDoneUpTo ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
              {/if}
              <div class="relative z-10 flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold
                          {i < data.setupDoneUpTo
                            ? 'border-2 border-success-z6 bg-success-z1 text-success-z7'
                            : i === data.setupDoneUpTo
                              ? 'border-2 border-primary-z6 bg-primary-z1 text-primary-z7'
                              : 'border-2 border-surface-z3 bg-surface-z1 text-surface-z4'}">
                {i < data.setupDoneUpTo ? '✓' : i + 1}
              </div>
              <span class="text-center text-[10px] text-surface-z4 {i === data.setupDoneUpTo ? 'text-surface-z6' : ''}">{step}</span>
            </div>
          {/each}
        </div>
        <p class="mb-1.5 text-sm font-semibold text-primary-z7">Step 4 — Link your account</p>
        <p class="mb-3 text-xs text-surface-z5">Run this in your terminal to connect local sensei to your account:</p>
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
          {#each data.sessions as s}
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
