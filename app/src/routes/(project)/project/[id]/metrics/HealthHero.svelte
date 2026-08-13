<script lang="ts">
    import { Eyebrow, MetricSparkline } from '$lib/components';
    import { TREND_TEXT, type SignalVM } from '$lib/metrics/metric-view.js';

    // The composite/health signal, promoted to a hero readout: big score, its
    // change chip, and a wide 14-day sparkline with a coloured endpoint. Pure
    // template — the value/trend are pre-formatted by metric-view.
    let {
        signal,
        series = [],
        label = 'Health',
        periodLabel = 'last fourteen days',
    }: {
        signal: SignalVM;
        series?: (number | null)[];
        label?: string;
        periodLabel?: string;
    } = $props();
</script>

<div data-component="health-hero" class="flex flex-col items-end gap-1 shrink-0">
    <Eyebrow>{label}</Eyebrow>
    <div class="flex items-baseline gap-2">
        <span data-component="health-value" class="display text-4xl font-light leading-none text-ink tabular-nums tracking-tight">
            {signal.value}
        </span>
        {#if signal.trend}
            <span
                data-component="health-delta"
                data-tone={signal.trend.tone}
                class="mono text-sm {TREND_TEXT[signal.color]}"
            >{signal.trend.label}</span>
        {/if}
    </div>
    {#if series.length > 1}
        <MetricSparkline {series} tone="neutral" endDot={signal.color} width={240} height={48} />
    {/if}
    <div class="text-xs text-ink-faint">{periodLabel}</div>
</div>
