<script lang="ts">
  import {
    kindColor,
    kindNeedsRing,
    type InspectorCommunity,
    type InspectorSymbol,
    type InspectorOverview,
  } from './atlas-graph.svelte.js';

  let {
    community = null,
    symbol = null,
    dependsOn = [],
    usedBy = [],
    overview,
  }: {
    community?: InspectorCommunity | null;
    symbol?: InspectorSymbol | null;
    dependsOn?: string[];
    usedBy?: string[];
    overview: InspectorOverview;
  } = $props();

  const capped = $derived(
    overview.total != null &&
      overview.shown != null &&
      overview.total > overview.shown,
  );
</script>

<div class="py-5 px-4 h-full overflow-auto" data-component="atlas-inspector">
  {#if community}
    <!-- Focused community -->
    <div class="flex items-center gap-3 mb-1">
      <span
        class="w-[34px] h-[34px] rounded-full shrink-0"
        class:border={kindNeedsRing(community.kind)}
        class:border-ink-faint={kindNeedsRing(community.kind)}
        style="background: {kindColor(community.kind)};"
      ></span>
      <div class="min-w-0">
        <div class="text-xs uppercase tracking-wider text-ink-mute">Community · {community.kind}</div>
        <div class="font-mono text-sm text-ink font-semibold truncate">{community.path || '(root)'}</div>
      </div>
    </div>

    <div class="grid grid-cols-[1fr_auto] gap-y-2 mt-4">
      <span class="text-[13px] text-ink-soft">Members</span>
      <span class="font-mono text-[13px] text-ink">{community.nodeCount}</span>
      <span class="text-[13px] text-ink-soft">Share</span>
      <span class="font-mono text-[13px] text-ink">{community.sharePct}%</span>
    </div>

    <p class="text-[13px] text-ink-soft leading-relaxed mt-4 mb-0">
      A cluster the analyzer grouped by call structure. Open the symbol view to
      trace individual calls.
    </p>
  {:else if symbol}
    <!-- Focused symbol -->
    <div class="flex items-center gap-3 mb-1">
      <span
        class="w-[34px] h-[34px] rounded-full shrink-0"
        class:border={kindNeedsRing(symbol.kind)}
        class:border-ink-faint={kindNeedsRing(symbol.kind)}
        style="background: {kindColor(symbol.kind)};"
      ></span>
      <div class="min-w-0">
        <div class="text-xs uppercase tracking-wider text-ink-mute">{symbol.kind}</div>
        <div class="font-mono text-sm text-ink font-semibold truncate">{symbol.name}</div>
      </div>
    </div>
    <div class="font-mono text-xs text-ink-faint truncate mt-1 mb-4">{symbol.file}</div>

    <div class="grid grid-cols-[1fr_auto] gap-y-2 mb-4">
      <span class="text-[13px] text-ink-soft">Connections</span>
      <span class="font-mono text-[13px] text-ink">{symbol.degree}</span>
    </div>

    {#if dependsOn.length > 0}
      <div class="mb-3">
        <div class="text-xs uppercase tracking-wider text-ink-mute mb-2">Calls · {dependsOn.length}</div>
        <div class="flex flex-wrap gap-1">
          {#each dependsOn as name, i (name + i)}
            <span class="font-mono text-xs text-ink-soft border border-paper-edge rounded py-1 px-2">{name}</span>
          {/each}
        </div>
      </div>
    {/if}
    {#if usedBy.length > 0}
      <div class="mb-3">
        <div class="text-xs uppercase tracking-wider text-ink-mute mb-2">Called by · {usedBy.length}</div>
        <div class="flex flex-wrap gap-1">
          {#each usedBy as name, i (name + i)}
            <span class="font-mono text-xs text-ink-soft border border-paper-edge rounded py-1 px-2">{name}</span>
          {/each}
        </div>
      </div>
    {/if}
  {:else}
    <!-- Nothing focused — describe the whole view -->
    <div class="text-xs uppercase tracking-wider text-ink-mute mb-3">This view</div>
    <div class="grid grid-cols-[1fr_auto] gap-y-2 mb-4">
      <span class="text-[13px] text-ink-soft">Nodes</span>
      <span class="font-mono text-[13px] text-ink">{overview.nodes}</span>
      <span class="text-[13px] text-ink-soft">Relations</span>
      <span class="font-mono text-[13px] text-ink">{overview.relations}</span>
      <span class="text-[13px] text-ink-soft">Communities</span>
      <span class="font-mono text-[13px] text-ink">{overview.communities}</span>
    </div>
    {#if capped}
      <p class="text-xs text-ink-mute mb-3" data-atlas-cap>
        Showing the {overview.shown} most-connected of {overview.total} symbols.
      </p>
    {/if}
    <p class="text-[13px] text-ink-soft leading-relaxed m-0">
      Select any node to trace what it connects to.
    </p>
  {/if}
</div>
