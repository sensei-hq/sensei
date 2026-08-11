<script lang="ts">
    import type { MetricCardVM } from '$lib/metrics/metric-view.js';
    import MetricSparkline from './MetricSparkline.svelte';

    // A single metric tile — the overview stat-block anatomy (eyebrow · big
    // display value · sub) plus a trend chip and an optional sparkline. Pure
    // template: the value/trend/sub are already formatted by metric-view.
    let { card, series = [] }: { card: MetricCardVM; series?: number[] } = $props();

    const trendClass: Record<'good' | 'bad' | 'neutral', string> = {
        good: 'text-success',
        bad: 'text-danger',
        neutral: 'text-ink-mute',
    };
    const trendArrow: Record<'up' | 'down' | 'flat', string> = {
        up: '↑',
        down: '↓',
        flat: '→',
    };
</script>

<div data-component="metric-card" class="bg-paper-soft border border-paper-edge rounded-lg px-4 py-3">
    <div class="flex items-start justify-between gap-2 mb-1">
        <span class="text-xs tracking-wide uppercase text-ink-mute">{card.name}</span>
        {#if card.trend}
            <span
                data-component="metric-trend"
                data-tone={card.trend.tone}
                class="mono text-xs shrink-0 {trendClass[card.trend.tone]}"
            >{trendArrow[card.trend.dir]} {card.trend.label}</span>
        {/if}
    </div>

    <div class="flex items-end justify-between gap-3">
        <span data-component="metric-value" class="display text-2xl font-normal leading-none text-ink">
            {card.value}
        </span>
        {#if series.length > 1}
            <MetricSparkline {series} tone={card.trend?.tone ?? 'neutral'} />
        {/if}
    </div>

    {#if card.sub}
        <div data-component="metric-sub" class="text-xs text-ink-mute mt-1">{card.sub}</div>
    {/if}
</div>
