<script lang="ts">
    import { wizardState } from "$lib/wizard-state.svelte.js";

    const assistants = $derived(wizardState.assistants.assistants);

    // What gets registered when each family is enabled.
    // Claude gets the full plugin suite; all others get MCP server only.
    const CAPABILITIES: Record<string, string[]> = {
        claude: ["plugins", "skills", "commands", "agents"],
    };

    function caps(id: string): string[] {
        return CAPABILITIES[id] ?? ["mcp server"];
    }

    function toggle(id: string) {
        const fam = assistants.find((a) => a.id === id);
        if (fam && fam.variants.some((v) => v.installed))
            fam.selected = !fam.selected;
    }
</script>

<div>
    <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
        Registers plugins, skills, commands, agents, logging and metrics.
    </p>

    <div class="grid grid-cols-2 gap-3">
        {#each assistants as fam (fam.id)}
            {@const installedCount = fam.variants.filter(
                (v) => v.installed,
            ).length}
            {@const anyInstalled = installedCount > 0}
            <button
                class="card flex items-center gap-4 px-6 py-5 rounded-lg cursor-pointer text-left bg-surface-z1 border border-surface-z3 transition-all duration-fast min-w-0"
                class:card-selected={fam.selected}
                class:card-missing={!anyInstalled}
                onclick={() => toggle(fam.id)}
            >
                <div class="flex-1 min-w-0">
                    <div
                        class="flex items-baseline justify-between gap-2 mb-1.5"
                    >
                        <span class="text-base font-semibold">{fam.name}</span>
                        {#if anyInstalled}
                            <span
                                class="text-xs text-surface-z5 whitespace-nowrap"
                                >{installedCount} detected</span
                            >
                        {:else}
                            <span
                                class="text-xs text-surface-z5 italic whitespace-nowrap"
                                >not found</span
                            >
                        {/if}
                    </div>
                    <div class="flex flex-wrap gap-1">
                        {#each caps(fam.id) as cap}
                            <span
                                class="chip text-xs font-mono text-surface-z7 px-2 py-0.5 bg-surface-z3 rounded-sm whitespace-nowrap"
                                >{cap}</span
                            >
                        {/each}
                    </div>
                </div>
                <div
                    class="checkbox w-5.5 h-5.5 rounded-md border-2 border-surface-z3 flex items-center justify-center shrink-0 bg-surface-z1 transition-all duration-fast"
                    class:checked={fam.selected}
                >
                    {#if fam.selected}<span
                            class="text-surface-z1 text-xs font-bold"
                            >&#10003;</span
                        >{/if}
                </div>
            </button>
        {/each}
    </div>

    {#if assistants.length === 0}
        <p class="text-sm text-surface-z6 italic">
            No AI coding assistants detected. Make sure the daemon is running.
        </p>
    {/if}
</div>

<style>
    /* Card selected state */
    .card-selected {
        border: 1.5px solid oklch(var(--color-surface-z6) / 1);
        background: oklch(var(--color-surface-z2) / 1);
    }
    .card-selected .chip {
        background: oklch(var(--color-surface-z1) / 1);
    }

    /* Card missing (unavailable) */
    .card-missing {
        opacity: 0.55;
        cursor: default;
    }
    .card-missing:hover {
        opacity: 0.7;
    }

    /* Checkbox checked */
    .checkbox.checked {
        border-color: oklch(var(--color-surface-z5) / 1);
        background: oklch(var(--color-surface-z6) / 1);
    }
</style>
