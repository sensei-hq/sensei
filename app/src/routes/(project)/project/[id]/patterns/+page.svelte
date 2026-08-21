<script lang="ts">
    import { PageHeader } from '$lib/components';
    import type { PatternEntry } from '$lib/types.js';

    let { data } = $props();

    // Expand/collapse map — a pattern row shows only name + confidence by
    // default. Clicking a row expands to show description, example, and
    // enforcement guidance where present.
    let expanded = $state<Record<string, boolean>>({});

    function toggle(id: string) {
        expanded = { ...expanded, [id]: !expanded[id] };
    }

    // "+12%" / "-4%" / "±0%" / "—" — same vocabulary as the observatory
    // Impact page so both surfaces read the same. Null means no baseline
    // could be computed (fewer than the analyzer's minimum session count).
    function fmtFtrDelta(v: number | null | undefined): string {
        if (v == null) return '—';
        const pct = Math.round(v * 100);
        if (pct === 0) return '±0%';
        return pct > 0 ? `+${pct}%` : `${pct}%`;
    }
    function deltaTone(v: number | null | undefined, anti: boolean): string {
        // On an anti-pattern the meaning of the sign flips — a NEGATIVE FTR
        // delta on an anti-pattern is EXPECTED and reads healthy (the locus
        // performs worse, which is why we're flagging it). So invert the
        // tone rule on anti-pattern rows.
        if (v == null || v === 0) return 'text-ink-mute';
        const good = anti ? v < 0 : v > 0;
        return good ? 'text-success' : 'text-warning';
    }
</script>

{#snippet PatternRow(p: PatternEntry, anti: boolean)}
    {@const open = expanded[p.id]}
    <li class="pattern-row py-2.5 border-b border-paper-edge text-sm" class:opacity-80={anti} data-testid={`pattern-row-${p.id}`}>
        <button
            type="button"
            class="flex items-center gap-3 w-full text-left bg-transparent border-none cursor-pointer text-inherit"
            aria-expanded={open}
            data-testid={`pattern-toggle-${p.id}`}
            onclick={() => toggle(p.id)}
        >
            <!-- kanji marker: 紋 for pattern, 禁 for anti-pattern -->
            <span
                class="kanji text-base w-5 text-center"
                class:text-accent={!anti}
                class:text-warning={anti}
            >{anti ? '禁' : '紋'}</span>
            <div class="flex-1 min-w-0">
                <p class="m-0 text-sm text-ink truncate">{p.name}</p>
                <p class="m-0 mt-0.5 text-xs text-ink-soft flex items-center gap-2">
                    {#if p.instanceCount != null && p.instanceCount > 0}
                        <span class="font-mono">{p.instanceCount}×</span>
                        <span class="text-ink-faint">·</span>
                    {/if}
                    {#if p.lifecycle}
                        <span class="font-mono uppercase tracking-wide">{p.lifecycle}</span>
                    {/if}
                    {#if p.confidence != null}
                        <span class="text-ink-faint">·</span>
                        <span class="font-mono">conf {Math.round(p.confidence * 100)}%</span>
                    {/if}
                </p>
            </div>
            <span
                class="text-xs font-mono min-w-[3.5rem] text-right {deltaTone(p.ftrDelta, anti)}"
                data-testid={`pattern-ftr-delta-${p.id}`}
                title="FTR delta vs project baseline"
            >{fmtFtrDelta(p.ftrDelta)}</span>
            <span class="text-xs text-ink-faint w-3">{open ? '▾' : '▸'}</span>
        </button>

        {#if open}
            <div class="mt-2 flex flex-col gap-2 pl-1" data-testid={`pattern-detail-${p.id}`}>
                {#if p.description}
                    <p class="text-sm text-ink-soft m-0 leading-normal">{p.description}</p>
                {:else}
                    <p class="text-xs text-ink-soft opacity-60 m-0 italic">
                        No description captured for this pattern yet.
                    </p>
                {/if}

                {#if p.enforcement}
                    <div>
                        <span class="text-xs uppercase tracking-wide opacity-60">Enforcement</span>
                        <p class="text-sm text-ink-soft m-0 mt-1">{p.enforcement}</p>
                    </div>
                {/if}

                {#if p.example}
                    <div>
                        <span class="text-xs uppercase tracking-wide opacity-60">Example</span>
                        <pre class="mt-1 px-3 py-2 text-xs font-mono bg-paper-mute rounded-md overflow-auto whitespace-pre-wrap break-all m-0">{p.example}</pre>
                    </div>
                {/if}
            </div>
        {/if}
    </li>
{/snippet}

<PageHeader title="Patterns" />
<div class="px-6 py-6">
    {#if data.followed.length > 0}
        <section class="mb-5">
            <h3 class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide">
                Followed ({data.followed.length})
            </h3>
            <ul class="list-none m-0 p-0">
                {#each data.followed as p (p.id)}
                    {@render PatternRow(p, false)}
                {/each}
            </ul>
        </section>
    {/if}
    {#if data.antiPatterns.length > 0}
        <section>
            <h3 class="text-xs font-semibold m-0 mb-2 uppercase tracking-wide text-accent">
                Anti-patterns ({data.antiPatterns.length})
            </h3>
            <ul class="list-none m-0 p-0">
                {#each data.antiPatterns as p (p.id)}
                    {@render PatternRow(p, true)}
                {/each}
            </ul>
        </section>
    {/if}
    {#if data.followed.length === 0 && data.antiPatterns.length === 0}
        <p class="text-sm text-ink-soft">No patterns detected yet.</p>
    {/if}
</div>

<style>
    .pattern-row:last-child {
        border-bottom: none;
    }
</style>
