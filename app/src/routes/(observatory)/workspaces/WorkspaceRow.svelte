<script lang="ts">
  // A workspace as a single screen-line. Four columns: icon · name-block ·
  // signal · repos/libs. The name block carries the same identity elements
  // as the card (dot, name, client + status pills) plus a one-line vision;
  // the vision truncates with an ellipsis so the row never wraps to two
  // lines. Presentational only.
  import type { EnrichedWorkspace } from './buckets.js';
  import { workspaceStatus, workspaceIcon, lastSessionLabel } from './buckets.js';
  import { ftrPctLabel, FTR_UNAVAILABLE } from '$lib/ftr.js';
  import { apiBase } from '$lib/api.js';
  import { glyphMode } from '$lib/glyphs';
  import { appState } from '$lib/appstate.svelte.js';
  import WorkspacePill from './WorkspacePill.svelte';
  import WorkspaceDot from './WorkspaceDot.svelte';
  import WorkspaceGlyph from './WorkspaceGlyph.svelte';

  let {
    p,
    onOpen,
    last = false,
  }: {
    p: EnrichedWorkspace;
    onOpen?: (id: string, name: string) => void;
    last?: boolean;
  } = $props();

  const status = $derived(workspaceStatus(p));
  // Image icons resolve to the daemon's serve route; kanji icons stay glyphs.
  const icon = $derived(workspaceIcon(p, apiBase(appState.port), glyphMode(appState.config)));
  // "NN%" or the "—" no-data marker (never a fabricated 0%).
  const ftrLabel = $derived(ftrPctLabel(p.ftr14d));
</script>

<button
  type="button"
  data-workspace-row={p.id}
  data-status={status}
  onclick={() => onOpen?.(p.id, p.name)}
  class="w-full text-left bg-transparent hover:bg-paper-mute grid grid-cols-[auto_1fr_auto_auto] items-center gap-3 py-3 px-4 cursor-pointer text-inherit"
  class:border-b={!last}
  class:border-paper-edge={!last}
  class:opacity-60={status === 'archived'}
>
  <!-- col 1 — icon -->
  <WorkspaceGlyph {icon} />

  <!-- col 2 — name block -->
  <div class="min-w-0">
    <div class="flex items-center gap-2">
      <WorkspaceDot ftr={p.ftr14d} warn={p.warn} />
      <span class="text-sm text-ink truncate">{p.name}</span>
      {#if p.client}
        <WorkspacePill text={p.client} />
      {/if}
      {#if status !== 'active'}
        <WorkspacePill text={status} tone="dormant" />
      {/if}
    </div>
    {#if p.vision}
      <div data-vision class="text-xs text-ink-soft leading-snug mt-0.5 truncate">{p.vision}</div>
    {/if}
  </div>

  <!-- col 3 — signal -->
  <div class="font-mono text-xs text-right tabular-nums whitespace-nowrap">
    {#if status === 'active'}
      {#if p.readError}
        <!-- The quality reads failed (#288): said so, never a blank rate. -->
        <span data-ftr-unavailable title={p.readError} class="text-danger">{FTR_UNAVAILABLE} ftr</span>
      {:else}
        <span class={p.warn ? 'text-warning' : 'text-ink-mute'}>{ftrLabel} ftr</span>
      {/if}
    {:else}
      <span class="text-ink-faint">last · {lastSessionLabel(p.last_session_at)}</span>
    {/if}
  </div>

  <!-- col 4 — repos · libs -->
  <div class="font-mono text-xs text-ink-faint text-right tabular-nums whitespace-nowrap min-w-[78px]">
    {p.repos_count} repos · {p.libs_count} libs
  </div>
</button>
