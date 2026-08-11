<script lang="ts">
    import type { TrendColor, SeriesDistribution } from '$lib/metrics/metric-view.js';

    // The drill-down trend chart: an area + line over the selected metric's
    // series, with high/mean/low gridlines, a dashed mean reference, and a
    // coloured endpoint. Scale-free (maps low..high onto the plot box). Pure
    // template; values + labels are supplied already-formatted by the page.
    let {
        values,
        periods = [],
        distribution,
        format,
        color = 'accent',
        caption = '',
    }: {
        values: number[];
        periods?: string[];
        distribution: SeriesDistribution | null;
        format: (v: number) => string;
        color?: TrendColor;
        caption?: string;
    } = $props();

    // Plot box inside the 720×200 viewBox — left gutter for y-labels.
    const X0 = 48;
    const X1 = 700;
    const YHIGH = 24;
    const YLOW = 168;

    // Named-token fills (literal strings so UnoCSS extracts them).
    const AREA: Record<TrendColor, string> = {
        success: 'fill-success-soft',
        accent: 'fill-accent-soft',
        'ink-faint': 'fill-paper-mute',
    };
    const DOT: Record<TrendColor, string> = {
        success: 'fill-success',
        accent: 'fill-accent',
        'ink-faint': 'fill-ink-faint',
    };

    function yOf(v: number, d: SeriesDistribution): number {
        const span = d.high - d.low || 1;
        return YLOW - ((v - d.low) / span) * (YLOW - YHIGH);
    }

    const geom = $derived.by(() => {
        if (!distribution || values.length < 2) return null;
        const d = distribution;
        const pts = values.map((v, i) => ({
            x: X0 + (i / (values.length - 1)) * (X1 - X0),
            y: yOf(v, d),
        }));
        const line = pts.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ');
        const area =
            `M${X0},${YLOW} ` +
            pts.map((p) => `L${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ') +
            ` L${X1},${YLOW} Z`;
        return { pts, line, area, last: pts.at(-1)!, yMean: yOf(d.mean, d) };
    });

    // Endpoint period labels (honest, no invented dates): first + last only.
    const firstLabel = $derived(periods.at(0) ?? '');
    const lastLabel = $derived(periods.at(-1) ?? 'today');
</script>

<div data-component="detail-chart" class="bg-paper border border-paper-edge rounded-md p-4 flex flex-col gap-2">
    {#if caption}<div class="text-xs text-ink-mute">{caption}</div>{/if}
    {#if geom && distribution}
        <svg viewBox="0 0 720 200" preserveAspectRatio="none" class="w-full h-[220px]" aria-hidden="true">
            <!-- gridlines at high / mean / low -->
            <line x1={X0} y1={YHIGH} x2={X1} y2={YHIGH} stroke="var(--paper-edge)" stroke-width="1" />
            <line x1={X0} y1={YLOW} x2={X1} y2={YLOW} stroke="var(--paper-edge)" stroke-width="1" />
            <line
                x1={X0}
                y1={geom.yMean}
                x2={X1}
                y2={geom.yMean}
                stroke="var(--ink-faint)"
                stroke-width="1"
                stroke-dasharray="2 4"
            />
            <!-- y-axis labels -->
            <text x="0" y={YHIGH + 4} class="fill-ink-faint mono text-[11px]">{format(distribution.high)}</text>
            <text x="0" y={geom.yMean + 4} class="fill-ink-faint mono text-[11px]">{format(distribution.mean)}</text>
            <text x="0" y={YLOW + 4} class="fill-ink-faint mono text-[11px]">{format(distribution.low)}</text>
            <!-- area + line + endpoint -->
            <path d={geom.area} class={AREA[color]} stroke="none" />
            <polyline
                points={geom.line}
                fill="none"
                stroke="var(--ink)"
                stroke-width="1.4"
                stroke-linejoin="round"
                stroke-linecap="round"
                vector-effect="non-scaling-stroke"
            />
            <circle cx={geom.last.x} cy={geom.last.y} r="4" class={DOT[color]} stroke="none" />
        </svg>
        <div class="flex justify-between pl-12 text-[11px] mono text-ink-faint">
            <span>{firstLabel}</span>
            <span>{lastLabel}</span>
        </div>
    {:else}
        <div class="text-sm text-ink-mute py-8 text-center">Not enough history to chart yet.</div>
    {/if}
</div>
