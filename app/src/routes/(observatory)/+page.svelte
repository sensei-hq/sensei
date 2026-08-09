<script lang="ts">
  import { Eyebrow, FtrStrip, Kanji } from '$lib/components';
  import RecentSessions from './RecentSessions.svelte';
  import HeroKoan from './HeroKoan.svelte';
  import InsightCard from './InsightCard.svelte';
  import AdoptedCard from './AdoptedCard.svelte';
  import { ftrChip, ftrBars, greetingLine } from './today-view.svelte.js';
  import type { PageData } from './$types.js';

  let { data }: { data: PageData } = $props();

  // The daemon owns the maturity gate — the screen renders it, it does not
  // decide. Everything below switches presentation only, never copy.
  const home = $derived(data.today);
  const mode = $derived(home.dataMaturity);
  const chip = $derived(ftrChip(data.ftr));
  const bars = $derived(ftrBars(data.ftr));
  const greeting = $derived(greetingLine(home.greeting));
</script>

<div class="max-w-[1060px] mx-auto px-7 py-6 pb-16" data-mode={mode}>
  <!-- Pinned greeting + FTR header -->
  <div class="flex items-start justify-between mb-8">
    <div>
      <p class="m-0 mb-2"><Eyebrow>{home.today}</Eyebrow></p>
      <h1 class="display text-2xl font-normal m-0 tracking-tight">{greeting}</h1>
    </div>

    <div data-ftr-header class="flex items-end gap-5 text-right" data-dim={mode === 'early' || undefined}>
      <div>
        <p class="m-0 mb-1"><Eyebrow>First-Try-Right · 14d</Eyebrow></p>
        <div class="flex items-baseline gap-2 justify-end">
          {#if chip.value === null}
            <span class="display text-3xl font-normal text-ink-mute">—</span>
          {:else}
            <span
              class="display text-3xl font-normal"
              class:text-ink={mode === 'mature'}
              class:text-ink-mute={mode === 'early'}
            >{chip.value}</span>
            <span class="text-xs text-ink-soft">%</span>
            {#if chip.delta !== null}
              <span data-ftr-arrow={chip.direction} class="font-mono text-xs {chip.toneClass}">
                {chip.arrow} {Math.abs(chip.delta)}%
              </span>
            {/if}
          {/if}
        </div>
      </div>
      <FtrStrip data={bars} width={168} height={56} dim={mode === 'early'} />
    </div>
  </div>

  <!-- Hero koan — the one focal thing -->
  <HeroKoan hero={home.hero} {mode} />

  <!-- Two columns: Insights + Adopted teachings -->
  <div class="grid grid-cols-[1.4fr_1fr] gap-6 mt-6 mb-10">
    <!-- Insights behind the hero -->
    <div>
      {@render sectionHead(mode === 'early' ? 'Early signals' : 'Also worth noticing', '/insights', 'all insights →')}
      {#if home.insights.length > 0}
        <div class="flex flex-col">
          {#each home.insights as insight, i (i)}
            <InsightCard {insight} />
          {/each}
        </div>
      {:else}
        <div
          data-insights-empty
          class="py-4 text-center text-sm text-ink-faint italic border border-dashed border-paper-edge rounded-lg"
        >
          Nothing yet. Keep working — sensei watches.
        </div>
      {/if}
    </div>

    <!-- What sensei has adopted on the user's behalf -->
    <div>
      {@render sectionHead('System has learned', '/learnings', 'all teachings →')}
      {#if home.adopted.length > 0}
        <div class="flex flex-col gap-2">
          {#each home.adopted as item, i (i)}
            <AdoptedCard {item} />
          {/each}
        </div>
      {:else}
        <div
          data-adopted-empty
          class="py-5 px-4 text-center border border-dashed border-paper-edge rounded-lg"
        >
          <span class="block mb-2"><Kanji char="空" size="2xl" tone="muted" /></span>
          <p class="text-sm text-ink-soft leading-snug m-0">
            No teachings adopted yet.<br />
            sensei needs a few more sessions to be confident.
          </p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Recent sessions — both modes -->
  <RecentSessions sessions={data.recentSessions} />
</div>

{#snippet sectionHead(title: string, href: string, linkText: string)}
  <div class="flex items-baseline justify-between mb-4">
    <h2 class="display text-lg font-normal m-0 tracking-tight">{title}</h2>
    <a {href} class="text-xs text-ink-soft hover:text-ink">{linkText}</a>
  </div>
{/snippet}
