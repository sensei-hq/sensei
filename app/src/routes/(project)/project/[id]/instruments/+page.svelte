<script lang="ts">
    import { PageHeader } from '$lib/components';
    import type { ProjectMcpToolStat } from '$lib/types.js';

    let { data } = $props();

    // Kind chip filter mirrors the observatory Playground so users move
    // between the two pages with the same mental model.
    let kindFilter = $state<'all' | 'query' | 'action'>('all');
    const kindChips: Array<{ id: 'all' | 'query' | 'action'; label: string }> = [
        { id: 'all', label: 'All' },
        { id: 'query', label: 'Queries' },
        { id: 'action', label: 'Actions' },
    ];

    const stats: ProjectMcpToolStat[] = $derived(data.mcpToolStats);
    const visibleStats = $derived(
        kindFilter === 'all' ? stats : stats.filter((t) => t.kind === kindFilter),
    );

    const totalCalls = $derived(stats.reduce((a, t) => a + (t.calls || 0), 0));
    const totalErrors = $derived(stats.reduce((a, t) => a + (t.errors || 0), 0));

    function fmtInt(n: number | null | undefined): string {
        return n == null ? '—' : n.toLocaleString();
    }
    function fmtPct(n: number | null | undefined): string {
        return n == null ? '—' : `${Math.round(n * 100)}%`;
    }
    function fmtMs(n: number | null | undefined): string {
        if (n == null) return '—';
        if (n < 1000) return `${Math.round(n)} ms`;
        return `${(n / 1000).toFixed(2)} s`;
    }
</script>

<PageHeader title="Instruments">
    {#snippet right()}
        <span class="text-sm text-ink-mute">
            {stats.length} tools · {fmtInt(totalCalls)} calls · {fmtInt(totalErrors)} errors
        </span>
    {/snippet}
</PageHeader>

<div class="px-6 py-6">
    <!-- MCP tool aggregation — the new lens per T2 Slice F. -->
    <section class="mb-8">
        <h3 class="text-sm font-medium m-0 mb-3">MCP tools</h3>

        <div class="flex gap-2 mb-3" role="tablist" aria-label="Tool kind filter">
            {#each kindChips as chip}
                {@const active = kindFilter === chip.id}
                <button
                    class="px-3 py-1 rounded-full border text-xs cursor-pointer transition-colors duration-fast"
                    class:bg-primary={active}
                    class:text-on-primary={active}
                    class:border-primary={active}
                    class:bg-transparent={!active}
                    class:text-ink-soft={!active}
                    class:border-paper-mute={!active}
                    role="tab"
                    aria-selected={active}
                    onclick={() => (kindFilter = chip.id)}
                >{chip.label}</button>
            {/each}
        </div>

        {#if visibleStats.length === 0}
            <p class="text-sm text-ink-soft">No tools match this filter.</p>
        {:else}
            <div class="grid grid-cols-[1fr_60px_60px_80px_80px_100px] gap-3 px-3 py-2 text-xs text-ink-soft tracking-wide uppercase">
                <span>Tool</span>
                <span class="text-right">Calls</span>
                <span class="text-right">Errors</span>
                <span class="text-right">Avg</span>
                <span class="text-right">FTR</span>
                <span class="text-right">Last used</span>
            </div>
            {#each visibleStats as t (t.id)}
                <div class="grid grid-cols-[1fr_60px_60px_80px_80px_100px] gap-3 px-3 py-2 border-b border-paper-mute text-sm items-center">
                    <div class="min-w-0">
                        <div class="font-mono text-xs truncate">{t.name}</div>
                        <div class="text-xs text-ink-soft truncate">{t.summary}</div>
                    </div>
                    <span class="text-right font-mono text-xs">{fmtInt(t.calls)}</span>
                    <span class="text-right font-mono text-xs" class:text-danger={t.errors > 0}>
                        {fmtInt(t.errors)}
                    </span>
                    <span class="text-right font-mono text-xs opacity-70">{fmtMs(t.avgDurationMs)}</span>
                    <span class="text-right font-mono text-xs"
                        class:text-success={t.ftr != null && t.ftr >= 0.8}
                        class:text-warning={t.ftr != null && t.ftr < 0.5}>
                        {fmtPct(t.ftr)}
                    </span>
                    <span class="text-right text-xs text-ink-soft">
                        {t.lastUsedAt ? new Date(t.lastUsedAt).toLocaleDateString() : '—'}
                    </span>
                </div>
            {/each}
        {/if}
    </section>

    <!-- Extensions (skills / commands / agents) — historical, still shown. -->
    {#if data.tools.length > 0}
        <section>
            <h3 class="text-sm font-medium m-0 mb-3">Extensions</h3>
            <ul class="list-none m-0 p-0">
                {#each data.tools as tool (tool.id)}
                    <li class="tool-row flex items-center gap-2.5 py-2 border-b border-paper-mute text-sm">
                        <span class="font-semibold flex-1">{tool.name}</span>
                        <span class="opacity-50 text-xs">{tool.kind}</span>
                        <span class="scope-badge text-xs px-1.5 py-px rounded-md font-mono"
                              class:global={tool.scope === "global"}
                              class:proj={tool.scope === "project"}>
                            [{tool.scope}]
                        </span>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}
</div>

<style>
    .tool-row:last-child {
        border-bottom: none;
    }
    .scope-badge.global {
        background: var(--paper-mute);
        opacity: 0.7;
    }
    .scope-badge.proj {
        background: var(--accent-soft);
        color: var(--accent);
    }
</style>
