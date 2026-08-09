<script lang="ts">
  // Denser project card. Three vertical bands: identity row, optional vision
  // row, and a three-up stat grid. The stat grid switches on status — an
  // active project shows ftr/repos/libs; a dormant or archived one shows
  // repos/libs/last-session in a quieter tone. Presentational only: every
  // derivation is a pure helper from buckets.ts.
  import type { EnrichedProject } from './buckets.js';
  import { projectStatus, projectIcon, lastSessionLabel } from './buckets.js';
  import { ftrPct } from '$lib/ftr.js';
  import { apiBase } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import ProjPill from './ProjPill.svelte';
  import ProjectDot from './ProjectDot.svelte';
  import ProjectGlyph from './ProjectGlyph.svelte';

  let {
    p,
    onOpen,
  }: {
    p: EnrichedProject;
    onOpen?: (id: string, name: string) => void;
  } = $props();

  const status = $derived(projectStatus(p));
  // Image icons resolve to the daemon's serve route; kanji icons stay glyphs.
  const icon = $derived(projectIcon(p, apiBase(appState.port)));
  // Bare percentage number for the stat cell, or "—" when there's no FTR data
  // (honest no-data; never a fabricated 0).
  const ftrStat = $derived<number | string>(ftrPct(p.ftr14d) ?? '—');
</script>

{#snippet stat(label: string, value: string | number, tone: 'default' | 'mute' | 'warn')}
  <div class="flex flex-col h-full">
    <div class="text-[10px] uppercase tracking-[0.08em] text-ink-mute leading-tight">{label}</div>
    <div
      class="display text-base font-normal mt-auto leading-tight tabular-nums"
      class:text-ink={tone === 'default'}
      class:text-ink-mute={tone === 'mute'}
      class:text-warning={tone === 'warn'}
    >{value}</div>
  </div>
{/snippet}

<button
  type="button"
  data-project-card={p.id}
  data-status={status}
  onclick={() => onOpen?.(p.id, p.name)}
  class="flex flex-col gap-2 text-left bg-paper-soft hover:bg-paper-mute border border-paper-edge rounded-lg cursor-pointer text-inherit py-3 px-3"
  class:opacity-60={status === 'archived'}
>
  <!-- Row 1 — identity -->
  <div class="flex items-center gap-2">
    <ProjectGlyph {icon} />
    <ProjectDot ftr={p.ftr14d} warn={p.warn} />
    <span class="text-[13px] text-ink flex-1 min-w-0 truncate">{p.name}</span>
    {#if p.client}
      <ProjPill text={p.client} />
    {/if}
    {#if status !== 'active'}
      <ProjPill text={status} tone="dormant" />
    {/if}
  </div>

  <!-- Row 2 — vision (omitted entirely when absent) -->
  {#if p.vision}
    <div data-vision class="text-[12px] text-ink-soft leading-snug text-pretty">{p.vision}</div>
  {/if}

  <!-- Stat grid -->
  {#if status === 'active'}
    <div class="grid grid-cols-3 gap-2 pt-2 border-t border-paper-edge">
      {@render stat('ftr', ftrStat, p.warn ? 'warn' : 'default')}
      {@render stat('repos', p.repos_count, 'default')}
      {@render stat('libs', p.libs_count, 'default')}
    </div>
  {:else}
    <div class="grid grid-cols-3 gap-2 pt-2 border-t border-paper-edge">
      {@render stat('repos', p.repos_count, 'mute')}
      {@render stat('libs', p.libs_count, 'mute')}
      {@render stat('last session', lastSessionLabel(p.last_session_at), 'mute')}
    </div>
  {/if}
</button>
