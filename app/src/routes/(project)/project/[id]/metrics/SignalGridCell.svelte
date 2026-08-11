<script lang="ts">
    import { MetricSparkline } from '$lib/components';
    import { TREND_TEXT, TREND_RULE, type SignalVM } from '$lib/metrics/metric-view.js';

    // One cell of the uniform "all signals" grid: a top rule coloured by trend
    // (mover → accent/success, flat → hairline), the metric name, its value +
    // delta, and a full-width sparkline. The whole cell links to the signal's
    // detail view. Pure template.
    let { signal, series = [], href }: { signal: SignalVM; series?: number[]; href: string } = $props();
</script>

<a
    {href}
    data-component="signal-cell"
    data-signal={signal.key}
    data-moved={signal.moved}
    class="signal-cell bg-paper p-4 flex flex-col gap-2 border-t-2 {TREND_RULE[signal.color]} no-underline text-inherit transition-colors duration-fast"
>
    <div class="text-xs text-ink-mute leading-snug">{signal.name}</div>
    <div class="flex items-baseline gap-2">
        <span data-component="signal-value" class="text-ink tabular-nums">{signal.value}</span>
        {#if signal.trend}
            <span data-component="signal-delta" class="mono text-xs {TREND_TEXT[signal.color]}">{signal.trend.label}</span>
        {/if}
    </div>
    {#if series.length > 1}
        <MetricSparkline {series} tone={signal.trend?.tone ?? 'neutral'} fill height={28} />
    {/if}
</a>

<style>
    .signal-cell:hover {
        background: var(--paper-soft);
    }
</style>
