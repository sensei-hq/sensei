<script lang="ts">
    import { Eyebrow, MetricSparkline } from '$lib/components';
    import { TREND_TEXT, type TrendColor } from '$lib/metrics/metric-view.js';

    // The composite health score, promoted to a hero readout: big score, its
    // week-over-week change, and a sparkline of the weekly series.
    //
    // It reads the score from the `project_health_score` / `project_health_trend`
    // VIEWS, not from a metric row. The old stored `project_health` metric is
    // retired (it produced zero rows), so the previous "find the composite signal"
    // fallback silently landed on FTR — which is why this card said "First-turn
    // resolution" where the mockup says "HEALTH". Pure template: the value, delta
    // and series are shaped by `healthHeroReadout`.
    let {
        value,
        series = [],
        delta = null,
        label = 'Health',
        periodLabel = 'weekly, last twelve',
    }: {
        /** Pre-formatted score. */
        value: string;
        series?: number[];
        /** Week-over-week change; null when there is no prior week to compare. */
        delta?: number | null;
        label?: string;
        periodLabel?: string;
    } = $props();

    // Health is higher-better, so a rise is good. Flat (0) stays neutral rather
    // than being coloured as an improvement.
    const tone = $derived<TrendColor>(
        delta == null || delta === 0 ? 'ink-faint' : delta > 0 ? 'success' : 'accent',
    );
    const deltaLabel = $derived(
        delta == null || delta === 0 ? null : `${delta > 0 ? '+' : '−'}${Math.abs(delta)}`,
    );
</script>

<div data-component="health-hero" class="flex flex-col items-end gap-1 shrink-0">
    <Eyebrow>{label}</Eyebrow>
    <div class="flex items-baseline gap-2">
        <span
            data-component="health-value"
            class="display text-4xl font-light leading-none text-ink tabular-nums tracking-tight"
        >{value}</span>
        {#if deltaLabel}
            <span data-component="health-delta" data-tone={tone} class="mono text-sm {TREND_TEXT[tone]}"
                >{deltaLabel}</span
            >
        {/if}
    </div>
    {#if series.length > 1}
        <MetricSparkline {series} tone="neutral" endDot={tone} width={240} height={48} />
    {/if}
    <div class="text-xs text-ink-faint">{periodLabel}</div>
</div>
