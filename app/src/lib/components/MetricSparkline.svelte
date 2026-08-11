<script lang="ts">
    import type { TrendTone, TrendColor } from '$lib/metrics/metric-view.js';

    // A tiny normalized line sparkline for a metric's recent series. Scale-free:
    // it maps min..max of the given points onto the box, so any metric family
    // (pct, count, duration…) reads at a glance. Colour follows the trend tone
    // via a named token — matching the metrics legend (worsening → accent, not
    // danger). No <style> colour blocks (app CLAUDE.md §1).
    let {
        series,
        tone = 'neutral',
        width = 88,
        height = 26,
        fill = false,
        endDot = null,
    }: {
        series: number[];
        tone?: TrendTone;
        /** Viewbox width; ignored for layout when `fill` (stretches to parent). */
        width?: number;
        height?: number;
        /** Stretch to the parent's width (grid cells). */
        fill?: boolean;
        /** Draw a dot at the last point in this colour (health hero endpoint). */
        endDot?: TrendColor | null;
    } = $props();

    // Legend colours: improving → success, worsening → accent, flat → ink-faint.
    const toneClass: Record<TrendTone, string> = {
        good: 'text-success',
        bad: 'text-accent',
        neutral: 'text-ink-faint',
    };
    const dotClass: Record<TrendColor, string> = {
        success: 'fill-success',
        accent: 'fill-accent',
        'ink-faint': 'fill-ink-faint',
    };

    const coords = $derived(buildCoords(series, width, height));
    const points = $derived(coords.map((c) => `${c.x},${c.y}`).join(' '));
    const last = $derived(coords.at(-1) ?? null);

    function buildCoords(data: number[], w: number, h: number): { x: number; y: number }[] {
        if (data.length < 2) return [];
        const min = Math.min(...data);
        const max = Math.max(...data);
        const span = max - min || 1;
        const pad = 2;
        return data.map((v, i) => ({
            x: +((i / (data.length - 1)) * w).toFixed(1),
            y: +(pad + (1 - (v - min) / span) * (h - pad * 2)).toFixed(1),
        }));
    }
</script>

{#if points}
    <svg
        data-component="metric-sparkline"
        data-tone={tone}
        height={fill ? height : undefined}
        width={fill ? undefined : width}
        viewBox="0 0 {width} {height}"
        preserveAspectRatio={fill ? 'none' : 'xMidYMid meet'}
        fill="none"
        aria-hidden="true"
        class="{fill ? 'w-full' : 'shrink-0'} {toneClass[tone]}"
    >
        <polyline
            {points}
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linejoin="round"
            stroke-linecap="round"
            vector-effect="non-scaling-stroke"
        />
        {#if endDot && last}
            <circle cx={last.x} cy={last.y} r="3" class={dotClass[endDot]} stroke="none" />
        {/if}
    </svg>
{/if}
