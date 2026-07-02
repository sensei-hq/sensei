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
                            onclick={() => decide(batch.id, 'approved')}
                        >Approve</button>
                        <button
                            type="button"
                            class="px-2 py-1 rounded-md text-xs bg-transparent text-ink-soft border border-paper-edge cursor-pointer"
                            disabled={deciding[batch.id]}
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
                onclick={proposeBatch}
            >{creating ? 'Proposing…' : 'Propose batch'}</button>
        </div>
    {/if}

    <ul class="list-none m-0 p-0">
        {#each memories as m (m.id)}
            {@const checked = selected.has(m.id)}
            <li class="memory-row flex items-center gap-3 py-2 border-b border-paper-mute text-sm">
                <input
                    type="checkbox"
                    class="cursor-pointer"
                    aria-label={`Select memory ${m.title || m.name}`}
                    {checked}
                    onchange={() => toggle(m.id)}
                />
                <span class="flex-1">{m.title || m.name}</span>
                <span class="opacity-50 text-xs">{m.type}</span>
                <span class="text-xs font-mono">{Math.round((m.strength ?? 0) * 100)}%</span>
            </li>
        {/each}
    </ul>
</div>

<style>
    .memory-row:last-child {
        border-bottom: none;
    }
</style>
