<script lang="ts">
    import { Kanji, Eyebrow, PageHeader, Screen } from '$lib/components';
    import { emptyMetricsNotice } from '$lib/metrics/empty-notice';
    import {
        buildSignals,
        familyLookup,
        pickMovers,
        groupSignals,
        deterministicHeadline,
    } from '$lib/metrics/metric-view.js';
    import { healthHeroReadout } from '$lib/metrics/health-radar.js';
    import HealthHero from './HealthHero.svelte';
    import HealthRadar from './HealthRadar.svelte';
    import MoverCard from './MoverCard.svelte';
    import CorrelationCard from './CorrelationCard.svelte';
    import SignalGridCell from './SignalGridCell.svelte';
    import SignalLegend from './SignalLegend.svelte';
    import { metricsHost } from './metrics-host.svelte.js';

    let { data } = $props();

    // A page state manages the reads (load() waited on ~32 before drawing): the
    // layout draws at once, each section fills as its read lands, and what was
    // read is kept for the details and the next visit.
    const host = metricsHost();
    const metrics = $derived(host.metrics);
    $effect(() => metrics.open());

    const workspaceId = $derived(data.workspaceId);
    const grid = $derived(metrics.grid);
    const narrative = $derived(grid.data?.narrative ?? null);
    const signals = $derived(grid.data ? buildSignals(grid.data.rows, familyLookup(grid.data.registry), narrative) : []);
    const health = $derived(metrics.health.data);
    const hero = $derived(healthHeroReadout(health));
    const movers = $derived(pickMovers(signals));
    const groups = $derived(groupSignals(signals));
    const headline = $derived(narrative?.headline ?? deterministicHeadline(signals));
    // Display name by key, so a card never shows a raw registry key.
    const nameOf = $derived((key: string) => signals.find((s) => s.key === key)?.name ?? key);
    // Cap the list: past a handful this stops being a reading and becomes a table.
    const correlations = $derived(metrics.correlations.slice(0, 6));
    const sparks = $derived(metrics.sparks);
</script>

<Screen>
    {#snippet header()}
        <!-- Header names the screen; the card below carries the reading. Per the
             mockups (01-metrics-check.png, 02-ev.png) the title is the signal COUNT
             with the section glyph, and the narrative sentence lives in a bordered
             card beside the health readout. With no signals the screen is still
             named. -->
        <div data-component="metrics-header">
            <PageHeader
                kanji="測"
                icon="i-solar-chart-2-bold-duotone"
                eyebrow="Workspace · metrics · this week"
                title={grid.status === 'ready' && signals.length > 0
                    ? `${signals.length} ${signals.length === 1 ? 'signal' : 'signals'}`
                    : 'Metrics'}
                size="lg"
            />
        </div>
    {/snippet}
    <div class="pt-6 px-6 md:px-10 pb-12 max-w-[1040px]">
        {#if metrics.history}
            <div
                data-component="metrics-history-off"
                class="mb-6 bg-warning-soft border border-paper-edge rounded-lg px-4 py-3 text-sm text-ink"
            >
                {metrics.history.text}
            </div>
        {/if}
        {#if grid.status === 'error'}
            <div
                data-component="metrics-error"
                class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-soft"
            >
                Couldn’t load metrics — {grid.error}
            </div>
        {:else if grid.status === 'loading'}
            <!-- The layout at once: the reading card and the grid's shape, filled as
                 the values and registry land. -->
            <div data-component="metrics-loading" aria-busy="true" class="flex flex-col gap-10">
                <div class="h-28 rounded-lg border border-paper-edge bg-paper-soft animate-pulse"></div>
                <div class="grid grid-cols-2 lg:grid-cols-4 gap-px bg-paper-edge border border-paper-edge rounded-md overflow-hidden">
                    {#each Array(8) as _, i (i)}
                        <div class="h-24 bg-paper animate-pulse"></div>
                    {/each}
                </div>
                <span class="sr-only">Reading this workspace’s metrics</span>
            </div>
        {:else if signals.length === 0}
            <!-- Why there are none: not computed yet, or no repository to compute from (#144). -->
            {@const empty = emptyMetricsNotice(data.workspace?.repos_count)}
            <div
                data-component="metrics-empty"
                data-empty-kind={empty.kind}
                class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-soft"
            >
                {empty.text}
            </div>
        {:else}

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
                    {#if narrative?.subhead}
                        <p class="text-sm text-ink-mute leading-relaxed text-pretty m-0">
                            {narrative.subhead}
                        </p>
                    {/if}
                </div>
                {#if hero}
                    <HealthHero value={hero.value} series={hero.series} delta={hero.delta} />
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
                            <MoverCard signal={m} series={sparks[m.key] ?? []} />
                        {/each}
                    </div>
                </section>
            {/if}

            <!-- All signals at once, on the one scale they share (0-5 ratings).
                 Sits above the per-metric grid: shape first, then the detail. -->
            <section data-component="metrics-radar" class="mb-10">
                <HealthRadar
                    components={health?.components ?? {}}
                    score={health?.health_score ?? null}
                    ratedMetrics={health?.rated_metrics ?? 0}
                />
            </section>

            {#if correlations.length}
                <!-- What moves with what. The daemon already drops pairs related by
                     CONSTRUCTION (tokens_in ⊂ tokens_per_day correlates 1.00 and says
                     nothing), so everything here is a relationship worth reading. -->
                <section data-component="metrics-correlations" class="mb-10 flex flex-col gap-3">
                    <div class="flex items-center gap-2">
                        <Kanji char="繋" size="sm" tone="accent" />
                        <Eyebrow>What moves together</Eyebrow>
                    </div>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                        {#each correlations as c (c.a + '|' + c.b)}
                            <CorrelationCard
                                correlation={c}
                                seriesA={sparks[c.a] ?? []}
                                seriesB={sparks[c.b] ?? []}
                                {nameOf}
                            />
                        {/each}
                    </div>
                </section>
            {/if}

            <!-- Grouped, and led by the signals that matter rather than by whatever
                 moved this week. Ordering by movement is why this read as an
                 undifferentiated wall — see KEY_SIGNAL_KEYS. -->
            <section data-component="metrics-grid" class="flex flex-col gap-6">
                <div class="flex items-center justify-between gap-4 flex-wrap">
                    <div class="flex items-center gap-2">
                        <Kanji char="観" size="sm" tone="muted" />
                        <!-- The count is the h1 now; repeating it here just says the
                             same number twice on one screen. -->
                        <Eyebrow>All signals</Eyebrow>
                    </div>
                    <SignalLegend />
                </div>

                {#each groups as g (g.id)}
                    <div data-component="signal-group" data-group={g.id} class="flex flex-col gap-2">
                        <Eyebrow>{g.label}</Eyebrow>
                        <div class="grid grid-cols-2 lg:grid-cols-4 gap-px bg-paper-edge border border-paper-edge rounded-md overflow-hidden">
                            {#each g.signals as s (s.key)}
                                <SignalGridCell
                                    signal={s}
                                    series={sparks[s.key] ?? []}
                                    href={`/workspace/${workspaceId}/metrics/${s.key}`}
                                />
                            {/each}
                        </div>
                    </div>
                {/each}
                <p class="text-xs text-ink-faint leading-relaxed">
                    Key signals lead; the rest are grouped by what they measure, movers first within each
                    group and carrying a coloured rule. Several ratios rest on a denominator of one or two
                    — sensei shows the shape, not a verdict.
                </p>
            </section>
        {/if}
    </div>
</Screen>
