<script lang="ts">
  import { PageHeader, Eyebrow, EmptyState } from '$lib/components';
  import type { TriageRec } from './triage.js';

  let { data } = $props();
</script>

{#snippet card(rec: TriageRec, tone: 'now' | 'soon' | 'settled')}
  <a
    data-triage-rec={rec.id}
    href={`/project/${rec.projectId}/overview`}
    class="block p-4 border rounded-md bg-paper-soft hover:bg-paper-mute mb-2 text-inherit"
    class:border-accent-soft={tone === 'now'}
    class:border-warning-soft={tone === 'soon'}
    class:border-paper-edge={tone === 'settled'}
  >
    <div class="flex items-baseline justify-between mb-1.5">
      <Eyebrow tone={tone === 'now' ? 'text-accent' : tone === 'soon' ? 'text-warning' : 'text-ink-mute'}>
        {rec.projectName}
      </Eyebrow>
      <span class="text-xs text-ink-mute uppercase tracking-wide">{rec.urgency}</span>
    </div>
    <p class="text-sm text-ink m-0 leading-snug">{rec.title}</p>
    {#if rec.why}
      <p class="text-xs text-ink-mute mt-1 m-0 leading-snug line-clamp-2">{rec.why}</p>
    {/if}
    {#if tone === 'settled' && rec.actedAt}
      <p class="text-xs text-ink-mute mt-1 m-0 font-mono">
        {rec.status} · {new Date(rec.actedAt).toLocaleDateString()}
      </p>
    {/if}
  </a>
{/snippet}

{#snippet column(kanji: string, title: string, sub: string, recs: TriageRec[], tone: 'now' | 'soon' | 'settled')}
  <section class="flex flex-col min-w-0" data-triage-column={tone}>
    <div class="mb-4">
      <div class="flex items-baseline gap-3 mb-1">
        <span
          class="kanji text-2xl"
          class:text-accent={tone === 'now'}
          class:text-warning={tone === 'soon'}
          class:text-success={tone === 'settled'}
        >{kanji}</span>
        <h2 class="display text-lg font-normal m-0">{title}</h2>
        <Eyebrow>{recs.length}</Eyebrow>
      </div>
      <p class="text-xs text-ink-mute m-0">{sub}</p>
    </div>
    {#if recs.length === 0}
      <p class="text-xs text-ink-soft italic">Nothing here.</p>
    {:else}
      {#each recs as rec (rec.id)}
        {@render card(rec, tone)}
      {/each}
    {/if}
  </section>
{/snippet}

<PageHeader kanji="學" eyebrow="Observatory · Insights" title="Triage" />
<div class="px-6 pb-10">
  {#if data.total === 0}
    <EmptyState
      kanji="學"
      title="Nothing to triage yet."
      description="Recommendations show up here once the analyzer has enough signal to teach — usually after 20+ sessions across a project."
    />
  {:else}
    <div class="grid grid-cols-3 gap-6" data-triage-grid>
      {@render column('今', 'Now',     'act this week',                        data.buckets.now,     'now')}
      {@render column('近', 'Soon',    'worth a look',                         data.buckets.soon,    'soon')}
      {@render column('定', 'Settled', 'recent decisions · low-noise archive', data.buckets.settled, 'settled')}
    </div>
  {/if}
</div>
