<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();
</script>

<div class="flex items-center justify-between mb-6">
  <h1 class="text-2xl font-semibold text-surface-z8">Dashboard</h1>
</div>

<!-- Stats row -->
<div class="grid gap-3 mb-8" style="grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); max-width: 320px">
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
    <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.repoCount}</div>
    <div class="text-xs uppercase tracking-wider text-surface-z5">Repos</div>
  </div>
  <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
    <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.recentEvents.length}</div>
    <div class="text-xs uppercase tracking-wider text-surface-z5">Events</div>
  </div>
</div>

<!-- Recent activity -->
{#if data.recentEvents.length > 0}
  <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">Recent Activity</h2>
  <div class="rounded-lg border border-surface-z3 overflow-hidden mb-8">
    <table class="w-full border-collapse text-sm">
      <thead>
        <tr class="bg-surface-z2 border-b border-surface-z3">
          <th class="text-left px-3.5 py-2 text-xs font-semibold uppercase tracking-wider text-surface-z5">Tool</th>
          <th class="text-left px-3.5 py-2 text-xs font-semibold uppercase tracking-wider text-surface-z5">Phase</th>
          <th class="text-left px-3.5 py-2 text-xs font-semibold uppercase tracking-wider text-surface-z5">Time</th>
        </tr>
      </thead>
      <tbody>
        {#each data.recentEvents as event}
          <tr class="border-b border-surface-z2 last:border-b-0">
            <td class="px-3.5 py-2 font-mono text-xs text-surface-z7">{event.tool}</td>
            <td class="px-3.5 py-2 text-surface-z7">{event.phase}</td>
            <td class="px-3.5 py-2 text-surface-z5 text-xs">{new Date(event.ts).toLocaleTimeString()}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<!-- Recent repos -->
{#if data.repos.length > 0}
  <div class="flex items-center justify-between mb-3">
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Recent Repos</h2>
    <a href="/repos" class="text-xs text-primary-z6 hover:text-primary-z7 transition-colors">View all →</a>
  </div>
  <div class="rounded-lg border border-surface-z3 overflow-hidden">
    {#each data.repos as repo}
      <a
        href="/repos"
        class="flex items-center justify-between px-3.5 py-2.5 no-underline border-b border-surface-z2 last:border-b-0 hover:bg-surface-z2 transition-colors"
      >
        <span class="font-medium text-surface-z8 text-sm">{repo.name}</span>
        <div class="flex items-center gap-3">
          {#if (repo.stack ?? []).length > 0}
            <span class="text-xs text-surface-z5">{(repo.stack ?? []).join(', ')}</span>
          {/if}
          <span class="text-xs text-surface-z4">
            {repo.last_indexed_at
              ? new Date(repo.last_indexed_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
              : 'Not indexed'}
          </span>
        </div>
      </a>
    {/each}
  </div>
{/if}
