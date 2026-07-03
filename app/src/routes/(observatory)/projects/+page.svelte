<script lang="ts">
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  import { PageHeader, Eyebrow } from '$lib/components';
  import type { EnrichedProject } from './buckets.js';

  let { data } = $props();

  const totalActive   = $derived(data.buckets.active.length);
  const totalRecent   = $derived(data.buckets.recent.length);
  const totalArchived = $derived(data.buckets.archived.length);

  function ftrPct(v: number): string {
    return `${Math.round(v * 100)}`;
  }

  function sessionsShort(n: number): string {
    return n === 0 ? 'no 7d' : `${n}× 7d`;
  }
</script>

{#snippet card(p: EnrichedProject, big: boolean, dim: boolean)}
  <button
    type="button"
    data-project-card={p.id}
    data-big={big || undefined}
    data-dim={dim || undefined}
    class="flex flex-col gap-2 text-left bg-paper-mute hover:bg-paper-soft border border-paper-edge rounded-[10px] cursor-pointer text-inherit"
    class:opacity-70={dim}
    style={big ? 'padding: 18px 20px; min-height: 120px;' : 'padding: 14px 16px; min-height: 84px;'}
    onclick={() => openProjectWindow(p.id, p.name).catch(console.error)}
  >
    <div class="flex items-center gap-3">
      <span
        class="kanji text-accent"
        style={big ? 'font-size: 26px;' : 'font-size: 18px;'}
      >{p.icon?.value ?? '場'}</span>
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <span
            class="w-2 h-2 rounded-full shrink-0"
            class:bg-warning={p.warn}
            class:bg-success={!p.warn && p.ftr14d >= 0.8}
            class:bg-ink-mute={!p.warn && p.ftr14d < 0.8}
            aria-label={p.warn ? 'warning' : 'ok'}
          ></span>
          <span
            class="text-ink truncate"
            style={big ? 'font-size: 14.5px;' : 'font-size: 13px;'}
          >{p.name}</span>
        </div>
        <div class="font-mono text-xs text-ink-soft mt-1 truncate">
          {p.client ?? '—'}
        </div>
      </div>
      <span class="font-mono text-xs text-ink-soft">{ftrPct(p.ftr14d)}</span>
    </div>
    {#if big}
      <div class="font-mono flex gap-3 text-xs text-ink-soft mt-auto">
        <span>{sessionsShort(p.sessions7d)}</span>
        <span>·</span>
        <span>{p.maturity}</span>
        {#if p.stack?.languages && p.stack.languages.length > 0}
          <span>·</span>
          <span class="truncate">{p.stack.languages.slice(0, 2).join(' / ')}</span>
        {/if}
      </div>
    {/if}
  </button>
{/snippet}

<div class="max-w-[1060px] mx-auto px-7 py-6 pb-16">
  <!-- Header — kanji + workspace eyebrow + running tally -->
  <div class="flex items-center gap-5 mb-6">
    <span class="kanji text-[28px] text-accent">場</span>
    <div>
      <p class="m-0 mb-0.5"><Eyebrow>Workspace</Eyebrow></p>
      <h1 class="display text-[22px] font-normal m-0">
        {totalActive} active · {totalRecent} recent · {totalArchived} archived
      </h1>
    </div>
    <span class="flex-1"></span>
    <span class="font-mono text-xs text-ink-soft py-1 px-2 border border-paper-edge rounded">
      ⌘K to jump
    </span>
  </div>

  {#if totalActive === 0 && totalRecent === 0 && totalArchived === 0}
    <p class="text-sm text-ink-soft opacity-70">No projects yet. Set up a project to get started.</p>
  {:else}
    {#if totalActive > 0}
      <h2 class="display text-[15px] font-normal mt-5 mb-2" data-bucket="active">Active</h2>
      <div class="grid grid-cols-2 gap-3">
        {#each data.buckets.active as p (p.id)}
          {@render card(p, true, false)}
        {/each}
      </div>
    {/if}

    {#if totalRecent > 0}
      <h2 class="display text-[15px] font-normal text-ink-mute mt-5 mb-2" data-bucket="recent">Recent</h2>
      <div class="grid grid-cols-3 gap-3">
        {#each data.buckets.recent as p (p.id)}
          {@render card(p, false, false)}
        {/each}
      </div>
    {/if}

    {#if totalArchived > 0}
      <h2 class="display text-[15px] font-normal text-ink-soft mt-5 mb-2" data-bucket="archived">Archived</h2>
      <div class="grid grid-cols-3 gap-3">
        {#each data.buckets.archived as p (p.id)}
          {@render card(p, false, true)}
        {/each}
      </div>
    {/if}
  {/if}
</div>
