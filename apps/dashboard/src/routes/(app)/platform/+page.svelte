<script lang="ts">
  let { data } = $props();

  function ftrColor(pct: number) {
    if (pct >= 90) return 'text-success-z6';
    if (pct >= 70) return 'text-warning-z6';
    return 'text-error-z6';
  }
</script>

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
    {#each data.stats as stat}
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
        {#each data.ftrBuckets as b}
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
        {#each data.mcpTools as tool}
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
      {#each data.tenants as t}
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
