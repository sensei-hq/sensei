<script lang="ts">
  import type { RepoEntry } from '$lib/repos.svelte.js';
  import StatusBadge from './StatusBadge.svelte';

  let { repo, href, onexclude, onadd }: {
    repo: RepoEntry;
    href?: string;
    onexclude?: (repoId: string) => void;
    onadd?: (repoId: string) => void;
  } = $props();

  const pct = $derived(
    repo.filesTotal > 0 ? Math.round((repo.filesCompleted / repo.filesTotal) * 100) : 0
  );
</script>

<div class="group flex items-center gap-3 rounded-lg bg-surface-z2 px-3 py-2.5 hover:bg-surface-z3/60 transition-colors">
  {#if href}
    <a {href} class="flex-1 min-w-0">
      <p class="text-sm font-medium text-surface-z7 truncate">{repo.project.name}</p>
      <p class="text-[10px] text-surface-z4 truncate">{repo.project.path}</p>
    </a>
  {:else}
    <div class="flex-1 min-w-0">
      <p class="text-sm font-medium text-surface-z7 truncate">{repo.project.name}</p>
      <p class="text-[10px] text-surface-z4 truncate">{repo.project.path}</p>
    </div>
  {/if}

  <!-- Progress -->
  {#if repo.indexState === 'indexing' || repo.indexState === 'queued'}
    <div class="flex items-center gap-2 shrink-0">
      {#if repo.indexState === 'indexing' && repo.filesTotal > 0}
        <div class="w-20 h-1.5 rounded-full bg-surface-z3 overflow-hidden">
          <div class="h-full rounded-full bg-primary-z5 transition-all" style="width: {pct}%"></div>
        </div>
        <span class="text-[10px] text-surface-z4 w-8 text-right">{pct}%</span>
      {/if}
      {#if repo.currentFile}
        <span class="text-[10px] text-info-z6 max-w-32 truncate">{repo.currentFile.split('/').pop()}</span>
      {/if}
    </div>
  {/if}

  <!-- Stack tags -->
  {#if repo.project.stack?.length}
    <span class="text-[10px] text-surface-z4 shrink-0 hidden group-hover:inline">
      {repo.project.stack.slice(0, 2).join(', ')}
    </span>
  {/if}

  <!-- Status badge -->
  <StatusBadge status={repo.indexState} variant="index" />

  <!-- Actions (visible on hover) -->
  <div class="shrink-0 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
    {#if onadd}
      <button
        onclick={() => onadd?.(repo.project.repo_id)}
        class="text-surface-z4 hover:text-primary-z6 text-xs"
        title="Add to solution"
      ><span class="i-solar-add-circle-bold-duotone text-sm"></span></button>
    {/if}
    {#if onexclude}
      <button
        onclick={() => onexclude?.(repo.project.repo_id)}
        class="text-surface-z4 hover:text-error-z6 text-xs"
        title="Exclude from indexing"
      >✕</button>
    {/if}
  </div>
</div>
