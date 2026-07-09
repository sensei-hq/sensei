<script lang="ts">
    import { memoryState, memoryUsageStrip } from '$lib/memoryState.svelte.js';
    const d = $derived(memoryState.detail);
</script>

{#if !d}
    <p class="text-sm text-ink-mute">Select a memory to view details.</p>
{:else}
    {@const usage = memoryUsageStrip(d)}
    <header class="flex flex-wrap gap-1 mb-2">
        <span class="inline-block px-2 py-0.5 rounded-full text-xs bg-paper-mute text-ink">
            {d.memory.scope}{d.memory.scope_filter ? ':' + d.memory.scope_filter : ''}
        </span>
        <span class="inline-block px-2 py-0.5 rounded-full text-xs bg-paper-mute text-ink">
            {d.memory.status}
        </span>
        {#each d.memory.tags as tag (tag)}
            <span class="inline-block px-2 py-0.5 rounded-full text-xs bg-paper-mute text-ink">{tag}</span>
        {/each}
    </header>

    <h2 class="text-xl font-light text-ink my-2">{d.memory.title}</h2>
    <p class="whitespace-pre-wrap text-sm text-ink m-0" data-testid="detail-content">{d.memory.content}</p>

    {#if d.memory.impact}
        <section class="mt-4">
            <h4 class="text-xs uppercase tracking-wide text-ink-mute m-0 mb-1">Impact</h4>
            <p class="text-sm text-ink m-0">{d.memory.impact}</p>
        </section>
    {/if}

    <section class="mt-4">
        <h4 class="text-xs uppercase tracking-wide text-ink-mute m-0 mb-1">Usage</h4>
        <p data-testid="usage-strip" class="text-sm text-ink m-0">
            <span data-testid="usage-loaded">{usage.loaded}</span>
            <span class="text-ink-faint"> · </span>
            <span data-testid="usage-followed">{usage.followed}</span>
            <span class="text-ink-faint"> · </span>
            <span data-testid="usage-skipped">{usage.skipped}</span>
            <span class="text-ink-mute"> {usage.window}</span>
        </p>
        <p data-testid="usage-lifetime" class="text-xs text-ink-mute m-0 mt-1">
            strength {d.memory.strength.toFixed(1)} / 5
            · applied {d.memory.applied_count}× · violated {d.memory.violated_count}× all-time
        </p>
    </section>

    {#if d.evidence.length}
        <section class="mt-4">
            <h4 class="text-xs uppercase tracking-wide text-ink-mute m-0 mb-1">Evidence</h4>
            <ul class="pl-5 m-0 text-sm text-ink">
                {#each d.evidence as e, i (i)}
                    <li class="my-1">
                        {#if e.session_id}<code class="font-mono text-xs bg-paper-mute px-1 rounded">{e.session_id.slice(0, 8)}</code>{/if}
                        {e.note ?? ''}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if d.examples.length}
        <section class="mt-4">
            <h4 class="text-xs uppercase tracking-wide text-ink-mute m-0 mb-1">Examples</h4>
            <ul class="pl-5 m-0 text-sm text-ink">
                {#each d.examples as ex, i (i)}
                    <li class="my-1">
                        <span
                            class="inline-block px-2 py-0.5 rounded-full text-xs {ex.is_good
                                ? 'bg-success-soft text-success'
                                : 'bg-danger-soft text-danger'}"
                        >{ex.is_good ? 'good' : 'bad'}</span>
                        {#if ex.node_id}<code class="font-mono text-xs bg-paper-mute px-1 rounded">{ex.node_id}</code>{/if}
                        {ex.note ?? ''}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if d.outcomes.length}
        <section class="mt-4" data-testid="detail-outcomes">
            <h4 class="text-xs uppercase tracking-wide text-ink-mute m-0 mb-1">Recent outcomes</h4>
            <ul class="pl-5 m-0 text-sm text-ink">
                {#each d.outcomes as o, i (i)}
                    <li class="my-1">
                        <span
                            class="inline-block px-2 py-0.5 rounded-full text-xs {o.outcome === 'applied'
                                ? 'bg-success-soft text-success'
                                : o.outcome === 'violated'
                                    ? 'bg-danger-soft text-danger'
                                    : 'bg-paper-mute text-ink'}"
                        >{o.outcome}</span>
                        <time class="text-xs text-ink-mute">{new Date(o.recorded_at).toLocaleString()}</time>
                        {#if o.context}— {o.context}{/if}
                    </li>
                {/each}
            </ul>
        </section>
    {/if}
{/if}
