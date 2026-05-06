<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";

    type Memory = {
        id: string;
        title: string;
        content: string;
        scope: string;
        strength: number;
        status: string;
        type: string;
        project_name?: string;
    };

    let memories = $state<Memory[]>([]);
    let loading = $state(true);
    let tab = $state<"all" | "memories" | "patterns" | "corrections">("all");
    let selectedMemory = $state<Memory | null>(null);

    onMount(async () => {
        await appState.load();
        // Memories API not yet wired — will populate when daemon supports it
        loading = false;
    });

    function strengthDots(s: number): string {
        const filled = Math.round(s);
        return "●".repeat(filled) + "○".repeat(5 - filled);
    }
</script>

<div class="max-w-[960px] mx-auto px-12 py-12 pb-16">
    <div class="mb-6">
        <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-2">
            Learnings
        </p>
        <h1 class="display text-2xl font-normal m-0">學 Learnings</h1>
    </div>

    <!-- Tabs -->
    <div class="flex gap-0 border-b border-surface-z2 mb-7">
        {#each [["all", "All"], ["memories", "Memories"], ["patterns", "Patterns"], ["corrections", "Corrections"]] as [key, label]}
            <button
                class="tab px-4.5 py-2 border-none bg-none text-ui cursor-pointer border-b-2 border-transparent -mb-px text-surface-z6"
                class:active={tab === key}
                onclick={() => (tab = key as any)}>{label}</button
            >
        {/each}
    </div>

    {#if loading}
        <p class="text-ui text-surface-z6">Loading...</p>
    {:else if memories.length === 0}
        <div class="flex flex-col items-center text-center py-20 gap-4">
            <span class="kanji text-6xl text-primary-z5 opacity-30">學</span>
            <p class="display text-xl font-normal m-0">Nothing learned yet.</p>
            <p
                class="text-ui text-surface-z6 max-w-[380px] leading-relaxed m-0"
            >
                As you work with your assistants, sensei observes corrections
                and patterns. Learnings appear here once sensei has enough
                evidence to teach.
            </p>
        </div>
    {:else}
        <div class="grid grid-cols-[1fr_340px] gap-6">
            <!-- Memory list -->
            <div class="flex flex-col gap-1">
                {#each memories as mem (mem.id)}
                    <button
                        class="memory-card text-left px-4 py-3.5 border border-surface-z3 rounded-md bg-surface-z1 cursor-pointer transition-colors duration-100"
                        class:selected={selectedMemory?.id === mem.id}
                        onclick={() => (selectedMemory = mem)}
                    >
                        <div class="flex justify-between mb-1.5">
                            <span
                                class="text-3xs text-primary-z5 tracking-wide"
                                title="Strength {mem.strength}/5"
                                >{strengthDots(mem.strength)}</span
                            >
                            <span
                                class="text-3xs text-surface-z5 uppercase tracking-widest"
                                >{mem.scope}</span
                            >
                        </div>
                        <p class="text-ui text-surface-z9 m-0 leading-normal">
                            {mem.title}
                        </p>
                        {#if mem.project_name}
                            <span
                                class="text-2xs text-surface-z6 mt-1 inline-block"
                                >{mem.project_name}</span
                            >
                        {/if}
                    </button>
                {/each}
            </div>

            <!-- Detail drawer -->
            {#if selectedMemory}
                <div
                    class="p-6 bg-surface-z2 border border-surface-z3 rounded-lg sticky top-6 flex flex-col gap-5"
                >
                    {#each [{ label: "What", text: selectedMemory.title }, { label: "Why", text: selectedMemory.content }] as section}
                        <div>
                            <p
                                class="text-micro tracking-label uppercase text-surface-z6 m-0 mb-1.5"
                            >
                                {section.label}
                            </p>
                            <p
                                class="text-ui text-surface-z9 m-0 leading-relaxed"
                            >
                                {section.text}
                            </p>
                        </div>
                    {/each}
                    <div>
                        <p
                            class="text-micro tracking-label uppercase text-surface-z6 m-0 mb-1.5"
                        >
                            Scope
                        </p>
                        <span
                            class="inline-block px-2.5 py-0.75 rounded-full text-2xs bg-surface-z3 text-surface-z7"
                            >{selectedMemory.scope}</span
                        >
                    </div>
                    <div>
                        <p
                            class="text-micro tracking-label uppercase text-surface-z6 m-0 mb-1.5"
                        >
                            Strength
                        </p>
                        <span class="text-ui text-primary-z5"
                            >{strengthDots(selectedMemory.strength)}
                            {selectedMemory.strength}/5</span
                        >
                    </div>
                    <div>
                        <p
                            class="text-micro tracking-label uppercase text-surface-z6 m-0 mb-1.5"
                        >
                            Status
                        </p>
                        <span
                            class="inline-block px-2.5 py-0.75 rounded-full text-2xs bg-surface-z3 text-surface-z7"
                            >{selectedMemory.status}</span
                        >
                    </div>
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .tab:hover {
        color: oklch(var(--color-surface-z8) / 1);
    }
    .tab.active {
        color: oklch(var(--color-surface-z9) / 1);
        border-bottom-color: oklch(var(--color-primary-z5) / 1);
    }

    .memory-card:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
    .memory-card.selected {
        border-color: oklch(var(--color-surface-z6) / 1);
        background: oklch(var(--color-surface-z2) / 1);
    }
</style>
