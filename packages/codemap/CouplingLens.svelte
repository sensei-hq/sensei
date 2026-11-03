<script lang="ts">
  // The Hidden coupling lens (M17): the pairs of modules that keep changing together
  // with no dependency between them — strongest first, each a dashed tie whose weight
  // is how often the rarer of the two changes with the other. Only the coupled
  // modules appear; either side opens its module. The text follows the zoom.
  import LensFailed from './LensFailed.svelte';
  import type { CoChange } from './types';

  let { data, zoom = 1, onfocus, focus = null, failed = null, onretry = () => {} }: {
    data: CoChange | null;
    zoom?: number;
    onfocus: (id: string) => void;
    /** What the map is focused on, so an empty list says whose it is. */
    focus?: string | null;
    /** Why this lens's own read failed; shown with Retry instead of "Reading…". */
    failed?: string | null;
    onretry?: () => void;
  } = $props();
  const pairs = $derived([...(data?.hidden ?? [])].sort((a, b) => b.together - a.together || b.strength - a.strength));
  const most = $derived(Math.max(1, ...pairs.map((p) => p.together)));
</script>

{#snippet side(id: string, align: 'end' | 'start')}
  <span class="flex items-center gap-1.5 min-w-0" class:justify-end={align === 'end'}>
    <span class="font-mono text-ink truncate" title={id}>{id}</span>
    <button class="shrink-0 p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:text-ink"
      aria-label={`Focus ${id}`} title={`Focus ${id}`} onclick={() => onfocus(id)}>↗</button>
  </span>
{/snippet}

<!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="w-full overflow-auto" tabindex="0" role="region" aria-label="Hidden coupling, scrollable" data-testid="codemap-coupling"
  style:font-size={`${Math.round(13 * zoom)}px`}>
  {#if !data && failed}
    <LensFailed reason={failed} {onretry} />
  {:else if !data}
    <p class="m-0 text-ink-faint">Reading the commit history…</p>
  {:else if !data.commits}
    <p class="m-0 text-ink-mute">No commit history read for this workspace yet.</p>
  {:else if !pairs.length}
    {#if focus}
      <p class="m-0 text-success">No hidden coupling involving {focus}: no module changes together with it {data.minTogether}+ times without a dependency between them.</p>
    {:else}
      <p class="m-0 text-success">No hidden coupling: no two modules change together {data.minTogether}+ times without a dependency between them.</p>
    {/if}
  {:else}
    <ol class="m-0 p-0 list-none grid gap-y-2 items-center" style="grid-template-columns: minmax(0, 1fr) minmax(6em, 14em) minmax(0, 1fr) auto;">
      {#each pairs as p (`${p.a}|${p.b}`)}
        <li class="contents" data-pair={`${p.a}|${p.b}`}>
          {@render side(p.a, 'end')}
          <span class="px-2" aria-hidden="true">
            <svg width="100%" height="10" preserveAspectRatio="none" viewBox="0 0 100 10">
              <line x1="0" y1="5" x2="100" y2="5" stroke="var(--warning)" stroke-dasharray="6 4"
                stroke-width={1 + 4 * (p.together / most)} stroke-opacity={0.35 + 0.65 * p.strength} />
            </svg>
          </span>
          {@render side(p.b, 'start')}
          <span class="pl-3 text-ink-mute whitespace-nowrap">{p.together} commits · {Math.round(p.strength * 100)}%</span>
        </li>
      {/each}
    </ol>
  {/if}
</div>
