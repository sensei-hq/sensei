<script lang="ts">
    import { memoryState } from '$lib/memoryState.svelte.js';
</script>

{#if memoryState.archive.length === 0}
    <p class="empty">Nothing archived.</p>
{:else}
    {#each memoryState.archive as m (m.id)}
        <article class="memory-row" data-testid="archive-row" data-id={m.id}>
            <div class="meta">
                <span class="chip scope">{m.scope}</span>
                <span class="chip status status-{m.status}">{m.status}</span>
            </div>
            <button type="button" class="title-btn" onclick={() => memoryState.select(m.id)}>{m.title}</button>
        </article>
    {/each}
{/if}

<style>
    .memory-row { padding: 0.75rem; border: 1px solid var(--surface-z3); border-radius: 4px; margin-bottom: 0.5rem; opacity: 0.7; }
    .title-btn { background: none; border: none; padding: 0; margin: 0.25rem 0; font-size: inherit; font-weight: 600; cursor: pointer; text-align: left; display: block; width: 100%; }
    .meta { display: flex; gap: 0.25rem; flex-wrap: wrap; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--surface-z3); }
    .empty { color: var(--text-muted); padding: 1rem; }
</style>
