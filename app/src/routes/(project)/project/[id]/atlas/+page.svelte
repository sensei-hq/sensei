<script lang="ts">
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { Select } from '@rokkit/ui';
  import { EmptyState, Eyebrow } from '$lib/components';
  import {
    kindColor,
    kindNeedsRing,
    scaleRadius,
    legendItems,
    initialLevel,
    type AtlasLevel,
    type RenderNode,
  } from './atlas-graph.svelte.js';
  import { AtlasView } from './atlas-view.svelte.js';
  import AtlasCanvas from './AtlasCanvas.svelte';
  import AtlasInspector from './AtlasInspector.svelte';
  import AtlasLegend from './AtlasLegend.svelte';

  let { data } = $props();

  // Pick the opening granularity once from the first payload; `data` is
  // reactive, but the initial level is a one-shot decision (untracked).
  const view = new AtlasView(
    untrack(() => initialLevel(data.totalSymbols, data.communities.length)),
  );

  // Dropping the focus on a scope change keeps a stale id from lighting up an
  // unrelated node in the new graph.
  $effect(() => {
    data.repoId;
    view.clear();
  });

  const LEVELS: { label: string; value: AtlasLevel }[] = [
    { label: 'Communities', value: 'communities' },
    { label: 'Symbols', value: 'symbols' },
  ];

  // "Hide tests" — swap the symbol view to the production-only one (test-file
  // nodes excluded before the degree cap; the loader precomputes both).
  let hideTests = $state(false);
  const active = $derived(
    hideTests
      ? data.code
      : { symbols: data.symbols, links: data.links, totalSymbols: data.totalSymbols },
  );

  const empty = $derived(data.communities.length === 0 && data.symbols.length === 0);

  // ── Render nodes per level ──────────────────────────────────────────────────
  const maxCommunity = $derived(Math.max(1, ...data.communities.map((c) => c.nodeCount)));
  const communityNodes = $derived<RenderNode[]>(
    data.communities.map((c) => ({
      id: c.id,
      label: c.path || c.kind,
      kind: c.kind,
      r: scaleRadius(c.nodeCount, maxCommunity, 12, 42),
      color: kindColor(c.kind),
      ring: kindNeedsRing(c.kind),
    })),
  );

  const maxDegree = $derived(Math.max(1, ...active.symbols.map((s) => s.degree)));
  const symbolNodes = $derived<RenderNode[]>(
    active.symbols.map((s) => ({
      id: s.id,
      label: s.name,
      kind: s.kind,
      r: scaleRadius(s.degree, maxDegree, 5, 22),
      color: kindColor(s.kind),
      ring: kindNeedsRing(s.kind),
    })),
  );

  const renderNodes = $derived(view.level === 'communities' ? communityNodes : symbolNodes);
  const renderLinks = $derived(view.level === 'communities' ? [] : active.links);
  const legend = $derived(legendItems(renderNodes.map((n) => n.kind)));

  // ── Inspector selection ─────────────────────────────────────────────────────
  const communityById = $derived(new Map(data.communities.map((c) => [c.id, c])));
  const symbolById = $derived(new Map(active.symbols.map((s) => [s.id, s])));

  const selCommunity = $derived(
    view.level === 'communities' && view.selectedId
      ? (communityById.get(view.selectedId) ?? null)
      : null,
  );
  const selSymbol = $derived(
    view.level === 'symbols' && view.selectedId
      ? (symbolById.get(view.selectedId) ?? null)
      : null,
  );

  const inspectorCommunity = $derived(
    selCommunity
      ? {
          label: selCommunity.label,
          kind: selCommunity.kind,
          path: selCommunity.path,
          nodeCount: selCommunity.nodeCount,
          sharePct: Math.round((selCommunity.nodeCount / Math.max(1, data.communityNodeTotal)) * 100),
        }
      : null,
  );

  const dependsOn = $derived(
    selSymbol
      ? active.links
          .filter((l) => l.source === selSymbol.id)
          .map((l) => symbolById.get(l.target)?.name ?? l.target)
      : [],
  );
  const usedBy = $derived(
    selSymbol
      ? active.links
          .filter((l) => l.target === selSymbol.id)
          .map((l) => symbolById.get(l.source)?.name ?? l.source)
      : [],
  );

  const overview = $derived({
    nodes: renderNodes.length,
    relations: renderLinks.length,
    communities: data.communities.length,
    shown: view.level === 'symbols' ? active.symbols.length : undefined,
    total: view.level === 'symbols' ? active.totalSymbols : undefined,
  });

  // Scope options for the rokkit Select. When the active repo isn't among the
  // project's recorded git roots (a graph indexed under a name we don't list),
  // surface it as a synthetic first entry so the trigger shows the real value.
  const scopeItems = $derived.by(() => {
    const items = data.scopes.map((s) => ({ value: s.name, label: s.name }));
    if (!data.scopes.some((s) => s.name === data.repoId)) {
      items.unshift({ value: data.repoId, label: data.repoId });
    }
    return items;
  });

  function onScopeSelect(value: unknown) {
    goto(`?repo=${encodeURIComponent(String(value))}`, { invalidateAll: true });
  }
