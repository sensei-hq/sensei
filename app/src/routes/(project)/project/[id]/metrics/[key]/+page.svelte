<script lang="ts">
    import { page } from '$app/state';
    import { Kanji, Eyebrow } from '$lib/components';
    import {
        buildSignals,
        familyLookup,
        orderSignals,
        seriesValues,
        seriesDistribution,
        densifySeries,
        metricYDomain,
        chartKindForType,
        historyNote,
        metricAbout,
        linkifyMetrics,
        formatMetricValue,
        TREND_TEXT,
    } from '$lib/metrics/metric-view.js';
    import SignalRail from '../SignalRail.svelte';
    import DetailChart from '../DetailChart.svelte';
    import AboutMetric from '../AboutMetric.svelte';
    import DatapointDrilldown from '../DatapointDrilldown.svelte';
    import ActionItems from '../ActionItems.svelte';
    import { buildActionItems } from '../action-items.js';

    let { data } = $props();

    const projectId = $derived(page.params.id ?? '');
    const signals = $derived(buildSignals(data.rows, familyLookup(data.registry), data.narrative));
    const ordered = $derived(orderSignals(signals));
    const selected = $derived(signals.find((s) => s.key === data.selectedKey) ?? null);
    // The static "about this metric" reference (purpose / how-to-read / formula),
    // from the selected metric's registry row + the series' formula facet.
    const selectedRow = $derived(data.rows.find((r) => r.metric === data.selectedKey));
    const about = $derived(metricAbout(selectedRow, data.formula));
    // Linkify any companion metric named in `how_to_read` (e.g. FTR's
    // "Companion: rework ratio") to that metric's detail.
    const howToReadSegments = $derived(
        about ? linkifyMetrics(about.howToRead, data.registry, data.selectedKey) : [],
    );

    const values = $derived(seriesValues(data.series));
    const distribution = $derived(seriesDistribution(values));
    const format = $derived((v: number) => formatMetricValue(selected?.type ?? 'count', v));

    // Densified {date,value|null}[] for the chart (absent periods → gaps) and a
    // fixed y-domain per metric type (a flat series stays flat, not a mountain).
    const chartSeries = $derived(densifySeries(data.series, data.grain));
    const yDomain = $derived(metricYDomain(selected?.type ?? 'count', values));
    const note = $derived(historyNote(chartSeries));
    const caption = $derived(note ? `${note} · ${data.grain}` : '');

    // "About this metric" is a popover (info button), not interleaved with the
    // real content — it's reference material, opened on demand.
    let aboutOpen = $state(false);

    // The selected datapoint index — defaults to the latest defined point, and a
    // chart click moves it. Drives the highlight + the evidence panel's day.
    let selectedIndex = $state<number | null>(null);
    $effect(() => {
        // Re-default when the series changes (metric / grain switch): last point.
        let last: number | null = null;
        for (let i = chartSeries.length - 1; i >= 0; i--) {
            if (chartSeries[i].value != null) { last = i; break; }
        }
        selectedIndex = last;
    });
    // The selected point's day — only at daily grain, where a chart slot IS a day
    // the drill-down can load; at coarser grains the drill-down keeps its own
    // day selector (a week/month start isn't a daily session date).
    const selectedDay = $derived(
        data.grain === 'daily' && selectedIndex != null
            ? (chartSeries[selectedIndex]?.date ?? null)
            : null,
    );

    // Phase E: metric-specific actions — the pending recommendations relevant to
    // THIS metric (heuristic keyword match on its key / name / family), replacing
    // the old project-wide panel that duplicated Impact. Honest-empty when none
    // are specific to this metric.
    const metricTerms = $derived(
        [data.selectedKey, selected?.name, selected?.familyLabel]
            .filter((t): t is string => !!t)
            .map((t) => t.toLowerCase()),
    );
    const relevantRecs = $derived(
        data.recommendations.filter((r) => {
            const hay = `${r.title ?? ''} ${r.why ?? ''} ${r.impact ?? ''}`.toLowerCase();
            return metricTerms.some((t) => hay.includes(t));
        }),
    );
    const actionItems = $derived(buildActionItems(relevantRecs));

    const GRAINS = [
        { id: 'daily', label: 'Daily' },
        { id: 'weekly', label: 'Weekly' },
        { id: 'monthly', label: 'Monthly' },
    ];
</script>

<!-- Fills <main> and owns its own scroll: the signal rail and the detail column
     scroll INDEPENDENTLY (min-h-0 + overflow-auto), so a long evidence panel never
     drags the rail off-screen. -->
