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
        kind = 'line',
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
        /** `bar` for discrete/sporadic COUNT metrics (work happens in bursts, so a line
         *  implies a continuity that isn't there); `line` for rates/scores/durations. */
        kind?: 'line' | 'bar';
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
    // line/bar builder skips it (d3 `.defined`) instead of diving to 0. `vt` is a
    // carry-forward-filled copy used ONLY by the moving-average trend (bars/line stay
    // gap-aware via `v`): a trend inherently spans gaps, and rokkit's Plot.Trend reads
    // `Number(y)` where `Number(null) === 0` would drag the line to the floor at every
    // gap — carrying the last known value keeps the average honest across absent periods.
    const rows = $derived.by(() => {
        let firstDefined: number | null = null;
        for (const v of series) if (v != null) { firstDefined = v; break; }
        let last = firstDefined; // leading gaps carry the first defined value
        return series.map((v, i) => {
            if (v != null) last = v;
            return { i, v, vt: last };
        });
    });
    const definedCount = $derived(series.filter((v) => v != null).length);
    // A trailing simple moving average needs a few points to be meaningful; below this
    // the raw bars already tell the whole story, so the trend is omitted.
    const TREND_WINDOW = 3;
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
        {#if kind === 'bar'}
            <!-- Discrete counts as gap-aware bars (a null period draws no bar), so
                 sporadic work reads as bursts, not a misleading continuous line. A
                 dashed moving-average trend (rokkit Plot.Trend) rides over the bars so
                 the direction of travel is legible through the noise; it reads the
                 carry-forward `vt` channel so gaps don't drag it to zero. -->
            <Plot.Bar x="i" y="v" fill="currentColor" />
            {#if definedCount > TREND_WINDOW}
                <Plot.Trend x="i" y="vt" trend={{ type: 'ma', window: TREND_WINDOW }} />
            {/if}
        {:else}
            <Plot.Line x="i" y="v" color="currentColor" options={{ strokeWidth: 1.5 }} />
            {#if endDot && lastIndex != null}
                <Plot.Highlight x="i" y="v" highlight={lastIndex} />
            {/if}
        {/if}
    </ChartCanvas>
{/if}
