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
        <!-- Headline · interpreted movers · grid beneath: read the meaning first,
             scan the shapes second. -->
        <header
            data-component="metrics-header"
            class="flex flex-col md:flex-row md:items-start md:justify-between gap-6 mb-10"
        >
            <div class="flex flex-col gap-2 max-w-[560px]">
                <Eyebrow>Project · metrics · this week</Eyebrow>
                <h1 data-component="metrics-headline" class="display text-2xl md:text-3xl font-light leading-tight text-ink text-pretty">
                    {headline}
                </h1>
                {#if data.narrative?.subhead}
                    <p class="text-sm text-ink-soft leading-relaxed text-pretty">{data.narrative.subhead}</p>
                {/if}
            </div>
            {#if hero}
                <HealthHero
                    signal={hero}
                    label={hero.key === 'ftr' ? 'First-turn resolution' : 'Health'}
                    series={data.series[hero.key] ?? []}
                />
            {/if}
        </header>

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

        <section data-component="metrics-grid" class="flex flex-col gap-3">
            <div class="flex items-center justify-between gap-4 flex-wrap">
                <div class="flex items-center gap-2">
                    <Kanji char="観" size="sm" tone="muted" />
                    <Eyebrow>All {signals.length} signals</Eyebrow>
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
