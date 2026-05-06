<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";

    type Tool = { name: string; description: string; params: string[] };

    let tools = $state<Tool[]>([]);
    let loading = $state(true);
    let tab = $state<"playground" | "replay" | "insights">("playground");
    let selectedTool = $state<Tool | null>(null);
    let toolResult = $state<string>("");
    let toolParams = $state<Record<string, string>>({});
    let executing = $state(false);

    onMount(async () => {
        await appState.load();
        const api = senseiApi(appState.port);
        const data = await api.mcpListTools();
        tools = data.tools;
        loading = false;
    });

    async function executeTool() {
        if (!selectedTool) return;
        executing = true;
        toolResult = "";
        const api = senseiApi(appState.port);
        const result = await api.mcpCallTool(selectedTool.name, toolParams);
        toolResult = JSON.stringify(result, null, 2);
        executing = false;
    }
</script>

<div class="max-w-[960px] mx-auto px-12 py-12 pb-16">
    <div class="mb-6">
        <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-2">
            Instruments
        </p>
        <h1 class="display text-2xl font-normal m-0">具 Instruments</h1>
    </div>

    <!-- Tab bar -->
    <div class="flex gap-0 border-b border-surface-z2 mb-7">
        {#each [["playground", "具", "Playground"], ["replay", "録", "Replay"], ["insights", "照", "Insights"]] as [key, kanji, label]}
            <button
                class="tab flex items-center gap-2 px-4.5 py-2 border-none bg-none text-ui cursor-pointer border-b-2 border-transparent -mb-px text-surface-z6"
                class:active={tab === key}
                onclick={() => (tab = key as any)}
            >
                <span class="kanji text-xs">{kanji}</span>
                {label}
            </button>
        {/each}
    </div>

    {#if tab === "playground"}
        {#if loading}
            <p class="text-ui text-surface-z6">Loading tools...</p>
        {:else if tools.length === 0}
            <div class="flex flex-col items-center text-center py-20 gap-4">
                <span class="kanji text-6xl text-primary-z5 opacity-30">具</span
                >
                <p class="display text-xl font-normal m-0">
                    No MCP tools available.
                </p>
                <p
                    class="text-ui text-surface-z6 max-w-[380px] leading-relaxed m-0"
                >
                    Tools appear when the sensei daemon is running and MCP
                    services are configured. Check your instruments in the setup
                    wizard.
                </p>
            </div>
        {:else}
            <div class="grid grid-cols-[260px_1fr] gap-6">
                <!-- Tool list -->
                <div class="flex flex-col gap-0.5">
                    {#each tools as tool (tool.name)}
                        <button
                            class="tool-card text-left px-3.5 py-2.5 rounded-md bg-transparent border-none cursor-pointer transition-colors duration-100"
                            class:selected={selectedTool?.name === tool.name}
                            onclick={() => {
                                selectedTool = tool;
                                toolParams = {};
                                toolResult = "";
                            }}
                        >
                            <span
                                class="block text-ui font-medium text-surface-z9 font-mono"
                                >{tool.name}</span
                            >
                            <span class="block text-2xs text-surface-z6 mt-0.5"
                                >{tool.description}</span
                            >
                        </button>
                    {/each}
                </div>

                <!-- Tool detail + execution -->
                <div
                    class="p-6 bg-surface-z2 border border-surface-z3 rounded-lg"
                >
                    {#if selectedTool}
                        <h3 class="text-base font-mono m-0 mb-1.5">
                            {selectedTool.name}
                        </h3>
                        <p
                            class="text-ui text-surface-z7 m-0 mb-5 leading-normal"
                        >
                            {selectedTool.description}
                        </p>

                        {#if selectedTool.params.length > 0}
                            <div class="flex flex-col gap-3 mb-5">
                                {#each selectedTool.params as param}
                                    <div class="flex flex-col gap-1">
                                        <label
                                            class="text-2xs text-surface-z6 font-mono"
                                            for="param-{param}">{param}</label
                                        >
                                        <input
                                            id="param-{param}"
                                            class="param-input px-3 py-2 border border-surface-z3 rounded-md bg-surface-z1 text-surface-z9 text-ui font-mono outline-none"
                                            type="text"
                                            placeholder={param}
                                            bind:value={toolParams[param]}
                                        />
                                    </div>
                                {/each}
                            </div>
                        {/if}

                        <button
                            class="btn-solid"
                            onclick={executeTool}
                            disabled={executing}
                        >
                            {executing ? "Running..." : "Execute"}
                        </button>

                        {#if toolResult}
                            <div class="mt-5">
                                <p
                                    class="text-micro tracking-label uppercase text-surface-z6 m-0 mb-2"
                                >
                                    Response
                                </p>
                                <pre
                                    class="px-4 py-4 bg-surface-z1 border border-surface-z3 rounded-md text-xs font-mono text-surface-z9 overflow-auto max-h-[400px] whitespace-pre-wrap break-all m-0">{toolResult}</pre>
                            </div>
                        {/if}
                    {:else}
                        <p class="text-ui text-surface-z6">
                            Select a tool to try it.
                        </p>
                    {/if}
                </div>
            </div>
        {/if}
    {:else if tab === "replay"}
        <div class="flex flex-col items-center text-center py-20 gap-4">
            <span class="kanji text-6xl text-primary-z5 opacity-30">録</span>
            <p class="display text-xl font-normal m-0">Session replay</p>
            <p
                class="text-ui text-surface-z6 max-w-[380px] leading-relaxed m-0"
            >
                Tool calls from your assistant sessions will appear here. Each
                call shows the tool, arguments, response, and whether the
                assistant used the result.
            </p>
        </div>
    {:else}
        <div class="flex flex-col items-center text-center py-20 gap-4">
            <span class="kanji text-6xl text-primary-z5 opacity-30">照</span>
            <p class="display text-xl font-normal m-0">Tool insights</p>
            <p
                class="text-ui text-surface-z6 max-w-[380px] leading-relaxed m-0"
            >
                Aggregated usage and effectiveness metrics across sessions.
                Which tools are used most, which responses get ignored, and
                where tool usage correlates with FTR.
            </p>
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

    .tool-card:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
    .tool-card.selected {
        background: oklch(var(--color-surface-z2) / 1);
    }

    .param-input:focus {
        border-color: oklch(var(--color-surface-z6) / 1);
    }
</style>
