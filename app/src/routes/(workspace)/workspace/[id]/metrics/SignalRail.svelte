<script lang="ts">
    import { List, ItemContent } from '@rokkit/ui';
    import { Kanji, Eyebrow } from '$lib/components';
    import {
        signalRailItems,
        type SignalGroup,
        type SignalRailItem,
        type SeriesDistribution,
    } from '$lib/metrics/metric-view.js';

    // The drill-down left rail: every signal, grouped and led by the key signals
    // (the same order the grid uses), the selected one highlighted, plus the
    // selected metric's High/Mean/Low over the period. A flat 27-item list was
    // unnavigable — there was no way to tell which of them mattered.
    // A rokkit List (user, 2026-11-02), so the current row's shade is the List's
    // in both themes; the snippet adds only the trend, which carries its tone.
    let {
        groups,
        selectedKey,
        workspaceId,
        distribution,
        format,
    }: {
        groups: SignalGroup[];
        selectedKey: string;
        workspaceId: string;
        distribution: SeriesDistribution | null;
        format: (v: number) => string;
    } = $props();

    const items = $derived(signalRailItems(groups, workspaceId));
    const value = $derived(`/workspace/${workspaceId}/metrics/${selectedKey}`);
    const fields = { icon: 'kanji', label: 'text' };
</script>

<div data-component="signal-rail" class="border-r border-paper-edge p-6 flex flex-col gap-4">
    <div class="flex items-center gap-2">
        <Kanji char="観" size="sm" tone="accent" />
        <Eyebrow>Signals</Eyebrow>
    </div>

    <List {items} {fields} {value} collapsible={false}>
        {#snippet itemContent(proxy)}
            {@const row = proxy.original as SignalRailItem}
            <ItemContent {proxy} />
            {#if row.trend}
                <span data-signal-trend class="mono text-xs shrink-0 ml-auto {row.trendClass}">{row.trend}</span>
            {/if}
        {/snippet}
    </List>

    {#if distribution}
        <div class="flex flex-col gap-2 border-t border-paper-edge pt-4">
            <Eyebrow>In this period</Eyebrow>
            {#each [{ label: 'High', v: distribution.high }, { label: 'Mean', v: distribution.mean }, { label: 'Low', v: distribution.low }] as r (r.label)}
                <div class="flex justify-between text-sm text-ink-mute">
                    <span>{r.label}</span>
                    <span class="mono text-ink">{format(r.v)}</span>
                </div>
            {/each}
        </div>
    {/if}
</div>

