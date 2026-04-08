<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  const STATUS_CLS: Record<string, string> = {
    active:   'bg-success-z2 text-success-z7',
    recent:   'bg-primary-z2 text-primary-z7',
    stale:    'bg-warning-z2 text-warning-z7',
    archived: 'bg-surface-z3 text-surface-z5',
    unknown:  'bg-surface-z3 text-surface-z5',
  };
</script>

<div class="flex h-full flex-col min-h-0">
  <div class="flex items-center justify-between border-b border-surface-z0/50 px-4 py-2 shrink-0">
    <h1 class="text-sm font-semibold text-surface-z8">Libraries</h1>
    <span class="text-xs text-surface-z4">{data.libraries.length} repo{data.libraries.length !== 1 ? 's' : ''}</span>
  </div>

  <div class="flex-1 overflow-y-auto px-4 py-4">
    {#if data.libraries.length === 0}
      <div class="flex flex-col items-center justify-center h-full gap-3 text-center">
        <div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-surface-z3">
          <span class="i-solar-box-bold-duotone text-2xl text-info-z6"></span>
        </div>
        <div>
          <p class="text-sm font-medium text-surface-z6">No libraries yet</p>
          <p class="mt-1 text-xs text-surface-z4">Import your repos from setup — any repo with<br>an <code class="bg-surface-z3 px-1 rounded">exports</code>, <code class="bg-surface-z3 px-1 rounded">svelte</code>, or <code class="bg-surface-z3 px-1 rounded">main</code> field is tagged as a library.</p>
        </div>
      </div>
    {:else}
      <div class="grid gap-3 grid-cols-1 xl:grid-cols-2">
        {#each data.libraries as lib}
          <div class="rounded-2xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-3.5 hover:border-surface-z4 hover:bg-surface-z2 transition-all">
            <div class="flex items-start justify-between gap-2 mb-1.5">
              <div class="flex items-center gap-2 min-w-0">
                <span class="i-solar-box-bold-duotone text-base text-info-z6 shrink-0"></span>
                <span class="font-semibold text-surface-z8 truncate">{lib.name}</span>
              </div>
              <span class="shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-medium capitalize {STATUS_CLS[lib.status] ?? STATUS_CLS.unknown}">
                {lib.status}
              </span>
            </div>

            {#if lib.description}
              <p class="text-xs text-surface-z5 line-clamp-2 mb-2">{lib.description}</p>
            {/if}

            <div class="flex items-center gap-1.5 flex-wrap">
              {#each lib.tech_stack.slice(0, 3) as tech}
                <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">{tech}</span>
              {/each}
              {#if lib.client}
                <span class="rounded-full bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">
                  <span class="i-solar-tag-bold-duotone text-[8px] mr-0.5"></span>{lib.client}
                </span>
              {/if}
              {#if lib.last_commit_days != null}
                <span class="ml-auto text-[10px] text-surface-z4">
                  {lib.last_commit_days === 0 ? 'Today' : lib.last_commit_days < 30 ? lib.last_commit_days + 'd ago' : Math.floor(lib.last_commit_days / 30) + 'mo ago'}
                </span>
              {/if}
            </div>

            {#if lib.remote}
              <p class="mt-1.5 text-[10px] font-mono text-surface-z3 truncate">
                {lib.remote.replace(/^.*github\.com[:/]/, '').replace(/\.git$/, '')}
              </p>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
