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
      <h1 class="text-xl font-bold text-surface-z8">{data.teamName}</h1>
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
    {#each data.stats as stat}
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
            {#each data.contributors as m}
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
        {#each data.repos as repo}
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
          {#each data.pending as u}
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
