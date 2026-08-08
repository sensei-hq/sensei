<script lang="ts">
    import TabBar from "./TabBar.svelte";
    import EmptyState from "./EmptyState.svelte";

    let {
        pageLabel,
        pageTitle,
    }: {
        pageLabel: string;
        pageTitle: string;
    } = $props();

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
    let loading = $state(false);
    let tab = $state("all");
    let selectedMemory = $state<Memory | null>(null);

    const tabs: [string, string][] = [
        ["all", "All"],
        ["memories", "Memories"],
        ["patterns", "Patterns"],
        ["corrections", "Corrections"],
    ];

    function strengthDots(s: number): string {
        const filled = Math.round(s);
        return "\u25CF".repeat(filled) + "\u25CB".repeat(5 - filled);
    }
</script>

<div class="max-w-[960px] mx-auto px-12 py-12 pb-16">
    <div class="mb-6">
        <p class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-2">
            {pageLabel}
        </p>
        <h1 class="display text-2xl font-normal m-0">{pageTitle}</h1>
    </div>

    <TabBar {tabs} bind:active={tab} class="mb-7" />

    {#if loading}
        <p class="text-sm text-ink-soft">Loading...</p>
    {:else if memories.length === 0}
        <EmptyState
            kanji="學"
            title="Nothing learned yet."
            description="As you work with your assistants, sensei observes corrections and patterns. Learnings appear here once sensei has enough evidence to teach."
        />
    {:else}
        <div class="grid grid-cols-[1fr_340px] gap-6">
            <!-- Memory list -->
            <div class="flex flex-col gap-1">
                {#each memories as mem (mem.id)}
                    <button
                        class="memory-card text-left px-4 py-3.5 border border-paper-edge rounded-md bg-paper-soft cursor-pointer transition-colors duration-fast"
                        class:selected={selectedMemory?.id === mem.id}
                        onclick={() => (selectedMemory = mem)}
                    >
                        <div class="flex justify-between mb-1.5">
                            <span
                                class="text-xs text-accent tracking-wide"
                                title="Strength {mem.strength}/5"
                                >{strengthDots(mem.strength)}</span
                            >
                            <span
                                class="text-xs text-ink-soft uppercase tracking-wide"
                                >{mem.scope}</span
                            >
                        </div>
                        <p class="text-sm text-ink m-0 leading-normal">
                            {mem.title}
                        </p>
                        {#if mem.project_name}
                            <span
                                class="text-xs text-ink-soft mt-1 inline-block"
                                >{mem.project_name}</span
                            >
                        {/if}
                    </button>
                {/each}
            </div>

            <!-- Detail drawer -->
            {#if selectedMemory}
                <div
                    class="p-6 bg-paper-mute border border-paper-edge rounded-lg sticky top-6 flex flex-col gap-5"
                >
                    {#each [{ label: "What", text: selectedMemory.title }, { label: "Why", text: selectedMemory.content }] as section}
                        <div>
                            <p
                                class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-1.5"
                            >
                                {section.label}
                            </p>
                            <p
                                class="text-sm text-ink m-0 leading-normal"
                            >
                                {section.text}
                            </p>
                        </div>
                    {/each}
                    <div>
                        <p
                            class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-1.5"
                        >
                            Scope
                        </p>
                        <span
                            class="inline-block px-2.5 py-1 rounded-full text-xs bg-paper-mute text-ink-mute"
                            >{selectedMemory.scope}</span
                        >
                    </div>
                    <div>
                        <p
                            class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-1.5"
                        >
                            Strength
                        </p>
                        <span class="text-sm text-accent"
                            >{strengthDots(selectedMemory.strength)}
                            {selectedMemory.strength}/5</span
                        >
                    </div>
                    <div>
                        <p
                            class="text-xs tracking-wide uppercase text-ink-soft m-0 mb-1.5"
                        >
                            Status
                        </p>
                        <span
                            class="inline-block px-2.5 py-1 rounded-full text-xs bg-paper-mute text-ink-mute"
                            >{selectedMemory.status}</span
                        >
                    </div>
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .memory-card:hover {
        background: var(--paper-mute);
    }
    .memory-card.selected {
        border-color: var(--ink-soft);
        background: var(--paper-mute);
    }
</style>
