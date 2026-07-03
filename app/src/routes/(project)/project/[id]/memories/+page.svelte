<script lang="ts">
    import { PageHeader } from '$lib/components';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { invalidateAll } from '$app/navigation';
    import { page } from '$app/state';
    import type { MemoryShareBatch, ProjectMemory } from '$lib/types.js';

    let { data } = $props();

    // Working set of memory ids the user has ticked for the next share batch.
    // Cleared after a successful create so the same UI state doesn't linger.
    let selected = $state<Set<string>>(new Set());
    let creating = $state(false);
    let deciding = $state<Record<string, boolean>>({});
    // Memory Anatomy drawer — id of the row whose What/Because/Consequence
    // is expanded on the right. Only one row can be open at a time.
    let openedMemoryId = $state<string | null>(null);

    const projectId = $derived(page.params.id ?? '');

    function toggle(memoryId: string) {
        const next = new Set(selected);
        if (next.has(memoryId)) next.delete(memoryId);
        else next.add(memoryId);
        selected = next;
    }

    async function proposeBatch() {
        if (selected.size === 0 || !projectId) return;
        creating = true;
        try {
            await senseiApi(appState.port).createMemoryShareBatch(
                projectId,
                Array.from(selected),
            );
            selected = new Set();
            await invalidateAll();
        } finally {
            creating = false;
        }
    }

    async function decide(batchId: string, status: 'approved' | 'rejected' | 'withdrawn') {
        if (!projectId) return;
        deciding = { ...deciding, [batchId]: true };
        try {
            await senseiApi(appState.port).decideMemoryShareBatch(projectId, batchId, status);
            await invalidateAll();
        } finally {
            deciding = { ...deciding, [batchId]: false };
        }
    }

    const memories: ProjectMemory[] = $derived(data.memories);
    const proposedBatches: MemoryShareBatch[] = $derived(data.proposedBatches);
    const openedMemory = $derived(
        openedMemoryId ? memories.find((m) => m.id === openedMemoryId) ?? null : null,
    );
</script>

