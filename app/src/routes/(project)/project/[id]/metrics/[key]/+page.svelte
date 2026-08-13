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

    // Project-scoped action items — the pending recommendations, score-ranked by
    // the wire and mapped to view-models by the pure helper (honest-empty when
    // none). Not keyed on the selected metric: the panel is project-level.
    const actionItems = $derived(buildActionItems(data.recommendations));

    const GRAINS = [
        { id: 'daily', label: 'Daily' },
        { id: 'weekly', label: 'Weekly' },
        { id: 'monthly', label: 'Monthly' },
    ];
</script>

<div class="pt-8 px-6 md:px-10 pb-12 max-w-[1040px]">
    <a
        href={`/project/${projectId}/metrics`}
        class="inline-flex items-center gap-1 text-xs text-ink-mute hover:text-accent no-underline mb-4"
    >← All signals</a>

    {#if data.error}
        <div data-component="metrics-error" class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-mute">
            Couldn’t load metrics — {data.error}
        </div>
    {:else}
        <div
            data-component="signal-detail"
            data-signal={data.selectedKey}
            class="grid grid-cols-1 md:grid-cols-[260px_1fr] bg-paper-soft border border-paper-edge rounded-lg overflow-hidden"
        >
            <SignalRail signals={ordered} selectedKey={data.selectedKey} {projectId} {distribution} {format} />

            <div class="p-6 md:p-8 flex flex-col gap-6">
                {#if selected}
                    <div class="flex items-start justify-between gap-6 flex-wrap">
                        <div class="flex flex-col gap-1">
                            <Eyebrow>{selected.familyLabel}</Eyebrow>
                            <div class="display text-2xl font-light leading-tight text-ink">{selected.name}</div>
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
                        />
                    {/if}

                    {#if about}
                        <AboutMetric {about} {howToReadSegments} {projectId} />
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

                    {#key data.selectedKey}
                        <DatapointDrilldown
                            series={data.dailySeries}
                            seriesError={data.dailySeriesError}
                            projectId={projectId}
                            metricKey={data.selectedKey}
                        />
                    {/key}

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
