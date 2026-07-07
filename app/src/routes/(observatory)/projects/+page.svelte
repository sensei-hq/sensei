<script lang="ts">
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  import { Eyebrow } from '$lib/components';
  import type { EnrichedProject } from './buckets.js';

  let { data } = $props();

  type Status = 'all' | 'active' | 'dormant' | 'archived';

  let status = $state<Status>('all');
  let query  = $state('');

  const allProjects = $derived<EnrichedProject[]>([
    ...data.buckets.active,
    ...data.buckets.recent,
    ...data.buckets.archived,
  ]);

  const counts = $derived({
    all:      allProjects.length,
    active:   data.buckets.active.length,
    dormant:  data.buckets.recent.length,
    archived: data.buckets.archived.length,
  });

  function isActive(p: EnrichedProject): boolean {
    return p.sessions7d > 0;
  }
  function isArchived(p: EnrichedProject): boolean {
    return (p as { archived?: boolean }).archived === true;
  }
  function bucketOrder(p: EnrichedProject): number {
    if (isArchived(p)) return 2;
    if (isActive(p))   return 0;
    return 1;
  }

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const out = allProjects.filter((p) => {
      if (status === 'active'   && !isActive(p))   return false;
      if (status === 'dormant'  && (isActive(p) || isArchived(p))) return false;
      if (status === 'archived' && !isArchived(p)) return false;
      if (q) {
        const hay = `${p.name} ${p.client ?? ''}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
    out.sort((a, b) => {
      const ord = bucketOrder(a) - bucketOrder(b);
      if (ord !== 0) return ord;
      return b.ftr14d - a.ftr14d || a.name.localeCompare(b.name);
    });
    return out;
  });

  function ftrPct(v: number): string {
    return `${Math.round(v * 100)}`;
  }

  const chips: Array<{ v: Status; label: string; kanji: string }> = [
    { v: 'all',      label: 'All',      kanji: '全' },
    { v: 'active',   label: 'Active',   kanji: '動' },
    { v: 'dormant',  label: 'Dormant',  kanji: '眠' },
    { v: 'archived', label: 'Archived', kanji: '蔵' },
  ];
</script>

{#snippet card(p: EnrichedProject)}
  {@const active   = isActive(p)}
  {@const archived = isArchived(p)}
  <button
    type="button"
    data-project-card={p.id}
    data-status={archived ? 'archived' : active ? 'active' : 'dormant'}
    class="flex flex-col gap-2 text-left bg-paper-mute hover:bg-paper-soft border border-paper-edge rounded-[8px] cursor-pointer text-inherit p-3"
    class:opacity-60={archived}
    onclick={() => openProjectWindow(p.id, p.name).catch(console.error)}
  >
    <div class="flex items-center gap-2">
      <span class="kanji text-accent text-[22px] leading-none w-6 text-center shrink-0">
        {p.icon?.value ?? '場'}
      </span>
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <span
            class="w-[7px] h-[7px] rounded-full shrink-0"
            class:bg-warning={p.warn}
            class:bg-success={!p.warn && p.ftr14d >= 0.8}
            class:bg-ink-mute={!p.warn && p.ftr14d < 0.8}
            aria-label={p.warn ? 'warning' : 'ok'}
          ></span>
          <span class="text-[13px] text-ink truncate">{p.name}</span>
        </div>
        <div class="font-mono text-[11px] text-ink-soft mt-1 truncate">
          {p.client ?? '—'}
        </div>
      </div>
      {#if !active}
        <span class="font-mono text-[11px] text-ink-soft uppercase tracking-[0.12em]">
          {archived ? 'archived' : 'dormant'}
        </span>
      {/if}
    </div>

    {#if active}
      <div class="grid grid-cols-3 gap-2 pt-1 border-t border-paper-edge">
        <div>
          <div class="text-[11px] uppercase tracking-[0.12em] text-ink-soft">FTR</div>
          <div
            class="display text-[15px] font-normal leading-tight"
            class:text-warning={p.warn}
            class:text-ink={!p.warn}
          >{ftrPct(p.ftr14d)}</div>
        </div>
        <div>
          <div class="text-[11px] uppercase tracking-[0.12em] text-ink-soft">7d</div>
          <div class="display text-[15px] font-normal leading-tight text-ink">{p.sessions7d}</div>
        </div>
        <div>
          <div class="text-[11px] uppercase tracking-[0.12em] text-ink-soft">stack</div>
          <div class="display text-[13px] font-normal leading-tight text-ink truncate">
            {p.stack?.languages?.slice(0, 2).join(' / ') ?? '—'}
          </div>
        </div>
      </div>
    {:else}
      <div class="font-mono text-[11px] text-ink-faint pt-1 border-t border-paper-edge flex justify-between gap-2">
        <span class="truncate">{p.stack?.languages?.slice(0, 2).join(' · ') ?? p.maturity}</span>
        <span>{p.maturity}</span>
      </div>
    {/if}
  </button>
{/snippet}

<div class="w-full h-full flex flex-col bg-paper overflow-hidden">
  <!-- Header -->
  <div class="pt-5 pb-4 px-7 border-b border-paper-edge flex items-center gap-5">
    <span class="kanji text-[28px] text-accent">場</span>
    <div>
      <p class="m-0 mb-0.5"><Eyebrow>Projects</Eyebrow></p>
      <h1 class="display text-[22px] font-normal m-0">All the places you work.</h1>
    </div>
    <span class="flex-1"></span>
    <span class="font-mono text-[11px] text-ink-soft py-1 px-2 border border-paper-edge rounded">
      ⌘K to jump
    </span>
  </div>

  <!-- Filter chips + search -->
  <div class="py-3 px-7 gap-4 border-b border-paper-edge flex items-center">
    <div class="flex gap-1">
      {#each chips as c (c.v)}
        {@const on = status === c.v}
        {@const n  = counts[c.v]}
        <button
          type="button"
          data-chip={c.v}
          data-active={on || undefined}
          onclick={() => (status = c.v)}
          class="py-1 px-3 gap-2 text-[11px] rounded inline-flex items-center transition-colors"
          class:bg-ink={on}
          class:text-paper={on}
          class:text-ink-mute={!on}
        >
          <span class="kanji text-[11px]">{c.kanji}</span>
          {c.label}
          <span
            class="font-mono text-[11px]"
            class:text-paper={on}
            class:text-ink-faint={!on}
          >{n}</span>
        </button>
      {/each}
    </div>
    <span class="flex-1"></span>
    <div class="flex items-center bg-paper-soft border border-paper-edge rounded gap-2 py-1 px-2 min-w-[260px]">
      <span class="kanji text-[11px] text-ink-soft">探</span>
      <input
        type="text"
        bind:value={query}
        placeholder="search projects or clients…"
        aria-label="search projects"
        class="border-none outline-none bg-transparent text-[13px] flex-1 text-ink placeholder:text-ink-faint"
      />
      {#if query}
        <button
          type="button"
          onclick={() => (query = '')}
          class="text-[11px] text-ink-faint hover:text-ink"
          aria-label="clear search"
        >×</button>
      {/if}
    </div>
    <span class="font-mono text-[11px] text-ink-soft whitespace-nowrap">
      {filtered.length} of {counts.all}
    </span>
  </div>

  <!-- Grid -->
  <main class="flex-1 overflow-auto pt-5 pb-6 px-7">
    {#if counts.all === 0}
      <p class="text-sm text-ink-soft opacity-70 py-6 text-center">
        No projects yet. Set up a project to get started.
      </p>
    {:else if filtered.length === 0}
      <p class="text-[13px] text-ink-soft py-6 text-center">
        {query ? `No projects match “${query}”.` : 'No projects in this bucket.'}
      </p>
    {:else}
      <div class="grid grid-cols-3 gap-3 items-start">
        {#each filtered as p (p.id)}
          {@render card(p)}
        {/each}
      </div>
    {/if}
  </main>
</div>
