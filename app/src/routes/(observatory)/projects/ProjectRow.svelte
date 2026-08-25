<script lang="ts">
  // A project as a single screen-line. Four columns: icon · name-block ·
  // signal · repos/libs. The name block carries the same identity elements
  // as the card (dot, name, client + status pills) plus a one-line vision;
  // the vision truncates with an ellipsis so the row never wraps to two
  // lines. Presentational only.
  import type { EnrichedProject } from './buckets.js';
  import { projectStatus, projectIcon, lastSessionLabel } from './buckets.js';
  import { ftrPctLabel } from '$lib/ftr.js';
  import { apiBase } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import ProjPill from './ProjPill.svelte';
  import ProjectDot from './ProjectDot.svelte';
  import ProjectGlyph from './ProjectGlyph.svelte';

  let {
    p,
    onOpen,
    last = false,
  }: {
    p: EnrichedProject;
    onOpen?: (id: string, name: string) => void;
    last?: boolean;
  } = $props();

  const status = $derived(projectStatus(p));
  // Image icons resolve to the daemon's serve route; kanji icons stay glyphs.
  const icon = $derived(projectIcon(p, apiBase(appState.port)));
  // "NN%" or the "—" no-data marker (never a fabricated 0%).
  const ftrLabel = $derived(ftrPctLabel(p.ftr14d));
</script>

<button
  type="button"
  data-project-row={p.id}
  data-status={status}
  onclick={() => onOpen?.(p.id, p.name)}
  class="w-full text-left bg-transparent hover:bg-paper-mute grid grid-cols-[auto_1fr_auto_auto] items-center gap-3 py-3 px-4 cursor-pointer text-inherit"
  class:border-b={!last}
  class:border-paper-edge={!last}
  class:opacity-60={status === 'archived'}
>
  <!-- col 1 — icon -->
  <ProjectGlyph {icon} />

  <!-- col 2 — name block -->
  <div class="min-w-0">
    <div class="flex items-center gap-2">
      <ProjectDot ftr={p.ftr14d} warn={p.warn} />
      <span class="text-sm text-ink truncate">{p.name}</span>
      {#if p.client}
        <ProjPill text={p.client} />
      {/if}
      {#if status !== 'active'}
        <ProjPill text={status} tone="dormant" />
      {/if}
    </div>
    {#if p.vision}
      <div data-vision class="text-xs text-ink-mute leading-snug mt-0.5 truncate">{p.vision}</div>
    {/if}
  </div>

  <!-- col 3 — signal -->
  <div class="font-mono text-xs text-right tabular-nums whitespace-nowrap">
    {#if status === 'active'}
      <span class:text-warning={p.warn} class:text-ink-soft={!p.warn}>{ftrLabel} ftr</span>
    {:else}
      <span class="text-ink-faint">last · {lastSessionLabel(p.last_session_at)}</span>
    {/if}
  </div>

  <!-- col 4 — repos · libs -->
  <div class="font-mono text-xs text-ink-faint text-right tabular-nums whitespace-nowrap min-w-[78px]">
    {p.repos_count} repos · {p.libs_count} libs
  </div>
</button>
