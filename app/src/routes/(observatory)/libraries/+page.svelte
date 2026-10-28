<script lang="ts">
    import { railIcon } from "../observatory-nav";
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import { Eyebrow, PageHeader, Screen, ScreenState } from "$lib/components";
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

<Screen>
    {#snippet header()}
    <PageHeader kanji="書" icon={railIcon("/libraries")} eyebrow="Libraries" title="Libraries" />
    {/snippet}
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
            <p class="text-sm text-ink-mute">Loading libraries...</p>
        {:else if lib.error}
            <ScreenState status="error" error={lib.error} onretry={() => void lib.load(api)} />
        {:else if filtered.length === 0}
            <EmptyState
                kanji="書"
                title="No libraries indexed."
                description="Libraries appear once sensei scans your workspace dependencies. Add folders in the setup wizard, and sensei will detect libraries from your manifests."
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
                                <span class="text-xs text-ink-mute"
                                    >{l.repoCount} repo{l.repoCount !== 1
                                        ? "s"
                                        : ""}</span
                                >
                                {#if l.skillCount || l.agentCount}
                                    <span class="text-xs text-ink-mute"
                                        >· {l.skillCount ?? 0} skills · {l.agentCount ?? 0} agents</span
                                    >
                                {/if}
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
                        <p class="text-xs text-ink-mute m-0 mb-4">
                            {#if lib.selected.ecosystem}{lib.selected.ecosystem} · {/if}
                            {lib.selected.repoCount} repo{lib.selected.repoCount !== 1 ? 's' : ''}
                            {#if lib.selected.pageCount}· {lib.selected.pageCount} doc pages{/if}
                        </p>

                        {#if lib.selected.packages?.length}
                            <div class="mb-4">
                                <p class="m-0 mb-2"><Eyebrow>Packages</Eyebrow></p>
                                <div class="flex flex-wrap gap-1.5">
                                    {#each lib.selected.packages as pkg (pkg)}
                                        <span class="inline-block px-2 py-0.5 rounded-full text-xs bg-paper-soft text-ink-mute font-mono">{pkg}</span>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                        <div class="mb-4">
                            <p class="m-0 mb-1"><Eyebrow>Provided by the library</Eyebrow></p>
                            <p class="text-xs text-ink-faint m-0 mb-2">From its sensei.library.json — what assistants are given.</p>
                            {#if lib.provided.loading}
                                <p class="text-xs text-ink-mute">Loading...</p>
                            {:else if lib.provided.error}
                                <p class="text-xs text-error">Couldn't load what it provides: {lib.provided.error}</p>
                            {:else if !lib.provided.docs.length && !lib.provided.skills.length && !lib.provided.agents.length}
                                <p class="text-xs text-ink-mute">No docs, skills or agents held for this library.</p>
                            {:else}
                                {@render provides("Skills", lib.provided.skills.map((c) => ({ key: c.name, label: c.name, note: c.focus, body: c.body })))}
                                {@render provides("Agents", lib.provided.agents.map((c) => ({ key: c.name, label: c.name, note: c.focus, body: c.body })))}
                                {@render provides("Docs", lib.provided.docs.map((d, i) => ({ key: `${i}:${d.title}`, label: d.title, note: d.component, body: d.content })))}
                            {/if}
                        </div>

                        <div class="mb-4">
                            <p class="m-0 mb-2"><Eyebrow>Used in</Eyebrow></p>
                            <div class="flex flex-wrap gap-1.5">
                                {#each lib.selected.repos as repo}
                                    <span class="inline-block px-2 py-0.5 rounded-full text-xs bg-paper-mute text-ink-mute lowercase">{repo}</span>
                                {/each}
                            </div>
                        </div>

                        {#if lib.usageLoading}
                            <p class="text-xs text-ink-mute">Loading usage...</p>
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
                                                <span class="text-xs text-ink-mute ml-1.5">v{u.version_used}</span>
                                            {/if}
                                        </div>
                                        <span class="font-mono text-xs text-ink-mute">{u.import_count} imports</span>
                                    </div>
                                {/each}
                            </div>
                        {/if}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</Screen>

{#snippet provides(title: string, items: { key: string; label: string; note: string | null; body: string | null }[])}
    {#if items.length}
        <div class="mb-3">
            <p class="text-xs text-ink-mute m-0 mb-1">{title} · {items.length}</p>
            <ul class="m-0 p-0 list-none max-h-48 overflow-y-auto">
                {#each items as item (item.key)}
                    <li class="border-b border-paper-edge">
                        <details>
                            <summary class="py-1 text-sm cursor-pointer">
                                <span class="font-mono text-xs">{item.label}</span>
                                {#if item.note}<span class="text-xs text-ink-faint ml-1.5">{item.note}</span>{/if}
                            </summary>
                            <pre class="text-xs text-ink-mute whitespace-pre-wrap m-0 mb-2 max-h-64 overflow-y-auto">{item.body ?? "No body held."}</pre>
                        </details>
                    </li>
                {/each}
            </ul>
        </div>
    {/if}
{/snippet}

<style>
    .lib-search:focus {
        border-color: var(--ink-mute);
    }

    .lib-card:hover {
        background: var(--paper-mute);
    }
    .lib-card.selected {
        border-color: var(--ink-mute);
        background: var(--paper-mute);
    }
</style>
