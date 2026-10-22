<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
  import { Toggle } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import { openWorkspaceWindow } from '$lib/stores/windows.svelte.js';
  import { invalidateAll } from '$app/navigation';
  import { PageHeader, Screen, ScreenState } from '$lib/components';
  import { railIcon } from '../observatory-nav';
  import { workspaceStatus, type EnrichedWorkspace } from './buckets.js';
  import { workspacesView, type WorkspaceFilter, type WorkspaceView } from './workspaces-view.svelte.js';
  import WorkspaceCard from './WorkspaceCard.svelte';
  import WorkspaceRow from './WorkspaceRow.svelte';

  let { data } = $props();

  // No buckets means the list could not be read (#288): the page shows that
  // error, never an empty index.
  const buckets = $derived(data.buckets ?? { active: [], dormant: [], archived: [] });
  const allWorkspaces = $derived<EnrichedWorkspace[]>([
    ...buckets.active,
    ...buckets.dormant,
    ...buckets.archived,
  ]);

  const counts = $derived({
    all:      allWorkspaces.length,
    active:   buckets.active.length,
    dormant:  buckets.dormant.length,
    archived: buckets.archived.length,
  });

  // Buckets arrive pre-ordered (active by FTR, then dormant, then archived),
  // and Array.filter is stable, so filtering preserves that order.
  const filtered = $derived.by(() => {
    const q = workspacesView.query.trim().toLowerCase();
    const f = workspacesView.filter;
    return allWorkspaces.filter((p) => {
      if (f !== 'all' && workspaceStatus(p) !== f) return false;
      if (q) {
        const hay = `${p.name} ${p.client ?? ''}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
  });

  function open(id: string, name: string): void {
    openWorkspaceWindow(id, name).catch((e) => console.error(e));
  }

  // Rokkit Toggle options. Toggle reads `label`; extra keys (kanji/count/glyph)
  // are read in the itemContent snippet via proxy.get(). Filter counts are live,
  // so its options are derived.
  const filterOptions = $derived([
    { value: 'all',      label: 'All',      kanji: '全', count: counts.all },
    { value: 'active',   label: 'Active',   kanji: '動', count: counts.active },
    { value: 'dormant',  label: 'Dormant',  kanji: '眠', count: counts.dormant },
    { value: 'archived', label: 'Archived', kanji: '蔵', count: counts.archived },
  ]);

  const viewOptions = [
    { value: 'grid', label: 'grid view', icon: 'i-solar:widget-2-linear' },
    { value: 'list', label: 'list view', icon: 'i-solar:hamburger-menu-linear' },
  ];
</script>

<Screen data-component="workspaces-page">
  {#snippet header()}
    <div class="bg-paper">
      <PageHeader kanji="場" icon={railIcon('/workspaces')} eyebrow="Workspaces" title="All the places you work.">
        {#snippet right()}
          <span class="font-mono text-xs text-ink-soft py-1 px-2 border border-paper-edge rounded">
            ⌘K to jump
          </span>
        {/snippet}
      </PageHeader>
      <!-- Filter chips + view toggle + search -->
      {#if !data.error}
      <div class="py-3 px-7 gap-4 border-b border-paper-edge flex items-center">
        <Toggle
          options={filterOptions}
          value={workspacesView.filter}
          onchange={(v: unknown) => workspacesView.setFilter(v as WorkspaceFilter)}
          label="filter workspaces by status"
        >
          {#snippet itemContent(proxy: ProxyItem)}
            <Kanji char={proxy.get('kanji')} icon={null} size="inherit" tone="inherit" class="text-xs" />
            <span>{proxy.label}</span>
            <!-- Inherit the option's text colour (rokkit sets a contrast-correct colour
                 per selected/unselected state) instead of a hardcoded text-ink-mute,
                 which failed contrast on the SELECTED pill's background (a11y). -->
            <span class="font-mono text-xs">{proxy.get('count')}</span>
          {/snippet}
        </Toggle>
        <span class="flex-1"></span>

        <!-- View switcher: grid 田 / list ≣ -->
        <Toggle
          options={viewOptions}
          value={workspacesView.view}
          onchange={(v: unknown) => workspacesView.setView(v as WorkspaceView)}
          label="switch between grid and list"
        >
          {#snippet itemContent(proxy: ProxyItem)}
            <span class="{proxy.get('icon')} text-base" aria-hidden="true"></span>
          {/snippet}
        </Toggle>

        <div class="flex items-center bg-paper-soft border border-paper-edge rounded gap-2 py-1 px-2 min-w-[260px]">
          <Kanji char="探" size="inherit" tone="inherit" class="text-xs text-ink-soft" />
          <input
            type="text"
            bind:value={workspacesView.query}
            placeholder="search workspaces or clients…"
            aria-label="search workspaces"
            class="border-none outline-none bg-transparent text-sm flex-1 text-ink placeholder:text-ink-faint"
          />
          {#if workspacesView.query}
            <button
              type="button"
              onclick={() => workspacesView.clearQuery()}
              class="text-xs text-ink-faint hover:text-ink"
              aria-label="clear search"
            >×</button>
          {/if}
        </div>
        <span class="font-mono text-xs text-ink-soft whitespace-nowrap">
          {filtered.length} of {counts.all}
        </span>
      </div>

      {/if}
    </div>
  {/snippet}
  <div class="bg-paper min-h-full pt-5 pb-6 px-7">
    {#if data.error}
      <ScreenState status="error" error={data.error} onretry={() => invalidateAll()} />
    {:else if counts.all === 0}
      <p class="text-sm text-ink-soft opacity-70 py-6 text-center">
        No workspaces yet. Set up a workspace to get started.
      </p>
    {:else if filtered.length === 0}
      <p class="text-sm text-ink-soft py-6 text-center">
        {workspacesView.query ? `No workspaces match “${workspacesView.query}”.` : 'No workspaces in this bucket.'}
      </p>
    {:else if workspacesView.view === 'grid'}
      <div class="grid grid-cols-3 gap-3 items-start">
        {#each filtered as p (p.id)}
          <WorkspaceCard {p} onOpen={open} />
        {/each}
      </div>
    {:else}
      <div class="border border-paper-edge rounded-lg overflow-hidden bg-paper-soft">
        {#each filtered as p, i (p.id)}
          <WorkspaceRow {p} onOpen={open} last={i === filtered.length - 1} />
        {/each}
      </div>
    {/if}
  </div>
</Screen>
