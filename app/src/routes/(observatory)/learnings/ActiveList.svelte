<script lang="ts">
    import { memoryState, isTerminalStatus } from '$lib/memoryState.svelte.js';
</script>

{#if memoryState.active.length === 0}
    <p class="empty">No memories yet. Use /save or wait for AI proposals.</p>
{:else}
    {#each memoryState.active as m (m.id)}
        {@const acting = memoryState.isActing(m.id)}
        {@const terminal = isTerminalStatus(m.status)}
        {@const err = memoryState.actionError(m.id)}
        <article
            class="memory-row"
            class:selected={memoryState.selected === m.id}
            data-testid="active-row"
            data-id={m.id}
        >
            <div class="meta">
                <span class="chip scope">{m.scope}{m.scope_filter ? ':' + m.scope_filter : ''}</span>
                <span class="chip status status-{m.status}">{m.status}</span>
                {#each m.tags as tag (tag)}<span class="chip tag">{tag}</span>{/each}
            </div>
            <button type="button" class="title-btn" onclick={() => memoryState.select(m.id)}>{m.title}</button>
            <div class="metrics">
                <span title="strength">★ {m.strength.toFixed(1)} / 5</span>
                <span title="applied">✓ {m.applied_count}</span>
                <span title="violated">✗ {m.violated_count}</span>
            </div>
            <div class="flex flex-wrap gap-2 mt-2">
                <button
                    type="button"
                    class="px-2.5 py-1 rounded-md text-xs bg-transparent text-accent border border-accent-soft cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                    data-testid="reinforce-btn"
                    disabled={acting}
                    onclick={() => memoryState.reinforce(m.id)}
                >Reinforce</button>
                <button
                    type="button"
                    class="px-2.5 py-1 rounded-md text-xs bg-transparent text-ink-soft border border-paper-edge cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                    data-testid="challenge-btn"
                    disabled={acting || terminal}
                    onclick={() => memoryState.challenge(m.id)}
                >Challenge</button>
                <span class="flex-1"></span>
                <button
                    type="button"
                    class="px-2.5 py-1 rounded-md text-xs bg-transparent text-ink-soft border border-paper-edge cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                    data-testid="archive-btn"
                    disabled={acting}
                    onclick={() => memoryState.archiveMemory(m.id)}
                >Archive</button>
                <button
                    type="button"
                    class="px-2.5 py-1 rounded-md text-xs bg-transparent text-danger border border-danger-soft cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                    data-testid="dismiss-btn"
                    disabled={acting || terminal}
                    onclick={() => memoryState.dismiss(m.id)}
                >Dismiss</button>
            </div>
            {#if err}
                <p class="text-xs text-danger m-0 mt-1" data-testid="action-error">{err}</p>
            {/if}
        </article>
    {/each}
{/if}

<style>
    .memory-row { padding: 0.75rem; border: 1px solid var(--paper-mute); border-radius: 4px; margin-bottom: 0.5rem; }
    .memory-row.selected { border-color: var(--accent); }
    .title-btn { background: none; border: none; padding: 0; margin: 0.25rem 0; font-size: inherit; font-weight: 600; cursor: pointer; text-align: left; display: block; width: 100%; }
    .meta { display: flex; gap: 0.25rem; flex-wrap: wrap; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--paper-mute); }
    .metrics { display: flex; gap: 1rem; margin-top: 0.5rem; font-size: 0.85rem; opacity: 0.8; }
    .empty { color: var(--text-muted); padding: 1rem; }
</style>
