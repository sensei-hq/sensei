<script lang="ts">
    import { Eyebrow, MetricSparkline } from '$lib/components';
    import {
        pairedSeries,
        confidence,
        correlationSummary,
        type MetricCorrelation,
    } from '$lib/metrics/correlation-view.js';

    // One correlated pair, drawn as two lines on a SHARED normalised axis.
    //
    // Normalised, not dual-axis, and that is deliberate. Two independent Y axes
    // can be slid and stretched until any two series appear to track each other —
    // precisely the illusion a screen about real relationships must not create.
    // Min-max to [0,1] keeps the SHAPE (what a rank correlation measures) and
    // drops magnitude (which was never comparable between, say, a ratio in [0,1]
    // and a token count in the millions). The axis is labelled so nobody reads
    // the height as a value.
    let {
        correlation,
        seriesA = [],
        seriesB = [],
        nameOf,
    }: {
        correlation: MetricCorrelation;
        seriesA?: (number | null)[];
        seriesB?: (number | null)[];
        nameOf: (key: string) => string;
    } = $props();

    const paired = $derived(
        pairedSeries(correlation.a, seriesA, correlation.b, seriesB),
    );
    const strength = $derived(confidence(correlation.n));
    const summary = $derived(correlationSummary(correlation, nameOf));
    // Inverse pairs get the accent tone on the second line so the mirror-image
    // shape reads at a glance; positive pairs share the neutral tone.
    const inverse = $derived(correlation.rho < 0);
</script>

<section
    data-component="correlation-card"
    data-pair="{correlation.a}|{correlation.b}"
    class="rounded-md border border-paper-edge bg-paper p-4 flex flex-col gap-3"
>
    <header class="flex items-baseline justify-between gap-3 flex-wrap">
        <div class="flex flex-col gap-1 min-w-0">
            <Eyebrow>{inverse ? 'Moves inversely' : 'Moves together'}</Eyebrow>
            <div class="text-sm text-ink">
                {nameOf(correlation.a)} <span class="text-ink-faint">·</span>
                {nameOf(correlation.b)}
            </div>
        </div>
        <div class="flex items-baseline gap-2 shrink-0">
            <span
                data-component="correlation-rho"
                class="mono text-lg text-ink tabular-nums"
                style='font-feature-settings: "tnum";'
            >{correlation.rho.toFixed(2)}</span>
            <!-- Sample size sits beside the coefficient, never hidden: -0.95 over
                 156 days and 0.42 over 26 are different claims. -->
            <span
                data-component="correlation-n"
                data-confidence={strength}
                class="mono text-xs {strength === 'thin' ? 'text-warning' : 'text-ink-faint'}"
            >n={correlation.n}</span>
        </div>
    </header>

    {#if paired}
        <div class="flex flex-col gap-1">
            <div class="flex items-center gap-3">
                <MetricSparkline series={paired.a.points} tone="neutral" width={240} height={40} />
                <span class="text-xs text-ink-mute truncate">{nameOf(correlation.a)}</span>
            </div>
            <div class="flex items-center gap-3">
                <MetricSparkline
                    series={paired.b.points}
                    tone={inverse ? 'bad' : 'neutral'}
                    width={240}
                    height={40}
                />
                <span class="text-xs text-ink-mute truncate">{nameOf(correlation.b)}</span>
            </div>
            <p class="text-xs text-ink-faint m-0 mt-1">
                Both normalised to their own range over {paired.slots} shared days — the shapes are
                comparable, the heights are not.
            </p>
        </div>
    {:else}
        <!-- Honest-empty: the coefficient came from the daemon over the full
             history, but this screen only holds the recent window. Saying so beats
             drawing a line from one point. -->
        <p data-component="correlation-no-chart" class="text-xs text-ink-mute m-0">
            Not enough overlapping days in this window to plot the pair.
        </p>
    {/if}

    <p class="text-xs text-ink-soft m-0 leading-relaxed">{summary}</p>
</section>