<PageHeader title="Memories" />
<div class="px-6 py-6">
    {#if data.pendingShare > 0}
        <div class="text-sm px-3.5 py-2.5 rounded-md mb-4 bg-accent-soft text-accent">
            {data.pendingShare} memories pending collective share
        </div>
    {/if}

    <!-- Pending batches (proposed only). Approved / rejected verdicts fall
         off the top-level list to keep the review queue tight. -->
    {#if proposedBatches.length > 0}
        <section class="mb-6">
            <h3 class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wide">
                Proposed share batches ({proposedBatches.length})
            </h3>
            <ul class="list-none m-0 p-0 flex flex-col gap-2">
                {#each proposedBatches as batch (batch.id)}
                    <li class="border border-paper-mute rounded-md px-3 py-2 flex items-center gap-3">
                        <span class="text-sm flex-1">
                            {batch.memberCount} memor{batch.memberCount === 1 ? 'y' : 'ies'}
                            {#if batch.note} · <span class="opacity-70">{batch.note}</span>{/if}
                        </span>
                        <span class="text-xs text-ink-soft">{new Date(batch.createdAt).toLocaleDateString()}</span>
                        <button
                            type="button"
                            class="px-2 py-1 rounded-md text-xs bg-primary text-on-primary border-none cursor-pointer"
                            disabled={deciding[batch.id]}
                            data-testid={`batch-approve-${batch.id}`}
                            onclick={() => decide(batch.id, 'approved')}
                        >Approve</button>
                        <button
                            type="button"
                            class="px-2 py-1 rounded-md text-xs bg-transparent text-ink-soft border border-paper-edge cursor-pointer"
                            disabled={deciding[batch.id]}
                            data-testid={`batch-reject-${batch.id}`}
                            onclick={() => decide(batch.id, 'rejected')}
                        >Reject</button>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    <!-- Selection toolbar for building a new batch -->
    {#if memories.length > 0}
        <div class="flex items-center justify-between mb-3">
            <span class="text-xs uppercase tracking-wide opacity-60">
                {selected.size > 0 ? `${selected.size} selected` : 'Select memories to propose a share batch'}
            </span>
            <button
                type="button"
                class="px-3 py-1.5 rounded-md text-xs bg-primary text-on-primary border-none cursor-pointer"
                disabled={selected.size === 0 || creating}
                data-testid="propose-batch-button"
                onclick={proposeBatch}
            >{creating ? 'Proposing…' : 'Propose batch'}</button>
        </div>
    {/if}

    <div class="grid gap-6" class:grid-cols-1={!openedMemory} class:grid-cols-[1fr_360px]={openedMemory}>
        <ul class="list-none m-0 p-0" data-testid="memories-list">
            {#each memories as m (m.id)}
                {@const checked = selected.has(m.id)}
                {@const opened = openedMemoryId === m.id}
                <li class="memory-row flex items-center gap-3 py-2 border-b border-paper-mute text-sm"
                    class:bg-paper-mute={opened}
                    data-testid={`memory-row-${m.id}`}
                    data-opened={opened || undefined}
                >
                    <input
                        type="checkbox"
                        class="cursor-pointer"
                        aria-label={`Select memory ${m.title || m.name}`}
                        data-testid={`memory-checkbox-${m.id}`}
                        {checked}
                        onchange={() => toggle(m.id)}
                    />
                    <button
                        type="button"
                        class="flex-1 text-left bg-transparent border-none cursor-pointer text-inherit px-0"
                        data-testid={`memory-open-${m.id}`}
                        onclick={() => (openedMemoryId = opened ? null : m.id)}
                    >{m.title || m.name}</button>
                    <span class="opacity-50 text-xs">{m.type}</span>
                    <span class="text-xs font-mono">{Math.round((m.strength ?? 0) * 100)}%</span>
                </li>
            {/each}
        </ul>

        {#if openedMemory}
            <aside
                class="p-5 bg-paper-mute border border-paper-mute rounded-lg sticky top-6 h-fit flex flex-col gap-4"
                data-testid="memory-anatomy"
            >
                <div class="flex items-baseline justify-between gap-3">
                    <span class="text-xs uppercase tracking-wide text-ink-mute">Memory anatomy</span>
                    <button
                        type="button"
                        class="text-xs text-ink-soft bg-transparent border-none cursor-pointer"
                        data-testid="memory-anatomy-close"
                        onclick={() => (openedMemoryId = null)}
                    >close</button>
                </div>
                <div>
                    <p class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-1">What</p>
                    <p class="text-sm text-ink m-0 leading-normal">{openedMemory.title || openedMemory.name}</p>
                </div>
                {#if openedMemory.content}
                    <div>
                        <p class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-1">Because</p>
                        <p class="text-sm text-ink m-0 leading-normal whitespace-pre-line">{openedMemory.content}</p>
                    </div>
                {/if}
                {#if openedMemory.impact}
                    <div>
                        <p class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-1">Consequence</p>
                        <p class="text-sm text-ink m-0 leading-normal">{openedMemory.impact}</p>
                    </div>
                {/if}
                <div class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
                    <span class="text-ink-mute">Strength</span>
                    <span class="font-mono">{Math.round((openedMemory.strength ?? 0) * 100)}%</span>
                    <span class="text-ink-mute">Reinforced</span>
                    <span class="font-mono">{openedMemory.reinforcedCount ?? 0}</span>
                    <span class="text-ink-mute">Violated</span>
                    <span class="font-mono" class:text-warning={(openedMemory.violatedCount ?? 0) > 0}>{openedMemory.violatedCount ?? 0}</span>
                    <span class="text-ink-mute">Scope</span>
                    <span class="font-mono">{openedMemory.scope}{openedMemory.scopeFilter ? ` · ${openedMemory.scopeFilter}` : ''}</span>
                    <span class="text-ink-mute">Type</span>
                    <span class="font-mono">{openedMemory.type}</span>
                </div>
            </aside>
        {/if}
    </div>
</div>

<style>
    .memory-row:last-child {
        border-bottom: none;
    }
</style>
