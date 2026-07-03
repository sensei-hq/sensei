<script lang="ts">
  import { PageHeader, EmptyState, Eyebrow } from '$lib/components';
  import { formatDeltaPct, type ImpactRow } from './aggregate.js';

  let { data } = $props();

  const orderedBuckets: Array<[keyof typeof data.buckets, string, string, string]> = [
    ['positive', '陽', 'Positive', 'measured lift in FTR — keep these'],
    ['negative', '陰', 'Negative', 'regression against baseline — consider rolling back'],
    ['neutral',  '平', 'Neutral',  'safe, no measurable delta either way'],
    ['pending',  '待', 'Pending',  'accepted; measurement window still open'],
  ];

  function deltaTone(v: number | null): string {
    if (v == null || v === 0) return 'text-ink-mute';
    return v > 0 ? 'text-success' : 'text-warning';
  }
</script>

{#snippet row(r: ImpactRow)}
  <a
    data-impact-row={r.id}
    href={`/project/${r.projectId}/impact`}
    class="grid grid-cols-[1fr_auto_auto] gap-4 items-baseline p-3 border-b border-paper-mute hover:bg-paper-soft text-inherit"
  >
    <div class="min-w-0">
      <Eyebrow>{r.projectName}</Eyebrow>
      <p class="text-sm text-ink m-0 mt-0.5 truncate">{r.title}</p>
    </div>
    <span
      data-ftr-delta
      class="font-mono text-sm {deltaTone(r.ftrDelta)}"
    >{formatDeltaPct(r.ftrDelta)}</span>
    <span class="text-xs text-ink-mute font-mono">
      {r.baselineFtr != null ? Math.round(r.baselineFtr * 100) : '—'} → {r.currentFtr != null ? Math.round(r.currentFtr * 100) : '—'}
    </span>
  </a>
{/snippet}

<PageHeader
  variant="h2"
  eyebrow="Observatory · Change impact"
  kanji="果"
  title="Did sensei's advice actually work?"
  description="Each accepted recommendation gets a measurement window — FTR delta before vs after, plus the MOE consensus that closed the loop."
/>

<div class="px-6 pb-10" data-impact-total={data.total}>
  {#if data.total === 0}
    <EmptyState
      kanji="果"
      title="no measurements yet"
      description="Accepted recommendations show up here once the analyzer's MeasureVerdicts pass has enough post-acceptance sessions (typically ≥3 sessions across ≥3 days)."
    />
  {:else}
    {#each orderedBuckets as [key, kanji, title, sub] (key)}
      {@const items = data.buckets[key]}
      {#if items.length > 0}
        <section class="mb-8" data-impact-bucket={key}>
          <div class="flex items-baseline gap-3 mb-1">
            <span
              class="kanji text-2xl"
              class:text-success={key === 'positive'}
              class:text-warning={key === 'negative'}
              class:text-ink-mute={key === 'neutral' || key === 'pending'}
            >{kanji}</span>
            <h2 class="display text-lg font-normal m-0">{title}</h2>
            <Eyebrow>{items.length}</Eyebrow>
          </div>
          <p class="text-xs text-ink-mute mb-3">{sub}</p>
          <div class="rounded-md border border-paper-mute overflow-hidden">
            {#each items as r (r.id)}
              {@render row(r)}
            {/each}
          </div>
        </section>
      {/if}
    {/each}
  {/if}
</div>
