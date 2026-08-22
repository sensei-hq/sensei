<script lang="ts">
    import { page } from '$app/state';
    import { Kanji, Eyebrow } from '$lib/components';
    import {
        buildSignals,
        familyLookup,
        heroSignal,
        pickMovers,
        orderSignals,
        deterministicHeadline,
    } from '$lib/metrics/metric-view.js';
    import HealthHero from './HealthHero.svelte';
    import HealthRadar from './HealthRadar.svelte';
    import MoverCard from './MoverCard.svelte';
    import SignalGridCell from './SignalGridCell.svelte';
    import SignalLegend from './SignalLegend.svelte';

    let { data } = $props();

    const projectId = $derived(page.params.id ?? '');
    const signals = $derived(buildSignals(data.rows, familyLookup(data.registry), data.narrative));
    const hero = $derived(heroSignal(signals));
    const movers = $derived(pickMovers(signals));
    const ordered = $derived(orderSignals(signals));
    const headline = $derived(data.narrative?.headline ?? deterministicHeadline(signals));
</script>

<div class="pt-8 px-6 md:px-10 pb-12 max-w-[1040px]">
    {#if data.error}
        <div
            data-component="metrics-error"
            class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-mute"
        >
            Couldn’t load metrics — {data.error}
        </div>
    {:else if signals.length === 0}
        <div
            data-component="metrics-empty"
            class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-mute"
        >
            No metrics have been computed for this project yet.
        </div>
    {:else}
        <!-- Header names the screen; the card below carries the reading. Per the
             mockups (01-metrics-check.png, 02-ev.png) the h1 is the signal COUNT
             with the section glyph, and the narrative sentence lives in a bordered
             card beside the health readout — it was standing in as the h1, which
             left the screen with no title and the health score floating loose. -->
        <header data-component="metrics-header" class="flex items-baseline gap-4 mb-6">
            <Kanji char="測" size="screen" tone="accent" />
            <div class="flex flex-col gap-1">
                <Eyebrow>Project · metrics · this week</Eyebrow>
                <h1 class="display text-2xl md:text-3xl font-light leading-tight text-ink">
                    {signals.length} {signals.length === 1 ? 'signal' : 'signals'}
                </h1>
            </div>
        </header>

        <section
            data-component="metrics-narrative"
            class="mb-10 flex flex-col gap-6 rounded-lg border border-paper-edge bg-paper-soft p-6 md:flex-row md:items-start md:justify-between"
        >
            <div class="flex flex-col gap-3 max-w-[560px]">
                <p
                    data-component="metrics-headline"
                    class="display text-xl md:text-2xl font-light leading-snug text-ink text-pretty m-0"
                >
                    {headline}
                </p>
                {#if data.narrative?.subhead}
                    <p class="text-sm text-ink-soft leading-relaxed text-pretty m-0">
                        {data.narrative.subhead}
                    </p>
                {/if}
            </div>
            {#if hero}
                <HealthHero
                    signal={hero}
                    label={hero.key === 'ftr' ? 'First-turn resolution' : 'Health'}
                    series={data.series[hero.key] ?? []}
                />
            {/if}
        </section>

        {#if movers.length}
            <section data-component="metrics-movers" class="mb-10 flex flex-col gap-3">
                <div class="flex items-center gap-2">
                    <Kanji char="察" size="sm" tone="accent" />
                    <Eyebrow>What moved</Eyebrow>
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                    {#each movers as m (m.key)}
                        <MoverCard signal={m} series={data.series[m.key] ?? []} />
                    {/each}
                </div>
            </section>
        {/if}

        <!-- All signals at once, on the one scale they share (0-5 ratings).
             Sits above the per-metric grid: shape first, then the detail. -->
        <section data-component="metrics-radar" class="mb-10">
            <HealthRadar
                components={data.health?.components ?? {}}
                score={data.health?.health_score ?? null}
                ratedMetrics={data.health?.rated_metrics ?? 0}
            />
        </section>

        <section data-component="metrics-grid" class="flex flex-col gap-3">
            <div class="flex items-center justify-between gap-4 flex-wrap">
                <div class="flex items-center gap-2">
                    <Kanji char="観" size="sm" tone="muted" />
                    <!-- The count is the h1 now; repeating it here just says the
                         same number twice on one screen. -->
                    <Eyebrow>All signals</Eyebrow>
                </div>
                <SignalLegend />
            </div>
            <div class="grid grid-cols-2 lg:grid-cols-4 gap-px bg-paper-edge border border-paper-edge rounded-md overflow-hidden">
                {#each ordered as s (s.key)}
                    <SignalGridCell
                        signal={s}
                        series={data.series[s.key] ?? []}
                        href={`/project/${projectId}/metrics/${s.key}`}
                    />
                {/each}
            </div>
            <p class="text-xs text-ink-faint leading-relaxed">
                Movers this period are ordered first and carry a coloured rule. Several ratios rest on a
                denominator of one or two — sensei shows the shape, not a verdict.
            </p>
        </section>
    {/if}
</div>
