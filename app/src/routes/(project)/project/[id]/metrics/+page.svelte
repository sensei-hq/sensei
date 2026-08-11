<script lang="ts">
    import { MetricCard, Kanji, Eyebrow } from '$lib/components';
    import { groupByFamily, familyLookup } from '$lib/metrics/metric-view.js';

    let { data } = $props();

    const sections = $derived(groupByFamily(data.rows, familyLookup(data.registry)));
</script>

<div class="pt-8 px-6 md:px-10 pb-12 max-w-[960px]">
    <header class="mb-8">
        <div class="flex items-baseline gap-3 mb-2">
            <Kanji char="計" size="2xl" tone="accent" />
            <Eyebrow>Project · metrics</Eyebrow>
        </div>
        <h1 class="display text-2xl font-normal leading-none text-ink">How this project is trending</h1>
        <p class="text-sm text-ink-mute mt-2">
            Outcome, velocity, quality and knowledge signals — each with its recent trend.
        </p>
    </header>

    {#if data.error}
        <div
            data-component="metrics-error"
            class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-mute"
        >
            Couldn’t load metrics — {data.error}
        </div>
    {:else if sections.length === 0}
        <div
            data-component="metrics-empty"
            class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-mute"
        >
            No metrics have been computed for this project yet.
        </div>
    {:else}
        {#each sections as section (section.family)}
            <section class="mb-10">
                <div class="text-xs tracking-wide uppercase text-ink-mute mb-3">{section.label}</div>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    {#each section.cards as card (card.key)}
                        <MetricCard {card} series={data.series[card.key] ?? []} />
                    {/each}
                </div>
            </section>
        {/each}
    {/if}
</div>