<div class="h-full flex flex-col overflow-hidden pt-8 px-6 md:px-10 pb-6 max-w-[1040px]">
    <a
        href={`/project/${projectId}/metrics`}
        class="inline-flex items-center gap-1 text-xs text-ink-mute hover:text-accent no-underline mb-4 shrink-0"
    >← All signals</a>

    {#if data.error}
        <div data-component="metrics-error" class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-mute">
            Couldn’t load metrics — {data.error}
        </div>
    {:else}
        <div
            data-component="signal-detail"
            data-signal={data.selectedKey}
            class="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-[260px_1fr] bg-paper-soft border border-paper-edge rounded-lg overflow-hidden"
        >
            <div class="overflow-auto min-h-0">
                <SignalRail signals={ordered} selectedKey={data.selectedKey} {projectId} {distribution} {format} />
            </div>

            <div class="overflow-auto min-h-0 p-6 md:p-8 flex flex-col gap-6">
                {#if selected}
                    <div class="relative flex items-start justify-between gap-6 flex-wrap">
                        <div class="flex flex-col gap-1">
                            <Eyebrow>{selected.familyLabel}</Eyebrow>
                            <div class="flex items-center gap-2">
                                <div class="display text-2xl font-light leading-tight text-ink">{selected.name}</div>
                                {#if about}
                                    <button
                                        type="button"
                                        data-component="about-toggle"
                                        aria-expanded={aboutOpen}
                                        aria-label="About this metric"
                                        title="About this metric"
                                        onclick={() => (aboutOpen = !aboutOpen)}
                                        class="w-5 h-5 shrink-0 rounded-full border text-xs leading-none transition-colors duration-fast {aboutOpen
                                            ? 'border-accent text-accent'
                                            : 'border-paper-edge text-ink-mute hover:text-accent hover:border-accent'}"
                                    >i</button>
                                {/if}
                            </div>
                            <div class="flex items-baseline gap-3 pt-1">
                                <span class="text-ink tabular-nums">{selected.value}</span>
                                {#if selected.trend}
                                    <span class="mono text-sm {TREND_TEXT[selected.color]}">{selected.trend.label}</span>
                                {/if}
                                {#if selected.sub}<span class="text-sm text-ink-faint">{selected.sub}</span>{/if}
                            </div>
                        </div>

                        <div class="flex gap-1 p-1 bg-paper border border-paper-edge rounded-md">
                            {#each GRAINS as g (g.id)}
                                {@const active = data.grain === g.id}
                                <a
                                    href={`/project/${projectId}/metrics/${data.selectedKey}?grain=${g.id}`}
                                    data-grain={g.id}
                                    data-active={active}
                                    class="px-4 py-2 rounded-md text-sm no-underline transition-colors duration-fast {active ? 'bg-ink text-paper' : 'text-ink-mute hover:text-ink'}"
                                >{g.label}</a>
                            {/each}
                        </div>

                        {#if aboutOpen && about}
                            <div
                                data-component="about-popover"
                                class="absolute right-0 top-full mt-2 z-20 w-[22rem] max-w-[90vw] bg-paper border border-paper-edge rounded-lg shadow-md p-4"
                            >
                                <AboutMetric {about} {howToReadSegments} {projectId} />
                            </div>
                        {/if}
                    </div>

                    {#if data.seriesError}
                        <p data-component="chart-error" class="bg-paper border border-paper-edge rounded-md px-4 py-6 text-sm text-ink-mute">
                            Couldn’t load this signal’s history — {data.seriesError}
                        </p>
                    {:else}
                        <DetailChart
                            series={chartSeries}
                            {yDomain}
                            {format}
                            color={selected.color}
                            {caption}
                            kind={chartKindForType(selected.type)}
                            {selectedIndex}
                            onselect={(i) => (selectedIndex = i)}
                        />

                        <!-- Directly below the chart: the clicked point's sessions,
                             observation, and transcript evidence (Phase C/F). -->
                        {#key data.selectedKey}
                            <DatapointDrilldown
                                series={data.dailySeries}
                                seriesError={data.dailySeriesError}
                                projectId={projectId}
                                metricKey={data.selectedKey}
                                {selectedDay}
                            />
                        {/key}
                    {/if}

                    <div class="flex flex-col gap-2">
                        <div class="flex items-center gap-2">
                            <Kanji char="察" size="sm" tone="accent" />
                            <Eyebrow>What sensei noticed</Eyebrow>
                        </div>
                        <p data-component="signal-insight" class="text-sm text-ink-soft leading-relaxed text-pretty">
                            {selected.insight}
                        </p>
                    </div>

                    <ActionItems items={actionItems} error={data.recommendationsError} />
                {:else}
                    <div class="text-sm text-ink-mute py-8">
                        That signal isn’t computed for this project — pick one from the list.
                    </div>
                {/if}
            </div>
        </div>
    {/if}
</div>
