<script lang="ts">
    import { Plot } from '@rokkit/chart';
    import ChartCanvas from './ChartCanvas.svelte';
    import type { TrendTone, TrendColor } from '$lib/metrics/metric-view.js';

    // A tiny line sparkline for a metric's recent series, drawn with
    // @rokkit/chart's composable Plot.Line. Scale-free by design (min..max onto
    // the box) so any metric family reads at a glance. A `null` in the series is
    // an absent period and breaks the line (a real gap, never zero-filled); a
    // genuine 0 still plots. Colour follows the trend tone via currentColor +
    // a named token — no <style> colour blocks (app CLAUDE §1).
    let {
        series,
        tone = 'neutral',
        width = 88,
        height = 26,
        fill = false,
        endDot = null,
    }: {
        /** Recent values, one per period; `null` marks an absent period (a gap). */
        series: (number | null)[];
        tone?: TrendTone;
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
    const DOT_FILL: Record<TrendColor, string> = {
        success: 'var(--success)',
        accent: 'var(--accent)',
        'ink-faint': 'var(--ink-faint)',
    };

    const rows = $derived(series.map((v, i) => ({ i, v })));
    // A null element is a gap: @rokkit's Plot.Line breaks the path there via
    // d3's `.defined`. Its published `data` type omits null, so cast at the prop.
    const line = $derived(series.map((v, i) => (v == null ? null : { i, v })));
    const definedCount = $derived(series.filter((v) => v != null).length);
    const lastPt = $derived.by(() => {
        for (let i = series.length - 1; i >= 0; i--) {
            const v = series[i];
            if (v != null) return { i, v };
        }
        return null;
    });

    const MARGIN = { top: 2, right: 2, bottom: 2, left: 2 };
</script>

{#if definedCount >= 2}
    <ChartCanvas
        {rows}
        x="i"
        y="v"
        {width}
        {height}
        margin={MARGIN}
        dataComponent="metric-sparkline"
        dataTone={tone}
        class="{fill ? 'w-full' : 'shrink-0'} {toneClass[tone]}"
        preserveAspectRatio={fill ? 'none' : 'xMidYMid meet'}
        svgWidth={fill ? undefined : width}
        svgHeight={fill ? height : undefined}
    >
        <Plot.Line data={line as { i: number; v: number }[]} x="i" y="v" stroke="currentColor" strokeWidth={1.5} />
        {#if endDot && lastPt}
            <Plot.Point data={[lastPt]} x="i" y="v" fill={DOT_FILL[endDot]} r={3} />
        {/if}
    </ChartCanvas>
{/if}
