<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import { Eyebrow, PageHeader } from "$lib/components";
    import type { LibEntry } from "$lib/types.js";

    type UsageEntry = { library_name: string; folder: string; version_used: string | null; import_count: number };

    let libs = $state<LibEntry[]>([]);
    let loading = $state(true);
    let search = $state("");
    let selectedLib = $state<LibEntry | null>(null);
    let usageData = $state<UsageEntry[]>([]);
    let usageLoading = $state(false);
    let usageError = $state<string | null>(null);

    onMount(async () => {
        const api = senseiApi(appState.port);
        // Show every detected library annotated with its usage count (repoCount),
        // not just those shared across ≥2 repos — `shared: true` hid single-repo
        // libs entirely, which is why most @rokkit/* packages never appeared (#63).
        const data = await api.getLibs({});
        libs = data.libs;
        loading = false;
    });

    // Fetch usage data when selection changes
    $effect(() => {
        const lib = selectedLib;
        if (!lib?.id) { usageData = []; usageError = null; return; }
        usageLoading = true;
        usageError = null;
        senseiApi(appState.port).getLibraryUsage(lib.id).then(d => {
            usageData = d.usage ?? [];
            usageLoading = false;
        }).catch((e: unknown) => {
            // Surface the fetch failure so the user can tell "no usage" from
            // "we couldn't load usage". Was previously silently emptying the
            // list, which looked identical to a genuinely-unused library.
            console.warn('[libraries] getLibraryUsage failed', { libId: lib.id }, e);
            usageData = [];
            usageError = e instanceof Error ? e.message : String(e);
            usageLoading = false;
        });
    });

    let filtered = $derived(
        libs.filter((l) => {
            if (search && !l.name.toLowerCase().includes(search.toLowerCase()))
                return false;
            return true;
        }),
    );
</script>

<PageHeader kanji="書" eyebrow="Libraries" title="Libraries" />
<div class="max-w-[960px] mx-auto px-12 pt-8 pb-16">

    <!-- Search -->
    <div class="flex items-center gap-4 mb-6">
        <input
            class="lib-search flex-1 px-3.5 py-2 border border-paper-mute rounded-md bg-paper-soft text-ink text-sm outline-none"
            type="text"
            placeholder="Search libraries..."
            bind:value={search}
        />
    </div>

    {#if loading}
        <p class="text-sm text-ink-soft">Loading libraries...</p>
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
                {#each filtered as lib (lib.id)}
                    <button
                        class="lib-card text-left px-4 py-3.5 border border-paper-mute rounded-md bg-paper-soft cursor-pointer transition-colors duration-fast"
                        class:selected={selectedLib?.id === lib.id}
                        onclick={() => (selectedLib = lib)}
                    >
                        <div class="flex items-baseline gap-2 mb-1.5">
                            <span class="text-sm font-medium text-ink">{lib.name}</span>
                            {#if lib.ecosystem}
                                <span class="text-xs text-ink-faint">{lib.ecosystem}</span>
                            {/if}
                            <span class="text-xs text-ink-soft"
                                >{lib.repoCount} repo{lib.repoCount !== 1
                                    ? "s"
                                    : ""}</span
                            >
                        </div>
                    </button>
                {/each}
            </div>

            {#if selectedLib}
                <div
                    class="p-6 bg-paper-mute border border-paper-mute rounded-lg sticky top-6"
                >
                    <h3 class="text-base font-medium m-0 mb-1">
                        {selectedLib.name}
                    </h3>
                    <p class="text-xs text-ink-soft m-0 mb-4">
                        {#if selectedLib.ecosystem}{selectedLib.ecosystem} · {/if}
                        {selectedLib.repoCount} repo{selectedLib.repoCount !== 1 ? 's' : ''}
                        {#if selectedLib.pageCount}· {selectedLib.pageCount} doc pages{/if}
                    </p>

                    <div class="mb-4">
                        <p class="m-0 mb-2"><Eyebrow>Used in</Eyebrow></p>
                        <div class="flex flex-wrap gap-1.5">
                            {#each selectedLib.repos as repo}
                                <span class="inline-block px-2 py-0.5 rounded-full text-xs bg-paper-mute text-ink-soft lowercase">{repo}</span>
                            {/each}
                        </div>
                    </div>

                    {#if usageLoading}
                        <p class="text-xs text-ink-soft">Loading usage...</p>
                    {:else if usageError}
                        <p class="text-xs text-error">Couldn't load usage: {usageError}</p>
                    {:else if usageData.length > 0}
                        <div>
                            <p class="m-0 mb-2"><Eyebrow>Usage by folder</Eyebrow></p>
                            {#each usageData as u}
                                <div class="flex justify-between py-1.5 border-b border-paper-mute text-sm">
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
