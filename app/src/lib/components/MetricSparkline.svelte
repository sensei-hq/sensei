<script lang="ts">
    import { Plot } from '@rokkit/chart';
    import ChartCanvas from './ChartCanvas.svelte';
    import type { TrendTone, TrendColor } from '$lib/metrics/metric-view.js';

    // A tiny line sparkline for a metric's recent series, drawn with @rokkit/chart's
    // composable Plot.Line. Scale-free by design (min..max onto the box) so any metric
    // family reads at a glance. A `null` in the series is an absent period and breaks
    // the line (a real gap, never zero-filled — @rokkit's line builder skips a null y via
    // d3 `.defined`); a genuine 0 still plots. Colour follows the trend tone via
    // currentColor + a named token — no <style> colour blocks (app CLAUDE §1). The
    // endpoint dot (health hero) is drawn with Plot.Highlight, tinted by a chart CSS var.
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

    // @rokkit/chart reads the series from the canvas' PlotState. A null v is a gap: the
    // line builder breaks the path there (d3 `.defined`) instead of diving to 0.
    const rows = $derived(series.map((v, i) => ({ i, v })));
    const definedCount = $derived(series.filter((v) => v != null).length);
    // The last defined slot — the endpoint the health-hero dot marks (never a trailing gap).
    const lastIndex = $derived.by(() => {
        for (let i = series.length - 1; i >= 0; i--) {
            if (series[i] != null) return i;
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
        style={endDot ? `--chart-highlight-color: ${DOT_FILL[endDot]}; --chart-highlight-radius: 3` : undefined}
    >
        <Plot.Line x="i" y="v" color="currentColor" options={{ strokeWidth: 1.5 }} />
        {#if endDot && lastIndex != null}
            <Plot.Highlight x="i" y="v" highlight={lastIndex} />
        {/if}
    </ChartCanvas>
{/if}
