<script lang="ts">
    import { MetricSparkline } from '$lib/components';
    import { TREND_TEXT, type SignalVM } from '$lib/metrics/metric-view.js';

    // A "what moved" card: a sparkline on the left, then value · delta · name on
    // one baseline and the interpretive sentence beneath. The sentence is the
    // ollama-generated insight when present, else the data-grounded fallback —
    // both already resolved onto signal.insight by metric-view.
    let { signal, series = [] }: { signal: SignalVM; series?: (number | null)[] } = $props();
</script>

<div
    data-component="mover-card"
    data-signal={signal.key}
    class="flex gap-4 items-start p-4 bg-paper border border-paper-edge rounded-md"
>
    {#if series.length > 1}
        <MetricSparkline {series} tone={signal.trend?.tone ?? 'neutral'} width={88} height={40} />
    {/if}
    <div class="flex flex-col gap-1 min-w-0">
        <div class="flex items-baseline gap-2 flex-wrap">
            <span class="text-ink tabular-nums">{signal.value}</span>
            {#if signal.trend}
                <span data-component="mover-delta" class="mono text-xs {TREND_TEXT[signal.color]}">{signal.trend.label}</span>
            {/if}
            <span class="text-sm text-ink-soft">{signal.name}</span>
        </div>
        <div data-component="mover-insight" class="text-sm text-ink-soft leading-relaxed text-pretty">
            {signal.insight}
        </div>
    </div>
</div>
