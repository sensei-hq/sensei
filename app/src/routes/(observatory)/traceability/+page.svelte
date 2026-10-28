<script lang="ts">
    import { railIcon } from "../observatory-nav";
  import { invalidateAll } from '$app/navigation';
  import { PageHeader, Screen, EmptyState, ScreenState, UnreadWorkspacesNotice } from '$lib/components';
  import { openWorkspaceWindow } from '$lib/stores/windows.svelte.js';
  import {
    driftChipClasses,
    rollupHeadline,
    type DriftGroup,
    type DriftRow,
  } from './traceability-state.svelte.js';

  let { data } = $props();
</script>

{#snippet driftItem(r: DriftRow)}
  {@const chip = driftChipClasses(r.status)}
  <!-- A drift row opens the workspace in its OWN window (never replaces the
       observatory main window) — every workspace graph/screen lives in a workspace
       window. A button (not an anchor) so it can't navigate the main window. -->
  <button
    type="button"
    data-drift-row={r.id}
    onclick={() => { void openWorkspaceWindow(r.workspaceId, r.workspaceName); }}
    class="w-full text-left flex items-center gap-3 py-2 px-3 text-inherit bg-transparent border-0 border-b border-paper-edge last:border-b-0 cursor-pointer hover:bg-paper-mute"
  >
    <span class="flex-1 text-sm text-ink leading-snug">{r.detail}</span>
    <span class="font-mono text-xs uppercase tracking-wide px-2 py-0.5 rounded {chip.bg} {chip.text}">
      {r.status}
    </span>
  </button>
{/snippet}

{#snippet group(g: DriftGroup)}
  <section class="mb-8" data-drift-group={g.workspaceId}>
    <div class="flex items-baseline gap-3 mb-2">
      <h2 class="display text-lg font-normal m-0">{g.workspaceName}</h2>
      <span class="font-mono text-xs text-danger">{g.broken} broken</span>
      {#if g.drifted > 0}
        <span class="font-mono text-xs text-warning">{g.drifted} drifted</span>
      {/if}
    </div>
    <div class="border border-paper-edge rounded-md overflow-hidden bg-paper-soft">
      {#each g.rows as r (r.id)}
        {@render driftItem(r)}
      {/each}
    </div>
  </section>
{/snippet}

<Screen>
    {#snippet header()}
    <PageHeader
      eyebrow="Observatory · Document traceability"
      kanji="巻" icon={railIcon("/traceability")}
      title="Where the docs and the code disagree."
      description="Every doc-to-symbol link, checked nightly. Drift surfaces here before someone reads a stale doc and writes the wrong thing."
    />
    {/snippet}

    {#if !data.rollup || !data.groups}
    <div class="max-w-[1060px] mx-auto px-7 pb-10">
      <ScreenState status="error" error={data.error} onretry={() => invalidateAll()} />
    </div>
    {:else}
    <div class="max-w-[1060px] mx-auto px-7 pb-10" data-traceability-total={data.rollup.total}>
      <UnreadWorkspacesNotice unread={data.unread} />
      {#if data.rollup.total === 0 && data.indexedDocs === 0}
        <!-- Nothing to check (#290): no document is indexed in any workspace. -->
        <EmptyState
          kanji="巻"
          title="no documents indexed"
          description="Drift is checked against indexed documents, and sensei does not index markdown yet — so nothing has been checked. This is not a clean bill of health."
        />
      {:else if data.rollup.total === 0}
        <EmptyState
          kanji="巻"
          title="no drift detected"
          description="Doc-to-code references are all resolving. Broken or drifted mentions will surface here once a workspace's next scan finds one."
        />
      {:else}
        <p class="text-sm text-ink-mute mb-6" data-traceability-rollup>{rollupHeadline(data.rollup)}</p>
        {#each data.groups as g (g.workspaceId)}
          {@render group(g)}
        {/each}
      {/if}
    </div>
    {/if}
</Screen>
