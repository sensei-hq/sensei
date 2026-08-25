<script lang="ts">
    import { Kanji, Eyebrow } from '$lib/components';
    import { TREND_TEXT, type SignalGroup, type SeriesDistribution } from '$lib/metrics/metric-view.js';
    import { metricKanji } from '$lib/metrics/metric-kanji.js';

    // The drill-down left rail: every signal, grouped and led by the key signals
    // (the same order the grid uses), the selected one highlighted, plus the
    // selected metric's High/Mean/Low over the period. A flat 27-item list was
    // unnavigable — there was no way to tell which of them mattered.
    let {
        groups,
        selectedKey,
        projectId,
        distribution,
        format,
    }: {
        groups: SignalGroup[];
        selectedKey: string;
        projectId: string;
        distribution: SeriesDistribution | null;
        format: (v: number) => string;
    } = $props();
</script>

<div data-component="signal-rail" class="border-r border-paper-edge p-6 flex flex-col gap-4">
    <div class="flex items-center gap-2">
        <Kanji char="観" size="sm" tone="accent" />
        <Eyebrow>Signals</Eyebrow>
    </div>

    <nav class="flex flex-col gap-4">
        {#each groups as g (g.id)}
        <div class="flex flex-col gap-1">
            <div class="px-3 pb-0.5"><Eyebrow>{g.label}</Eyebrow></div>
            {#each g.signals as s (s.key)}
            <a
                href={`/project/${projectId}/metrics/${s.key}`}
                data-signal={s.key}
                data-active={s.key === selectedKey}
                class="flex items-center justify-between gap-3 border-l-2 p-3 pl-2.5 rounded-md no-underline text-sm transition-colors duration-fast {s.key ===
                selectedKey
                    ? 'border-accent bg-accent-soft text-ink'
                    : 'border-transparent text-ink-soft hover:bg-paper-mute'}"
            >
                <!-- Glyph column is reserved whether or not this metric has one,
                     so an unassigned signal doesn't shunt its name out of line. -->
                <span class="flex items-baseline gap-3 min-w-0">
                    <span class="w-4 shrink-0 text-center" aria-hidden="true">
                        {#if metricKanji(s.key)}
                            <Kanji char={metricKanji(s.key)!} size="sm" tone="muted" />
                        {/if}
                    </span>
                    <span class="text-ink truncate">{s.name}</span>
                </span>
                {#if s.trend}
                    <span class="mono text-xs shrink-0 {TREND_TEXT[s.color]}">{s.trend.label}</span>
                {/if}
            </a>
            {/each}
        </div>
        {/each}
    </nav>

    {#if distribution}
        <div class="flex flex-col gap-2 border-t border-paper-edge pt-4">
            <Eyebrow>In this period</Eyebrow>
            {#each [{ label: 'High', v: distribution.high }, { label: 'Mean', v: distribution.mean }, { label: 'Low', v: distribution.low }] as r (r.label)}
                <div class="flex justify-between text-sm text-ink-soft">
                    <span>{r.label}</span>
                    <span class="mono text-ink">{format(r.v)}</span>
                </div>
            {/each}
        </div>
    {/if}
</div>