</script>

<div class="flex flex-col h-full" data-screen="atlas" data-atlas-repo={data.repoId}>
  <!-- Header -->
  <div class="flex items-center gap-5 pt-5 pb-4 px-6 border-b border-paper-edge">
    <span class="kanji text-[40px] text-accent leading-none">図</span>
    <div class="flex-1 min-w-0">
      <p class="m-0 mb-1"><Eyebrow>This project · Atlas</Eyebrow></p>
      <h1 class="display text-[22px] font-normal m-0">Code graph</h1>
      <p class="text-[13px] text-ink-mute mt-1 mb-0 max-w-[640px] leading-relaxed">
        The call graph the daemon indexed for
        <span class="font-mono text-ink-soft">{data.repoId}</span> — clusters at a
        glance, individual symbols and their calls up close.
      </p>
    </div>
    <div class="flex gap-5 pl-5 border-l border-paper-edge">
      <div class="text-center">
        <div class="font-mono text-[17px] font-light text-ink">{data.stats.modules}</div>
        <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">modules</div>
      </div>
      <div class="text-center">
        <div class="font-mono text-[17px] font-light text-ink">{data.stats.exports}</div>
        <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">exports</div>
      </div>
      <div class="text-center">
        <div class="font-mono text-[17px] font-light text-ink">{data.stats.calls}</div>
        <div class="text-xs uppercase tracking-wider text-ink-faint mt-1">calls</div>
      </div>
    </div>
  </div>

  <!-- Controls -->
  <div class="flex items-center gap-3 flex-wrap px-6 py-3 border-b border-paper-edge">
    <!-- scope -->
    <div class="flex items-center gap-2 text-xs text-ink-mute" data-atlas-scope>
      <span class="uppercase tracking-wider">Scope</span>
      <Select
        class="font-mono text-xs max-w-[220px]"
        items={scopeItems}
        value={data.repoId}
        onchange={onScopeSelect}
      />
    </div>

    <span class="flex-1"></span>

    <!-- hide tests toggle — focus on production code -->
    <button
      type="button"
      class="text-xs rounded py-2 px-3 cursor-pointer border border-paper-edge"
      class:bg-paper-mute={hideTests}
      class:text-ink={hideTests}
      class:font-semibold={hideTests}
      class:bg-transparent={!hideTests}
      class:text-ink-mute={!hideTests}
      data-atlas-hide-tests
      aria-pressed={hideTests}
      onclick={() => (hideTests = !hideTests)}
    >
      Hide tests
    </button>

    <!-- level segmented control -->
    <div class="inline-flex bg-paper-mute rounded p-1" role="group" aria-label="Granularity">
      {#each LEVELS as l (l.value)}
        {@const active = view.level === l.value}
        <button
          type="button"
          class="text-xs rounded py-2 px-3 cursor-pointer border-none"
          class:bg-paper={active}
          class:text-ink={active}
          class:font-semibold={active}
          class:bg-transparent={!active}
          class:text-ink-mute={!active}
          data-atlas-level={l.value}
          aria-pressed={active}
          onclick={() => view.setLevel(l.value)}
        >
          {l.label}
        </button>
      {/each}
    </div>
  </div>

  {#if empty}
    <div class="p-6">
      <EmptyState
        kanji="図"
        title="no graph for this scope"
        description="sensei hasn't indexed a code graph for {data.repoId}. Pick a scope that has been scanned, or run a scan for this project."
      />
    </div>
  {:else}
    <!-- Body: canvas + inspector -->
    <div class="grid grid-cols-[1fr_290px] min-h-0 flex-1">
      <div class="relative min-h-0 bg-paper" data-atlas-graph>
        <AtlasCanvas
          nodes={renderNodes}
          links={renderLinks}
          selectedId={view.selectedId}
          onselect={(id) => view.select(id)}
        />
        {#if view.level === 'symbols' && active.totalSymbols > active.symbols.length}
          <div class="absolute left-5 top-3 text-xs text-ink-mute font-mono" data-atlas-cap-note>
            showing {active.symbols.length} of {active.totalSymbols} symbols · by connections{hideTests
              ? ' · tests hidden'
              : ''}
          </div>
        {/if}
        <div class="absolute left-5 bottom-4">
          <AtlasLegend items={legend} />
        </div>
      </div>
      <div class="border-l border-paper-edge bg-paper-soft min-h-0">
        <AtlasInspector
          community={inspectorCommunity}
          symbol={selSymbol}
          {dependsOn}
          {usedBy}
          {overview}
        />
      </div>
    </div>
  {/if}
</div>
