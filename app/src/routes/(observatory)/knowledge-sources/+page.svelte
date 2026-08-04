<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import { PageHeader } from "$lib/components";
    import { Button } from "@rokkit/ui";
    import { KnowledgeSourcesState } from "$lib/knowledge-sources.svelte.js";

    const state = new KnowledgeSourcesState();

    onMount(() => {
        state.load(senseiApi(appState.port));
    });

    const api = () => senseiApi(appState.port);
</script>

<PageHeader kanji="連" eyebrow="Federation" title="Knowledge sources" />
<div class="max-w-[960px] mx-auto px-12 pt-8 pb-16">

    <!-- Add a source -->
    <div class="p-5 border border-paper-mute rounded-lg bg-paper-soft mb-8">
        <p class="text-sm font-medium text-ink m-0 mb-3">Add a source</p>
        <div class="flex flex-wrap items-center gap-3">
            <input
                class="ks-input flex-1 min-w-[220px] px-3.5 py-2 border border-paper-mute rounded-md bg-paper-mute text-ink text-sm outline-none"
                type="text"
                placeholder="https://hive.example.com"
                bind:value={state.url}
            />
            <input
                class="ks-input w-[180px] px-3.5 py-2 border border-paper-mute rounded-md bg-paper-mute text-ink text-sm outline-none"
                type="password"
                placeholder="Access key (optional)"
                bind:value={state.key}
            />
            <input
                class="ks-input w-[180px] px-3.5 py-2 border border-paper-mute rounded-md bg-paper-mute text-ink text-sm outline-none"
                type="text"
                placeholder="Name (optional)"
                bind:value={state.name}
            />
            <Button
                variant="primary"
                size="sm"
                data-testid="add-source"
                disabled={state.adding}
                onclick={() => state.add(api())}
            >Add</Button>
        </div>
        {#if state.addError}
            <p class="text-xs text-error m-0 mt-2.5">{state.addError}</p>
        {/if}
    </div>

    {#if state.loading}
        <p class="text-sm text-ink-soft">Loading sources...</p>
    {:else if state.sources.length === 0}
        <EmptyState
            kanji="連"
            title="No knowledge sources."
            description="Register a hive-mind endpoint to federate rules and memories across machines."
        />
    {:else}
        <div class="flex flex-col gap-2">
            {#each state.sources as source (source.id)}
                <div
                    class="px-4 py-3.5 border border-paper-mute rounded-md bg-paper-soft"
                    data-testid="source-card"
                >
                    <div class="flex items-center gap-3">
                        <div class="flex-1 min-w-0">
                            <p class="text-sm font-medium text-ink m-0">
                                {source.name || source.url}
                            </p>
                            <p class="font-mono text-xs text-ink-soft m-0 mt-0.5 truncate">
                                {source.url}
                            </p>
                        </div>
                        <button
                            class="ks-btn px-3 py-1 rounded-md border border-paper-mute bg-transparent text-xs text-ink-mute cursor-pointer"
                            data-testid="sync-{source.id}"
                            onclick={() => state.sync(api(), source.id)}
                        >Sync</button>
                        <button
                            class="ks-btn px-3 py-1 rounded-md border border-paper-mute bg-transparent text-xs text-error cursor-pointer"
                            data-testid="remove-{source.id}"
                            onclick={() => state.remove(api(), source.id)}
                        >Remove</button>
                    </div>
                    <div class="flex items-center gap-3 mt-2 text-xs text-ink-soft">
                        <span>{source.direction}</span>
                        <span class="font-mono">seq {source.last_seq ?? "—"}</span>
                        <span>{source.enabled ? "enabled" : "disabled"}</span>
                    </div>
                </div>
            {/each}
        </div>
    {/if}

    {#if state.error}
        <p class="text-xs text-error mt-4">{state.error}</p>
    {/if}
</div>

<style>
    .ks-input:focus {
        border-color: var(--ink-soft);
    }
    .ks-btn:hover {
        background: var(--paper-mute);
    }
</style>
