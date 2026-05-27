<script lang="ts">
    import { memoryState } from '$lib/memoryState.svelte.js';
</script>

{#if memoryState.triage.length === 0}
    <p class="empty">No proposals yet. AI memories appear here for your review.</p>
{:else}
    {#each memoryState.triage as m (m.id)}
        <article
            class="memory-row"
            class:selected={memoryState.selected === m.id}
            data-testid="triage-row"
            data-id={m.id}
        >
            <div class="meta">
                <span class="chip scope">{m.scope}{m.scope_filter ? ':' + m.scope_filter : ''}</span>
                {#if m.triage_signal}<span class="chip signal">{m.triage_signal}</span>{/if}
                {#each m.tags as tag}<span class="chip tag">{tag}</span>{/each}
            </div>
            <button type="button" class="title-btn" onclick={() => memoryState.select(m.id)}>{m.title}</button>
            <div class="actions">
                <button type="button" data-testid="accept-btn" onclick={() => memoryState.accept(m.id)}>Accept</button>
                <button type="button" data-testid="reject-btn" onclick={() => memoryState.reject(m.id)}>Reject</button>
            </div>
        </article>
    {/each}
{/if}

<style>
    .memory-row { padding: 0.75rem; border: 1px solid var(--surface-z3); border-radius: 4px; margin-bottom: 0.5rem; }
    .memory-row.selected { border-color: var(--accent); }
    .title-btn { background: none; border: none; padding: 0; margin: 0.25rem 0; font-size: inherit; font-weight: 600; cursor: pointer; text-align: left; display: block; width: 100%; }
    .meta { display: flex; gap: 0.25rem; flex-wrap: wrap; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--surface-z3); }
    .chip.signal { background: var(--warning-bg, #553); color: var(--warning-fg, #fff); }
    .actions { display: flex; gap: 0.5rem; margin-top: 0.5rem; }
    .empty { color: var(--text-muted); padding: 1rem; }
</style>
