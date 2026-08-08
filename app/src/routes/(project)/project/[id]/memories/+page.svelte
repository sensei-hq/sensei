<script lang="ts">
    import { PageHeader, EmptyState, Eyebrow } from '$lib/components';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { invalidateAll } from '$app/navigation';
    import { page } from '$app/state';
    import { SvelteSet } from 'svelte/reactivity';
    import type { MemoryShareBatch } from '$lib/types.js';
    import {
        ProjectMemoriesState,
        SCOPE_LADDER,
        type ScopeRung,
    } from './memories-state.svelte.js';
    import MemoryChip from './MemoryChip.svelte';
    import { Button } from '@rokkit/ui';

    let { data } = $props();

    // Screen state — ready-to-share derivations, chip semantics, and the
    // generalise / widen state machines all live here (pure, tested). The effect
    // keeps it synced with the load, including re-fetches after a mutation.
    const memState = new ProjectMemoriesState();
    $effect(() => memState.setMemories(data.memories));

    // Working set of memory ids the user has ticked for the next share batch.
    // Cleared after a successful create so the same UI state doesn't linger.
    let selected = new SvelteSet<string>();
    let creating = $state(false);
    let deciding = $state<Record<string, boolean>>({});
    // Memory Anatomy drawer — id of the row whose What/Because/Consequence
    // is expanded on the right. Only one row can be open at a time.
    let openedMemoryId = $state<string | null>(null);

    const projectId = $derived(page.params.id ?? '');

    function toggle(memoryId: string) {
        if (selected.has(memoryId)) selected.delete(memoryId);
        else selected.add(memoryId);
    }

    // Hero "review next batch" — stage every generalised (ready) memory into the
    // batch selection so the user can propose it through the existing flow.
    function reviewNextBatch() {
        selected.clear();
        for (const m of memState.ready) selected.add(m.id);
    }

    async function proposeBatch() {
        if (selected.size === 0 || !projectId) return;
        creating = true;
        try {
            await senseiApi(appState.port).createMemoryShareBatch(
                projectId,
                Array.from(selected),
            );
            selected.clear();
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

    function generaliseMessage(status: number): string {
        if (status === 503) return "couldn't generalise this right now — the model was unavailable. try again.";
        if (status === 422) return 'this memory has no content to generalise.';
        return "couldn't generalise this right now. try again.";
    }

    // Rewrite a memory stack-agnostic. Immediate-feedback rule: the model takes
    // 1–4s, so show a loading label. On 503 the daemon degrades honestly — surface
    // the error, keep the original. On success re-fetch so the chip flips to yes.
    async function generalise(id: string) {
        memState.startGeneralise(id);
        const res = await senseiApi(appState.port).generaliseMemory(id);
        if (res.ok) {
            await invalidateAll();
            memState.finishGeneralise(id);
        } else {
            memState.failGeneralise(id, generaliseMessage(res.error.status));
        }
    }

    function widenMessage(status: number): string {
        if (status === 409) return "this memory isn't promotable yet.";
        return "couldn't widen this right now. try again.";
    }

    // Widen up the scope ladder. project → user proceeds immediately; org /
    // collective are governed, so chooseScope stages the confirmation note first.
    function pickScope(id: string, rung: ScopeRung) {
        if (memState.chooseScope(rung) === 'proceed') widen(id, rung.scope);
    }

    function confirmWiden(id: string) {
        if (memState.pendingScope) widen(id, memState.pendingScope);
    }

    async function widen(id: string, scope: string) {
        memState.startPromote(id);
        const res = await senseiApi(appState.port).promoteMemory(id, { gov_scope: scope });
        if (res.ok) {
            memState.finishPromote(id);
            await invalidateAll();
        } else {
            memState.failPromote(id, widenMessage(res.error.status));
        }
    }

    const proposedBatches: MemoryShareBatch[] = $derived(data.proposedBatches);
    const openedMemory = $derived(
        openedMemoryId ? memState.memories.find((m) => m.id === openedMemoryId) ?? null : null,
    );
</script>

<PageHeader title="Memories" />
<div class="px-6 py-6">
    <!-- Ready-to-share hero — counts the generalised memories and offers to
         stage them for the next share batch. Success-toned, quiet voice. -->
    {#if memState.hasReady}
        {@const h = memState.heroDisplay}
        <section
            class="mb-6 rounded-lg border border-success-soft bg-success-soft p-5"
            data-testid="ready-to-share-hero"
        >
            <div class="flex items-start gap-4">
                <span class="kanji text-2xl text-success">送</span>
                <div class="flex-1 min-w-0">
                    <div class="mb-1"><Eyebrow tone="text-success">{h.eyebrow}</Eyebrow></div>
                    <h2 class="display text-lg font-normal m-0 text-ink">{h.headline}</h2>
                    <p class="text-sm text-ink-mute leading-normal m-0 mt-2 max-w-[640px]">{h.body}</p>
                    <div class="flex items-center gap-3 mt-4">
                        <Button
                            variant="primary"
                            size="sm"
                            data-testid="hero-review-batch"
                            onclick={reviewNextBatch}
                        >{h.action}</Button>
                        <span class="text-xs text-ink-soft">{h.meta}</span>
                    </div>
                </div>
            </div>
        </section>
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
                    <li class="border border-paper-edge rounded-md px-3 py-2 flex items-center gap-3">
                        <span class="text-sm flex-1">
                            {batch.memberCount} memor{batch.memberCount === 1 ? 'y' : 'ies'}
                            {#if batch.note} · <span class="opacity-70">{batch.note}</span>{/if}
                        </span>
                        <span class="text-xs text-ink-soft">{new Date(batch.createdAt).toLocaleDateString()}</span>
                        <Button
                            variant="primary"
                            size="sm"
                            disabled={deciding[batch.id]}
                            data-testid={`batch-approve-${batch.id}`}
                            onclick={() => decide(batch.id, 'approved')}
                        >Approve</Button>
                        <Button
                            variant="secondary"
                            style="outline"
                            size="sm"
                            disabled={deciding[batch.id]}
                            data-testid={`batch-reject-${batch.id}`}
                            onclick={() => decide(batch.id, 'rejected')}
                        >Reject</Button>
                    </li>
                {/each}
            </ul>
        </section>
    {/if}

    {#if memState.memories.length > 0}
        <!-- Selection toolbar for building a new batch -->
        <div class="flex items-center justify-between mb-3">
            <span class="text-xs uppercase tracking-wide opacity-60">
                {selected.size > 0 ? `${selected.size} selected` : 'Select memories to propose a share batch'}
            </span>
            <span class="text-xs text-ink-faint font-mono">{memState.countLabel}</span>
        </div>
        <div class="flex items-center justify-end mb-3">
            <Button
                variant="primary"
                size="sm"
                disabled={selected.size === 0 || creating}
                data-testid="propose-batch-button"
                onclick={proposeBatch}
            >{creating ? 'Proposing…' : 'Propose batch'}</Button>
        </div>

        <div class="grid gap-6" class:grid-cols-1={!openedMemory} class:grid-cols-[280px_1fr]={openedMemory}>
            <ul class="list-none m-0 p-0" data-testid="memories-list">
                {#each memState.memories as m (m.id)}
                    {@const checked = selected.has(m.id)}
                    {@const opened = openedMemoryId === m.id}
                    {@const chip = memState.chipFor(m)}
                    <li class="memory-row flex items-center gap-3 py-2 border-b border-paper-edge text-sm"
                        class:bg-paper-mute={opened}
                        class:border-l-2={m.generalised}
                        class:border-l-success={m.generalised}
                        data-testid={`memory-row-${m.id}`}
                        data-opened={opened || undefined}
                        data-generalised={m.generalised || undefined}
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
                        <MemoryChip tone={chip.tone} label={chip.label} />
                        <span class="opacity-50 text-xs">{m.type}</span>
                        <span class="text-xs font-mono">{Math.round((m.strength ?? 0) * 100)}%</span>
                    </li>
                {/each}
            </ul>

            {#if openedMemory}
                {@const drawerChip = memState.chipFor(openedMemory)}
                <main
                    class="max-w-[720px]"
                    data-testid="memory-anatomy"
                >
                    <!-- Eyebrow row: type · strength · surface · generalised -->
                    <div class="flex items-center gap-3 text-xs uppercase tracking-wider text-ink-soft mb-3">
                        <span>{openedMemory.type}</span>
                        <span class="w-[3px] h-[3px] rounded-full bg-ink-faint"></span>
                        <span>strength {Math.round((openedMemory.strength ?? 0) * 100)}%</span>
                        {#if openedMemory.scope}
                            <span class="w-[3px] h-[3px] rounded-full bg-ink-faint"></span>
                            <span class="font-mono normal-case tracking-normal text-ink-mute">{openedMemory.scope}{openedMemory.scopeFilter ? ` · ${openedMemory.scopeFilter}` : ''}</span>
                        {/if}
                        <span class="normal-case tracking-normal"><MemoryChip tone={drawerChip.tone} label={drawerChip.label} /></span>
                        <span class="flex-1"></span>
                        <button
                            type="button"
                            class="text-xs text-ink-soft normal-case tracking-normal bg-transparent border-none cursor-pointer"
                            data-testid="memory-anatomy-close"
                            onclick={() => (openedMemoryId = null)}
                        >close</button>
                    </div>

                    <!-- The memory statement — display-scale, mockup vibe -->
                    <h2 class="display text-[40px] font-light leading-tight tracking-tight m-0 mb-5 text-ink">
                        What: {openedMemory.title || openedMemory.name}
                    </h2>

                    <!-- Because — quiet paragraph -->
                    {#if openedMemory.content}
                        <p class="text-[15px] text-ink-mute leading-relaxed m-0 mb-4 whitespace-pre-line">
                            {openedMemory.content}
                        </p>
                    {/if}

                    <!-- Consequence — either violated summary or reinforced summary -->
                    {#if (openedMemory.violatedCount ?? 0) > 0}
                        <p class="text-[13px] text-ink-soft leading-relaxed m-0 mb-6">
                            When this slipped, sensei saw
                            <span class="text-warning">
                                {openedMemory.violatedCount} correction{openedMemory.violatedCount === 1 ? '' : 's'}
                            </span>
                            across recent sessions.
                        </p>
                    {:else if (openedMemory.reinforcedCount ?? 0) > 0}
                        <p class="text-[13px] text-ink-soft leading-relaxed m-0 mb-6">
                            Reinforced
                            <span class="text-ink-mute">{openedMemory.reinforcedCount} time{openedMemory.reinforcedCount === 1 ? '' : 's'}</span>
                            without a violation.
                        </p>
                    {:else if openedMemory.impact}
                        <p class="text-[13px] text-ink-soft leading-relaxed m-0 mb-6">
                            {openedMemory.impact}
                        </p>
                    {/if}

                    <!-- Observation grid — the anatomy details -->
                    <div class="grid grid-cols-2 gap-6 border-t border-paper-edge pt-5">
                        <div>
                            <p class="text-xs uppercase tracking-wider text-ink-soft m-0 mb-2">Reinforced</p>
                            <p class="font-mono text-lg text-ink m-0">{openedMemory.reinforcedCount ?? 0}</p>
                            <p class="text-xs text-ink-faint m-0 mt-1">times seen in evidence</p>
                        </div>
                        <div>
                            <p class="text-xs uppercase tracking-wider text-ink-soft m-0 mb-2">Violated</p>
                            <p
                                class="font-mono text-lg m-0"
                                class:text-warning={(openedMemory.violatedCount ?? 0) > 0}
                                class:text-ink={(openedMemory.violatedCount ?? 0) === 0}
                            >{openedMemory.violatedCount ?? 0}</p>
                            <p class="text-xs text-ink-faint m-0 mt-1">times a correction contradicted it</p>
                        </div>
                    </div>

                    <!-- Ready to share — generalise + widen up the scope ladder -->
                    <div class="border-t border-paper-edge pt-5 mt-6" data-testid="memory-share">
                        <p class="text-xs uppercase tracking-wider text-ink-soft m-0 mb-3">Ready to share</p>

                        {#if openedMemory.generalised}
                            <div class="grid grid-cols-1 gap-4">
                                <div>
                                    <p class="text-xs uppercase tracking-wider text-ink-faint m-0 mb-1">Original</p>
                                    <p class="text-sm text-ink-mute leading-relaxed m-0 whitespace-pre-line">{openedMemory.content}</p>
                                </div>
                                {#if openedMemory.generalisedContent}
                                    <div>
                                        <p class="text-xs uppercase tracking-wider text-success m-0 mb-1">Generalised</p>
                                        <p class="text-sm text-ink leading-relaxed m-0 whitespace-pre-line" data-testid="generalised-text">{openedMemory.generalisedContent}</p>
                                    </div>
                                {/if}
                            </div>

                            {#if memState.canWiden(openedMemory)}
                                <div class="mt-5" data-testid="widen">
                                    <button
                                        type="button"
                                        class="px-3 py-1.5 rounded-md text-xs bg-transparent text-accent border border-accent-soft cursor-pointer"
                                        data-testid="widen-toggle"
                                        onclick={() => memState.toggleWiden(openedMemory.id)}
                                    >widen scope</button>

                                    {#if memState.isWidenOpen(openedMemory.id)}
                                        <div class="mt-3 flex flex-wrap items-center gap-2" data-testid="widen-menu">
                                            {#each SCOPE_LADDER as rung (rung.scope)}
                                                <button
                                                    type="button"
                                                    class="px-2.5 py-1 rounded-md text-xs cursor-pointer border"
                                                    class:bg-primary={!rung.governed}
                                                    class:text-on-primary={!rung.governed}
                                                    class:border-transparent={!rung.governed}
                                                    class:bg-transparent={rung.governed}
                                                    class:text-ink-mute={rung.governed}
                                                    class:border-paper-edge={rung.governed}
                                                    disabled={memState.isPromoting(openedMemory.id)}
                                                    data-testid={`widen-${rung.scope}`}
                                                    onclick={() => pickScope(openedMemory.id, rung)}
                                                >{rung.label}</button>
                                            {/each}
                                        </div>

                                        {#if memState.pendingScope}
                                            <div class="mt-3 rounded-md border border-warning-soft bg-warning-soft p-3" data-testid="governance-note">
                                                <p class="text-xs text-ink-mute leading-normal m-0 mb-2">
                                                    {memState.governanceNote(memState.pendingScope)}
                                                </p>
                                                <div class="flex items-center gap-2">
                                                    <Button
                                                        variant="primary"
                                                        size="sm"
                                                        disabled={memState.isPromoting(openedMemory.id)}
                                                        data-testid="widen-confirm"
                                                        onclick={() => confirmWiden(openedMemory.id)}
                                                    >{memState.isPromoting(openedMemory.id) ? 'widening…' : `confirm widen to ${memState.pendingScope}`}</Button>
                                                    <Button
                                                        variant="secondary"
                                                        style="outline"
                                                        size="sm"
                                                        data-testid="widen-cancel"
                                                        onclick={() => memState.closeWiden()}
                                                    >cancel</Button>
                                                </div>
                                            </div>
                                        {/if}

                                        {#if memState.promoteError(openedMemory.id)}
                                            <p class="text-xs text-danger m-0 mt-2" data-testid="widen-error">{memState.promoteError(openedMemory.id)}</p>
                                        {/if}
                                    {/if}
                                </div>
                            {/if}
                        {:else}
                            <p class="text-sm text-ink-soft leading-normal m-0 mb-3">
                                sensei can rewrite this stack-agnostic so it can widen up the scope ladder.
                            </p>
                            <Button
                                variant="primary"
                                size="sm"
                                disabled={memState.isGeneralising(openedMemory.id)}
                                data-testid="generalise-button"
                                onclick={() => generalise(openedMemory.id)}
                            >{memState.isGeneralising(openedMemory.id) ? 'generalising…' : 'generalise'}</Button>
                            {#if memState.generaliseError(openedMemory.id)}
                                <p class="text-xs text-danger m-0 mt-2" data-testid="generalise-error">
                                    {memState.generaliseError(openedMemory.id)}
                                </p>
                            {/if}
                        {/if}
                    </div>
                </main>
            {/if}
        </div>
    {:else}
        <EmptyState
            kanji="覚"
            title="Nothing to remember yet."
            description="As you work in this project, sensei watches for corrections and patterns. Memories appear here once there's enough evidence to teach — and the ones that generalise cleanly become ready to share."
        />
    {/if}
</div>

<style>
    .memory-row:last-child {
        border-bottom: none;
    }
</style>
