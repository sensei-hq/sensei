<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();
</script>

<div class="flex items-center justify-between mb-6">
  <h1 class="text-2xl font-semibold text-surface-z8">Repos</h1>
  <span class="text-sm text-surface-z5">{data.repos.length} repo{data.repos.length !== 1 ? 's' : ''}</span>
</div>

{#if data.repos.length === 0}
  <div class="flex flex-col items-center justify-center py-16 text-center gap-3">
    <p class="text-surface-z5">No repos indexed yet.</p>
    <code class="text-sm bg-surface-z2 px-3 py-1.5 rounded border border-surface-z3">sensei init</code>
    <p class="text-xs text-surface-z4">Run this in your project directory to get started.</p>
  </div>
{:else}
  <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))">
    {#each data.repos as repo}
      <a
        href="/repos/{repo.id}"
        class="flex flex-col p-4 rounded-lg border border-surface-z3 bg-surface-z1 no-underline transition-colors hover:border-primary-z5 hover:bg-surface-z2 min-h-28"
      >
        <div class="flex items-start justify-between gap-2 mb-1.5">
          <span class="font-semibold text-surface-z8 text-base leading-snug">{repo.name}</span>
          {#if !repo.last_indexed_at}
            <span class="shrink-0 text-xs px-2 py-0.5 rounded-full bg-surface-z3 text-surface-z5">Not indexed</span>
          {/if}
        </div>

        <div class="flex gap-3 text-sm mb-2.5">
          <span class="text-surface-z4">files <span class="font-medium text-surface-z7">{repo.file_count}</span></span>
          <span class="text-surface-z3">·</span>
          <span class="text-surface-z4">symbols <span class="font-medium text-surface-z7">{repo.symbol_count}</span></span>
        </div>

        {#if (repo.stack ?? []).length > 0}
          <div class="flex flex-wrap gap-1 mb-2.5">
            {#each (repo.stack ?? []) as tag}
              <span class="text-xs px-2 py-0.5 rounded-full bg-surface-z2 border border-surface-z3 text-surface-z6">{tag}</span>
            {/each}
          </div>
        {/if}

        <div class="text-xs text-surface-z4 mt-auto pt-1">
          {#if repo.last_indexed_at}
            Indexed {new Date(repo.last_indexed_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })}
          {:else}
            Never indexed
          {/if}
        </div>
      </a>
    {/each}
  </div>
{/if}
