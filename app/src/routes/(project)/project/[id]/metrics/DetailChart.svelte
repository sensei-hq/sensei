<script lang="ts">
    import { Plot } from '@rokkit/chart';
    import ChartCanvas from '$lib/components/ChartCanvas.svelte';
    import { movingAverage, type TrendColor, type ChartPoint, type ChartKind } from '$lib/metrics/metric-view.js';

    // The drill-down trend chart: an area + line over the selected metric's
    // series, drawn with @rokkit/chart's composable Plot geoms, plus an
    // INTERACTIVE overlay — every datapoint is clickable to drive the evidence
    // panel below, and the selected point is highlighted with a vertical guide.
    // The series is already densified by metric-view — absent periods carry
    // `value: null`, so Plot.Line / Plot.Area break the path there (a real gap,
    // never zero-filled) while a genuine 0 still plots. The overlay geometry uses
    // the SAME linear scales rokkit builds (slot index → x, fixed y-domain → y),
    // so the manual marks land exactly on the rokkit path. Pure template.
    let {
        series,
        yDomain,
        format,
        color = 'accent',
        caption = '',
        kind = 'line',
        selectedIndex = null,
        onselect = undefined,
    }: {
        series: ChartPoint[];
        yDomain: [number, number];
        format: (v: number) => string;
        color?: TrendColor;
        caption?: string;
        /** `bar` (discrete counts, with a moving-average trend line) or `line`
         *  (rates / scores / durations, area + line). */
        kind?: ChartKind;
        /** Index into `series` of the highlighted datapoint (null = none). */
        selectedIndex?: number | null;
        /** Called with the slot index when a datapoint is clicked. */
        onselect?: ((index: number) => void) | undefined;
    } = $props();

    // Named-token fills (never raw hex): area by trend tone, dots likewise.
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
    const line = $derived(series.map((p, i) => (p.value == null ? null : { t: i, v: p.value })));
    const definedCount = $derived(series.filter((p) => p.value != null).length);

    const fmtY = (v: unknown): string => format(Number(v));

    // Logical plot box; CSS scales it responsively (aspect-preserving, no distortion).
    const W = 720;
    const H = 200;
    const MARGIN = { top: 12, right: 14, bottom: 14, left: 48 };
    const innerW = W - MARGIN.left - MARGIN.right;
    const innerH = H - MARGIN.top - MARGIN.bottom;

    const n = $derived(series.length);
    // The same linear scales PlotState builds: slot index [0, n-1] → [0, innerW],
    // fixed y-domain → [innerH, 0]. Computed here so the overlay marks align.
    const xPix = (i: number): number => (n > 1 ? (i / (n - 1)) * innerW : innerW / 2);
    const yPix = (v: number): number => {
        const [y0, y1] = yDomain;
        const t = y1 > y0 ? (v - y0) / (y1 - y0) : 0;
        return innerH - t * innerH;
    };

    type Dot = { i: number; cx: number; cy: number };
    const dots = $derived(
        series
            .map((p, i): Dot | null => (p.value == null ? null : { i, cx: xPix(i), cy: yPix(p.value) }))
            .filter((d): d is Dot => d != null),
    );
    const selDot = $derived(
        selectedIndex != null && series[selectedIndex]?.value != null
            ? { cx: xPix(selectedIndex), cy: yPix(series[selectedIndex]!.value as number) }
            : null,
    );
    // Half a slot's width, so a click anywhere nearest a point selects it.
    const slotW = $derived(n > 1 ? innerW / (n - 1) : innerW);

    // Bar mode (discrete counts): manual bars in the inner coord space (adaptive
    // width, capped) rising from the 0 baseline, plus a moving-average trend line
    // drawn with Plot.Line (same scales, so it lands on the bars).
    const barW = $derived(Math.min(slotW * 0.62, 30));
    type Bar = { i: number; x: number; y: number; w: number; h: number };
    const bars = $derived(
        kind === 'bar'
            ? series
                  .map((p, i): Bar | null =>
                      p.value == null
                          ? null
                          : { i, x: xPix(i) - barW / 2, y: yPix(p.value), w: barW, h: innerH - yPix(p.value) },
                  )
                  .filter((b): b is Bar => b != null)
            : [],
    );
    const trendLine = $derived(
        kind === 'bar'
            ? movingAverage(series).map((v, i) => (v == null ? null : { t: i, v }))
            : [],
    );
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
            {#if kind === 'bar'}
                <!-- Discrete counts: bars from the 0 baseline + a moving-avg trend. -->
                {#each bars as b (b.i)}
                    <rect
                        data-bar={b.i}
                        data-selected={b.i === selectedIndex}
                        x={b.x}
                        y={b.y}
                        width={b.w}
                        height={b.h}
                        rx="1"
                        fill={b.i === selectedIndex ? 'var(--accent)' : AREA_FILL[color]}
                    />
                {/each}
                <Plot.Line data={trendLine as { t: number; v: number }[]} x="t" y="v" stroke="var(--ink)" strokeWidth={1.5} />
            {:else}
                <Plot.Area data={line as { t: number; v: number }[]} x="t" y="v" fill={AREA_FILL[color]} opacity={0.6} />
                <Plot.Line data={line as { t: number; v: number }[]} x="t" y="v" stroke="var(--ink)" strokeWidth={1.5} />
            {/if}
            <Plot.Axis type="y" ticks={4} format={fmtY} showLine={false} showTicks={false} />

            <!-- Interactive overlay (same inner coord space as the geoms). -->
            {#if selDot}
                <line
                    data-guide
                    x1={selDot.cx}
                    y1={0}
                    x2={selDot.cx}
                    y2={innerH}
                    stroke="var(--accent)"
                    stroke-width="1"
                    stroke-dasharray="3 3"
                    opacity="0.6"
                />
            {/if}
            {#if kind !== 'bar'}
                {#each dots as d (d.i)}
                    <circle
                        data-point-dot={d.i}
                        data-selected={d.i === selectedIndex}
                        cx={d.cx}
                        cy={d.cy}
                        r={d.i === selectedIndex ? 4.5 : 2}
                        fill={d.i === selectedIndex ? 'var(--accent)' : DOT_FILL[color]}
                    />
                {/each}
            {/if}
            {#each series as _p, i (i)}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <rect
                    data-hit={i}
                    role="button"
                    tabindex={-1}
                    aria-label={`Select datapoint ${i + 1}`}
                    x={xPix(i) - slotW / 2}
                    y={0}
                    width={slotW}
                    height={innerH}
                    fill="transparent"
                    style="cursor: pointer; outline: none"
                    onclick={() => onselect?.(i)}
                />
            {/each}
        </ChartCanvas>
    {:else}
        <div class="text-sm text-ink-mute py-8 text-center">Not enough history to chart yet.</div>
    {/if}
</div>
