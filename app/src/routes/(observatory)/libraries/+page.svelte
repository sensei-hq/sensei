<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import type { LibEntry } from "$lib/types.js";

    let libs = $state<LibEntry[]>([]);
    let loading = $state(true);
    let search = $state("");
    let kindFilter = $state<"all" | "code" | "service">("all");
    let selectedLib = $state<LibEntry | null>(null);

    onMount(async () => {
        await appState.load();
        const api = senseiApi(appState.port);
        const data = await api.getLibs({ shared: true });
        libs = data.libs;
        loading = false;
    });

    let filtered = $derived(
        libs.filter((l) => {
            if (search && !l.name.toLowerCase().includes(search.toLowerCase()))
                return false;
            return true;
        }),
    );
</script>

<div class="max-w-[960px] mx-auto px-12 py-12 pb-16">
    <div class="mb-6">
        <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-2">
            Libraries
        </p>
        <h1 class="display text-2xl font-normal m-0">書 Libraries</h1>
    </div>

    <!-- Search + filters -->
    <div class="flex items-center gap-4 mb-6">
        <input
            class="lib-search flex-1 px-3.5 py-2 border border-surface-z3 rounded-md bg-surface-z1 text-surface-z9 text-ui outline-none"
            type="text"
            placeholder="Search libraries..."
            bind:value={search}
        />
        <div class="flex gap-1.5">
            {#each [["all", "All"], ["code", "Code"], ["service", "Services"]] as [key, label]}
                <button
                    class="filter-chip px-3.5 py-1.25 rounded-full border border-surface-z3 bg-transparent text-xs cursor-pointer text-surface-z7"
                    class:active={kindFilter === key}
                    onclick={() => (kindFilter = key as any)}>{label}</button
                >
            {/each}
        </div>
    </div>

    {#if loading}
        <p class="text-ui text-surface-z6">Loading libraries...</p>
    {:else if filtered.length === 0}
        <div class="flex flex-col items-center text-center py-20 gap-4">
            <span class="kanji text-6xl text-primary-z5 opacity-30">書</span>
            <p class="display text-xl font-normal m-0">No libraries indexed.</p>
            <p
                class="text-ui text-surface-z6 max-w-[380px] leading-relaxed m-0"
            >
                Libraries appear once sensei scans your project dependencies.
                Add folders in the setup wizard, and sensei will detect
                libraries from your manifests.
            </p>
        </div>
    {:else}
        <div class="grid grid-cols-[1fr_340px] gap-6">
            <div class="flex flex-col gap-1">
                {#each filtered as lib (lib.name)}
                    <button
                        class="lib-card text-left px-4 py-3.5 border border-surface-z3 rounded-md bg-surface-z1 cursor-pointer transition-colors duration-100"
                        class:selected={selectedLib?.name === lib.name}
                        onclick={() => (selectedLib = lib)}
                    >
                        <div class="flex items-baseline gap-2 mb-1.5">
                            <span class="text-ui font-medium text-surface-z9"
                                >{lib.name}</span
                            >
                            <span class="text-2xs text-surface-z6"
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
                    class="p-6 bg-surface-z2 border border-surface-z3 rounded-lg sticky top-6"
                >
                    <h3 class="text-base font-medium m-0 mb-4">
                        {selectedLib.name}
                    </h3>
                    <div>
                        <p
                            class="text-micro tracking-label uppercase text-surface-z6 m-0 mb-2"
                        >
                            Used in
                        </p>
                        <div class="flex flex-wrap gap-1.5">
                            {#each selectedLib.repos as repo}
                                <span
                                    class="inline-block px-2 py-0.5 rounded-full text-3xs bg-surface-z3 text-surface-z6 lowercase"
                                    >{repo}</span
                                >
                            {/each}
                        </div>
                    </div>
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .lib-search:focus {
        border-color: oklch(var(--color-surface-z6) / 1);
    }

    .filter-chip:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
    .filter-chip.active {
        background: oklch(var(--color-surface-z9) / 1);
        color: oklch(var(--color-surface-z1) / 1);
        border-color: oklch(var(--color-surface-z9) / 1);
    }

    .lib-card:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
    .lib-card.selected {
        border-color: oklch(var(--color-surface-z6) / 1);
        background: oklch(var(--color-surface-z2) / 1);
    }
</style>
