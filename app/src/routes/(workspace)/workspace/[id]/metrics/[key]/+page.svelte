<script lang="ts">
    import { grainUnit } from '$lib/metrics/metric-view';
    import { Kanji, Eyebrow, Screen } from '$lib/components';
    import {
        chartKindForType,
        formatMetricValue,
        TREND_TEXT,
        GRADE_CLASS,
    } from '$lib/metrics/metric-view.js';
    import SignalRail from '../SignalRail.svelte';
    import DetailChart from '../DetailChart.svelte';
    import AboutMetric from '../AboutMetric.svelte';
    import DatapointDrilldown from '../DatapointDrilldown.svelte';
    import ActionItems from '../ActionItems.svelte';
    import { MetricDetailState } from './metric-detail-state.svelte.js';
    import { metricsHost } from '../metrics-host.svelte.js';

    let { data } = $props();

    // The reads live in the workspace's metrics state (shared with the landing,
    // kept across metrics and grains); MetricDetailState derives the screen from
    // them and holds the chart window and the selected point.
    const host = metricsHost();
    const metrics = $derived(host.metrics);
    $effect(() => metrics.open());
    const grid = $derived(metrics.grid);
    const series = $derived(metrics.series(data.selectedKey, data.grain));
    // The datapoint drill-down is day-scoped: the daily series, the same read on daily.
    const daily = $derived(data.grain === 'daily' ? series : metrics.series(data.selectedKey, 'daily'));
    const recommendations = $derived(metrics.recommendations);
    const view = $derived({
        rows: grid.data?.rows ?? [],
        registry: grid.data?.registry ?? [],
        narrative: grid.data?.narrative ?? null,
        selectedKey: data.selectedKey,
        grain: data.grain,
        series: series.data ?? [],
        // The formula is definition-level: the registry always carries it.
        formula: grid.data?.registry.find((r) => r.key === data.selectedKey)?.formula ?? null,
        recommendations: recommendations.data ?? [],
    });
    const md = new MetricDetailState(() => view);
    const workspaceId = $derived(data.workspaceId);
    const selected = $derived(md.selected);

    const GRAINS = [
        { id: 'daily', label: 'Daily' },
        { id: 'weekly', label: 'Weekly' },
        { id: 'monthly', label: 'Monthly' },
    ];
</script>

