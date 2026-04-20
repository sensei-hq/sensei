<script lang="ts">
  import { page } from '$app/stores';
  import { solutionDashboardDummy } from '$lib/observatory/dummy.js';

  let data = $derived(solutionDashboardDummy($page.params.id ?? ''));

  function roleCls(role: string): string {
    const map: Record<string, string> = {
      backend: 'bg-info-z2 text-info-z7', frontend: 'bg-accent-z2 text-accent-z7',
      docs: 'bg-warning-z2 text-warning-z7', monorepo: 'bg-primary-z2 text-primary-z7',
      reference: 'bg-surface-z3 text-surface-z5', infra: 'bg-secondary-z2 text-secondary-z7',
    };
    return map[role] ?? 'bg-surface-z3 text-surface-z5';
  }

  function sourceIcon(type: string): string {
    if (type === 'git') return 'i-solar-code-square-bold-duotone';
    if (type === 'unmanaged') return 'i-solar-folder-bold-duotone';
    return 'i-solar-link-round-bold-duotone';
  }

  function stateBadge(state: string): { cls: string; label: string } {
    if (state === 'active') return { cls: 'bg-success-z2 text-success-z7', label: 'active' };
    if (state === 'recent') return { cls: 'bg-info-z2 text-info-z7', label: 'recent' };
    if (state === 'inactive') return { cls: 'bg-surface-z3 text-surface-z5', label: 'inactive' };
    return { cls: 'bg-surface-z3 text-surface-z4', label: state };
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-5">

  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold text-surface-z8">Sources</h2>
    <button class="rounded-md bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3">
      + Add source
    </button>
  </div>

  <p class="text-xs text-surface-z4">{data.solution.projects.length} sources in {data.solution.name}</p>

  <div class="space-y-1.5">
    {#each data.solution.projects as project (project.id)}
      {@const badge = stateBadge(project.state)}
      <a href="/s/{data.solution.id}/p/{project.id}"
        class="flex items-center gap-3 rounded-lg bg-surface-z2 px-4 py-3 hover:bg-surface-z3/60 transition-colors">
        <span class="text-lg {sourceIcon(project.sourceType)} text-surface-z5"></span>
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium text-surface-z7">{project.name}</span>
            <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {roleCls(project.role)}">{project.role}</span>
          </div>
          <p class="text-[10px] text-surface-z4 mt-0.5">
            {project.sourceType === 'git' ? 'Git repo' : project.sourceType === 'unmanaged' ? 'Folder (no git)' : 'Connector'}
            {#if project.indexedAt}
              &middot; indexed {project.indexedAt.slice(0, 10)}
            {:else}
              &middot; not indexed
            {/if}
          </p>
        </div>
        <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {badge.cls}">{badge.label}</span>
      </a>
    {/each}
  </div>

  <!-- Future connectors placeholder -->
  <div class="rounded-lg border border-dashed border-surface-z3 p-4 text-center">
    <p class="text-xs text-surface-z4">External connectors (Confluence, Jira, Wiki) coming soon</p>
  </div>

</div>
