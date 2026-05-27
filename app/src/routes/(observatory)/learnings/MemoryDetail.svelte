<script lang="ts">
    import { memoryState } from '$lib/memoryState.svelte.js';
    const d = $derived(memoryState.detail);
</script>

{#if !d}
    <p class="empty">Select a memory to view details.</p>
{:else}
    <header class="head">
        <span class="chip scope">{d.memory.scope}{d.memory.scope_filter ? ':' + d.memory.scope_filter : ''}</span>
        <span class="chip status status-{d.memory.status}">{d.memory.status}</span>
        {#each d.memory.tags as tag}<span class="chip tag">{tag}</span>{/each}
    </header>

    <h2>{d.memory.title}</h2>
    <p class="content" data-testid="detail-content">{d.memory.content}</p>

    {#if d.memory.impact}
        <section><h4>Impact</h4><p>{d.memory.impact}</p></section>
    {/if}

    <section><h4>Metrics</h4>
        <p>Strength: <strong>{d.memory.strength.toFixed(1)} / 5</strong>
           — applied {d.memory.applied_count}× · violated {d.memory.violated_count}×</p>
    </section>

    {#if d.evidence.length}
        <section><h4>Evidence</h4>
            <ul>{#each d.evidence as e}
                <li>
                    {#if e.session_id}<code>{e.session_id.slice(0, 8)}</code>{/if}
                    {e.note ?? ''}
                </li>
            {/each}</ul>
        </section>
    {/if}

    {#if d.examples.length}
        <section><h4>Examples</h4>
            <ul>{#each d.examples as ex}
                <li>
                    <span class="chip ex-{ex.is_good ? 'good' : 'bad'}">{ex.is_good ? 'good' : 'bad'}</span>
                    {#if ex.node_id}<code>{ex.node_id}</code>{/if}
                    {ex.note ?? ''}
                </li>
            {/each}</ul>
        </section>
    {/if}

    {#if d.outcomes.length}
        <section data-testid="detail-outcomes"><h4>Recent outcomes</h4>
            <ul>{#each d.outcomes as o}
                <li>
                    <span class="chip outcome-{o.outcome}">{o.outcome}</span>
                    <time>{new Date(o.recorded_at).toLocaleString()}</time>
                    {#if o.context}— {o.context}{/if}
                </li>
            {/each}</ul>
        </section>
    {/if}
{/if}

<style>
    .empty { color: var(--text-muted); }
    .head { display: flex; gap: 0.25rem; flex-wrap: wrap; margin-bottom: 0.5rem; }
    .chip { padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; background: var(--surface-z3); }
    h2 { margin: 0.5rem 0; }
    .content { white-space: pre-wrap; }
    section { margin-top: 1rem; }
    section h4 { margin: 0 0 0.25rem 0; font-size: 0.85rem; opacity: 0.7; text-transform: uppercase; letter-spacing: 0.05em; }
    .chip.outcome-applied,  .chip.ex-good { background: var(--success-bg, #353); }
    .chip.outcome-violated, .chip.ex-bad  { background: var(--danger-bg, #533); }
    ul { padding-left: 1.25rem; margin: 0; }
    li { margin: 0.25rem 0; }
    code { font-size: 0.85em; background: var(--surface-z2); padding: 0 0.25rem; border-radius: 3px; }
</style>
