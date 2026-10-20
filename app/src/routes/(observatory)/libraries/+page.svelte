<script lang="ts">
    import { railIcon } from "../observatory-nav";
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import { Eyebrow, PageHeader, ScreenState } from "$lib/components";
    import { LibrariesState } from "./libraries-state.svelte.js";

    const api = senseiApi(appState.port);
    // Both reads and their failures live in the state (#288).
    const lib = new LibrariesState();
    onMount(() => void lib.load(api));

    let search = $state("");
    let filtered = $derived(
        lib.libs.filter((l) => !search || l.name.toLowerCase().includes(search.toLowerCase())),
    );
</script>

<PageHeader kanji="書" icon={railIcon("/libraries")} eyebrow="Libraries" title="Libraries" />
<div class="max-w-[960px] mx-auto px-12 pt-8 pb-16">

    <!-- Search -->
    <div class="flex items-center gap-4 mb-6">
        <input
            class="lib-search flex-1 px-3.5 py-2 border border-paper-edge rounded-md bg-paper-soft text-ink text-sm outline-none"
            type="text"
            placeholder="Search libraries..."
            bind:value={search}
        />
    </div>

    {#if lib.loading}
        <p class="text-sm text-ink-soft">Loading libraries...</p>
    {:else if lib.error}
        <ScreenState status="error" error={lib.error} onretry={() => void lib.load(api)} />
    {:else if filtered.length === 0}
        <EmptyState
            kanji="書"
            title="No libraries indexed."
            description="Libraries appear once sensei scans your project dependencies. Add folders in the setup wizard, and sensei will detect libraries from your manifests."
        />
    {:else}
        <div class="grid grid-cols-[1fr_340px] gap-6">
            <div class="flex flex-col gap-1">
                <!-- Key by id, not name: the same package name exists across
                     ecosystems (e.g. uuid npm + uuid cargo), so a name-keyed each
                     throws Svelte's each_key_duplicate and breaks the whole list.
                     Show the ecosystem so same-named libs are distinguishable. -->
                {#each filtered as l (l.id)}
                    <button
                        class="lib-card text-left px-4 py-3.5 border border-paper-edge rounded-md bg-paper-soft cursor-pointer transition-colors duration-fast"
                        class:selected={lib.selected?.id === l.id}
                        onclick={() => void lib.select(api, l)}
                    >
                        <div class="flex items-baseline gap-2 mb-1.5">
                            <span class="text-sm font-medium text-ink">{l.name}</span>
                            {#if l.ecosystem}
                                <span class="text-xs text-ink-faint">{l.ecosystem}</span>
                            {/if}
                            <span class="text-xs text-ink-soft"
                                >{l.repoCount} repo{l.repoCount !== 1
                                    ? "s"
                                    : ""}</span
                            >
                        </div>
                    </button>
                {/each}
            </div>

            {#if lib.selected}
                <div
                    class="p-6 bg-paper-mute border border-paper-edge rounded-lg sticky top-6"
                >
                    <h3 class="text-base font-medium m-0 mb-1">
                        {lib.selected.name}
                    </h3>
                    <p class="text-xs text-ink-soft m-0 mb-4">
                        {#if lib.selected.ecosystem}{lib.selected.ecosystem} · {/if}
                        {lib.selected.repoCount} repo{lib.selected.repoCount !== 1 ? 's' : ''}
                        {#if lib.selected.pageCount}· {lib.selected.pageCount} doc pages{/if}
                    </p>

                    <div class="mb-4">
                        <p class="m-0 mb-2"><Eyebrow>Used in</Eyebrow></p>
                        <div class="flex flex-wrap gap-1.5">
                            {#each lib.selected.repos as repo}
                                <span class="inline-block px-2 py-0.5 rounded-full text-xs bg-paper-mute text-ink-soft lowercase">{repo}</span>
                            {/each}
                        </div>
                    </div>

                    {#if lib.usageLoading}
                        <p class="text-xs text-ink-soft">Loading usage...</p>
                    {:else if lib.usageError}
                        <p class="text-xs text-error">Couldn't load usage: {lib.usageError}</p>
                    {:else if lib.usage.length > 0}
                        <div>
                            <p class="m-0 mb-2"><Eyebrow>Usage by folder</Eyebrow></p>
                            {#each lib.usage as u}
                                <div class="flex justify-between py-1.5 border-b border-paper-edge text-sm">
                                    <div>
                                        <span class="font-mono text-xs">{u.folder}</span>
                                        {#if u.version_used}
                                            <span class="text-xs text-ink-soft ml-1.5">v{u.version_used}</span>
                                        {/if}
                                    </div>
                                    <span class="font-mono text-xs text-ink-soft">{u.import_count} imports</span>
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .lib-search:focus {
        border-color: var(--ink-soft);
    }

    .lib-card:hover {
        background: var(--paper-mute);
    }
    .lib-card.selected {
        border-color: var(--ink-soft);
        background: var(--paper-mute);
    }
</style>
