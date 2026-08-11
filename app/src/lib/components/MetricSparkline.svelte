<script lang="ts">
    import type { TrendTone } from '$lib/metrics/metric-view.js';

    // A tiny normalized line sparkline for a metric's recent series. Scale-free:
    // it maps min..max of the given points onto the box, so any metric family
    // (pct, count, duration…) reads at a glance. Colour follows the trend tone
    // via a named token (no <style> colour blocks — app CLAUDE.md §1).
    let {
        series,
        tone = 'neutral',
        width = 88,
        height = 26,
    }: {
        series: number[];
        tone?: TrendTone;
        width?: number;
        height?: number;
    } = $props();

    const toneClass: Record<TrendTone, string> = {
        good: 'text-success',
        bad: 'text-danger',
        neutral: 'text-ink-faint',
    };

    const points = $derived(buildPoints(series, width, height));

    function buildPoints(data: number[], w: number, h: number): string {
        if (data.length < 2) return '';
        const min = Math.min(...data);
        const max = Math.max(...data);
        const span = max - min || 1;
        const pad = 2;
        return data
            .map((v, i) => {
                const x = (i / (data.length - 1)) * w;
                const y = pad + (1 - (v - min) / span) * (h - pad * 2);
                return `${x.toFixed(1)},${y.toFixed(1)}`;
            })
            .join(' ');
    }
</script>

{#if points}
    <svg
        data-component="metric-sparkline"
        data-tone={tone}
        {width}
        {height}
        viewBox="0 0 {width} {height}"
        fill="none"
        aria-hidden="true"
        class="shrink-0 {toneClass[tone]}"
    >
        <polyline
            {points}
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linejoin="round"
            stroke-linecap="round"
        />
    </svg>
{/if}