<!-- A list-and-detail screen: the signal rail and the detail column scroll
     INDEPENDENTLY inside a `fill` body, so a long evidence panel never drags the
     rail off-screen (#293). -->
<Screen fill>
    {#snippet header()}
        <!-- Back link + the trail that tells you where in the signal set you are
             (mockup: "metrics · quality · churn concentration"). Lowercased in CSS,
             not in the data, so the family/metric names stay as the registry has them. -->
        <div class="flex items-baseline gap-4 pt-8 pb-4 px-6 md:px-10 max-w-[1040px]">
            <a
                href={`/workspace/${workspaceId}/metrics`}
                class="inline-flex items-center gap-1 text-xs text-ink-soft hover:text-accent no-underline"
            >← All signals</a>
            {#if selected}
                <div data-component="signal-breadcrumb" class="mono text-xs text-ink-faint lowercase truncate">
                    metrics · {selected.familyLabel} · {selected.name}
                </div>
            {/if}
        </div>
    {/snippet}
    <div class="flex-1 min-h-0 flex flex-col px-6 md:px-10 pb-6 max-w-[1040px]">
        {#if grid.status === 'error'}
            <div data-component="metrics-error" class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-6 text-sm text-ink-soft">
                Couldn’t load metrics — {grid.error}
            </div>
        {:else if grid.status === 'loading'}
            <div data-component="metrics-loading" aria-busy="true"
                class="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-[260px_1fr] bg-paper-soft border border-paper-edge rounded-lg overflow-hidden">
                <div class="border-r border-paper-edge p-6 flex flex-col gap-3">
                    {#each Array(8) as _, i (i)}<div class="h-8 rounded-md bg-paper animate-pulse"></div>{/each}
                </div>
                <div class="p-8 flex flex-col gap-6">
                    <div class="h-16 w-2/3 rounded-md bg-paper animate-pulse"></div>
                    <div class="h-64 rounded-md bg-paper animate-pulse"></div>
                </div>
                <span class="sr-only">Reading this workspace’s metrics</span>
            </div>
        {:else}
            <div
                data-component="signal-detail"
                data-signal={data.selectedKey}
                class="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-[260px_1fr] bg-paper-soft border border-paper-edge rounded-lg overflow-hidden"
            >
                <div class="overflow-auto min-h-0">
                    <SignalRail groups={md.groups} selectedKey={data.selectedKey} {workspaceId} distribution={md.distribution} format={md.format} />
                </div>

                <div class="overflow-auto min-h-0 p-6 md:p-8 flex flex-col gap-6">
                    {#if selected}
                        <div class="flex items-start justify-between gap-6 flex-wrap">
                            <div class="flex flex-col gap-1">
                                <Eyebrow>{selected.familyLabel}</Eyebrow>
                                <div class="display text-2xl font-light leading-tight text-ink">{selected.name}</div>
                                <!-- The value is the point of this screen, so it is the
                                     largest thing on it (mockup's hero "57%"). It read at
                                     body size against a text-2xl title, which inverted the
                                     hierarchy. Trend and comparison sit on its baseline. -->
                                <div class="flex items-baseline gap-3 pt-1">
                                    {#if selected.grade}
                                        <span
                                            class="self-center rounded px-1.5 py-0.5 text-sm font-medium leading-none {GRADE_CLASS[selected.grade]}"
                                            title="grade for the per-1,000-lines rate">{selected.grade}</span>
                                    {/if}
                                    <span class="display text-3xl font-light leading-none text-ink tabular-nums"
                                        >{selected.value}</span>
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
                                        href={`/workspace/${workspaceId}/metrics/${data.selectedKey}?grain=${g.id}`}
                                        data-grain={g.id}
                                        data-active={active}
                                        class="px-4 py-2 rounded-md text-sm no-underline transition-colors duration-fast {active ? 'bg-ink text-paper' : 'text-ink-soft hover:text-ink'}"
                                    >{g.label}</a>
                                {/each}
                            </div>
                        </div>

                        {#if md.about}
                            <AboutMetric about={md.about} howToReadSegments={md.howToReadSegments} {workspaceId} />
                        {/if}

                        {#if series.status === 'error'}
                            <p data-component="chart-error" class="bg-paper border border-paper-edge rounded-md px-4 py-6 text-sm text-ink-soft">
                                Couldn’t load this signal’s history — {series.error}
                            </p>
                        {:else if series.status === 'loading'}
                            <div data-component="chart-loading" aria-busy="true" class="h-64 rounded-md bg-paper animate-pulse"></div>
                        {:else}
                            <DetailChart
                                series={md.chartSeries}
                                yDomain={md.yDomain}
                                format={md.format}
                                color={selected.color}
                                caption={md.caption}
                                kind={chartKindForType(selected.type)}
                                selectedIndex={md.selectedIndex}
                                onselect={(i) => md.pick(i)}
                            />

                            {#if md.fullChartSeries.length > 2}
                                <div class="flex items-center gap-3 text-xs text-ink-soft px-1">
                                    <span class="shrink-0">Start</span>
                                    <input
                                        type="range"
                                        min="0"
                                        max={md.maxStart}
                                        value={md.effectiveStart}
                                        oninput={(e) => md.moveWindow(Number(e.currentTarget.value))}
                                        class="flex-1 h-1 cursor-pointer"
                                        style="accent-color: var(--accent)"
                                        aria-label="Chart start period"
                                        data-component="chart-window"
                                    />
                                    <span class="shrink-0 tabular-nums"
                                        >{md.chartSeries.length} of {md.fullChartSeries.length}
                                        {grainUnit(data.grain)}</span
                                    >
                                </div>
                            {/if}


                            <!-- Directly below the chart: the clicked point's sessions,
                                 observation, and transcript evidence (Phase C/F). -->
                            {#key data.selectedKey}
                                <DatapointDrilldown
                                    series={daily.data ?? []}
                                    seriesError={daily.error}
                                    workspaceId={workspaceId}
                                    metricKey={data.selectedKey}
                                    selectedDay={md.selectedDay}
                                />
                            {/key}
                        {/if}

                        <div class="flex flex-col gap-2">
                            <div class="flex items-center gap-2">
                                <Kanji char="察" size="sm" tone="accent" />
                                <Eyebrow>What sensei noticed</Eyebrow>
                            </div>
                            <p data-component="signal-insight" class="text-sm text-ink-mute leading-relaxed text-pretty">
                                {selected.insight}
                            </p>
                        </div>

                        <ActionItems items={md.actionItems} error={recommendations.error} />
                    {:else}
                        <div class="text-sm text-ink-soft py-8">
                            That signal isn’t computed for this workspace — pick one from the list.
                        </div>
                    {/if}
                </div>
            </div>
        {/if}
    </div>
</Screen>
