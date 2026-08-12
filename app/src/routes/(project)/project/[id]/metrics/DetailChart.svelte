<script lang="ts">
    import { Plot } from '@rokkit/chart';
    import ChartCanvas from '$lib/components/ChartCanvas.svelte';
    import type { TrendColor, ChartPoint } from '$lib/metrics/metric-view.js';

    // The drill-down trend chart: an area + line over the selected metric's
    // series, drawn with @rokkit/chart's composable Plot geoms. The series is
    // already densified by metric-view — absent periods carry `value: null`, so
    // Plot.Line / Plot.Area break the path there (a real gap, never zero-filled)
    // while a genuine 0 still plots. A fixed per-metric y-domain keeps a flat
    // series looking flat rather than a mountain. Pure template.
    let {
        series,
        yDomain,
        format,
        color = 'accent',
        caption = '',
    }: {
        series: ChartPoint[];
        yDomain: [number, number];
        format: (v: number) => string;
        color?: TrendColor;
        caption?: string;
    } = $props();

    // Named-token fills (never raw hex): area by trend tone, endpoint dot likewise.
    const AREA_FILL: Record<TrendColor, string> = {
        success: 'var(--success-soft)',
        accent: 'var(--accent-soft)',
        'ink-faint': 'var(--paper-mute)',
    };
    const DOT_FILL: Record<TrendColor, string> = {
        success: 'var(--success)',
        accent: 'var(--accent)',
        'ink-faint': 'var(--ink-faint)',
    };

    // x is the slot index (one per densified period) → a continuous scale whose
    // spacing is proportional to time, so a long lull reads as a wide gap.
    const rows = $derived(series.map((p, i) => ({ t: i, v: p.value })));
    // A null element is a gap: @rokkit's Plot.Line / Plot.Area break the path
    // there via d3's `.defined`. Their `data` type omits null, so cast at the prop.
    const line = $derived(series.map((p, i) => (p.value == null ? null : { t: i, v: p.value })));
    const definedCount = $derived(series.filter((p) => p.value != null).length);
    const lastPt = $derived.by(() => {
        for (let i = series.length - 1; i >= 0; i--) {
            if (series[i].value != null) return { t: i, v: series[i].value as number };
        }
        return null;
    });

    const fmtY = (v: unknown): string => format(Number(v));

    // Logical plot box; CSS scales it responsively (aspect-preserving, no distortion).
    const W = 720;
    const H = 200;
    const MARGIN = { top: 12, right: 14, bottom: 14, left: 48 };
</script>

<div data-component="detail-chart" class="bg-paper border border-paper-edge rounded-md p-4 flex flex-col gap-2">
    {#if caption}<div class="text-xs text-ink-mute">{caption}</div>{/if}
    {#if definedCount >= 2}
        <ChartCanvas
            {rows}
            x="t"
            y="v"
            {yDomain}
            width={W}
            height={H}
            margin={MARGIN}
            class="w-full text-ink-faint"
        >
            <Plot.Grid />
            <Plot.Area data={line as { t: number; v: number }[]} x="t" y="v" fill={AREA_FILL[color]} opacity={0.6} />
            <Plot.Line data={line as { t: number; v: number }[]} x="t" y="v" stroke="var(--ink)" strokeWidth={1.5} />
            {#if lastPt}
                <Plot.Point data={[lastPt]} x="t" y="v" fill={DOT_FILL[color]} r={4} />
            {/if}
            <Plot.Axis type="y" ticks={4} format={fmtY} showLine={false} showTicks={false} />
        </ChartCanvas>
    {:else}
        <div class="text-sm text-ink-mute py-8 text-center">Not enough history to chart yet.</div>
    {/if}
</div>
