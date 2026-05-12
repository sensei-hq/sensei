<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import TabBar from "$lib/components/TabBar.svelte";
    import EmptyState from "$lib/components/EmptyState.svelte";

    type Tool = { name: string; description: string; params: string[] };
    type ToolStat = { tool_name: string; call_count: number; error_count: number; avg_duration_ms: number | null; last_used_at: string };

    let tools = $state<Tool[]>([]);
    let toolStats = $state<ToolStat[]>([]);
    let loading = $state(true);
    let tab = $state("playground");
    let selectedTool = $state<Tool | null>(null);
    let toolResult = $state<string>("");
    let toolParams = $state<Record<string, string>>({});
    let executing = $state(false);

    const instrumentTabs: [string, string][] = [
        ["playground", "Playground"],
        ["replay", "Replay"],
        ["insights", "Insights"],
    ];

    onMount(async () => {
        const api = senseiApi(appState.port);
        const [data, stats] = await Promise.all([
            api.mcpListTools(),
            api.getToolUsage(),
        ]);
        tools = data.tools;
        toolStats = stats.tools ?? [];
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

    <TabBar tabs={instrumentTabs} bind:active={tab} class="mb-7" />

    {#if tab === "playground"}
        {#if loading}
            <p class="text-ui text-surface-z6">Loading tools...</p>
        {:else if tools.length === 0}
            <EmptyState
                kanji="具"
                title="No MCP tools available."
                description="Tools appear when the sensei daemon is running and MCP services are configured. Check your instruments in the setup wizard."
            />
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
        <EmptyState
            kanji="録"
            title="Session replay"
            description="Tool calls from your assistant sessions will appear here. Each call shows the tool, arguments, response, and whether the assistant used the result."
        />
    {:else}
        {@render ToolInsights()}
    {/if}
</div>

{#snippet ToolInsights()}
    {#if toolStats.length === 0}
        <EmptyState
            kanji="照"
            title="No tool usage data yet"
            description="Tool usage statistics appear after your assistant sessions call sensei tools. Start a session to begin tracking."
        />
    {:else}
        <div class="flex flex-col gap-1">
            <div class="grid grid-cols-[1fr_80px_80px_100px_120px] gap-3 px-3 py-2 text-2xs text-surface-z6 tracking-loose uppercase">
                <span>Tool</span>
                <span class="text-right">Calls</span>
                <span class="text-right">Errors</span>
                <span class="text-right">Avg ms</span>
                <span class="text-right">Last used</span>
            </div>
            {#each toolStats as stat (stat.tool_name)}
                {@const errorRate = stat.call_count > 0 ? stat.error_count / stat.call_count : 0}
                <div class="grid grid-cols-[1fr_80px_80px_100px_120px] gap-3 px-3 py-2.5 border-b border-surface-z2 text-ui items-center">
                    <span class="font-mono text-xs">{stat.tool_name}</span>
                    <span class="text-right mono text-xs">{stat.call_count}</span>
                    <span class="text-right mono text-xs" class:text-error={errorRate > 0.1}>
                        {stat.error_count}
                        {#if errorRate > 0}
                            <span class="text-2xs opacity-50">({Math.round(errorRate * 100)}%)</span>
                        {/if}
                    </span>
                    <span class="text-right mono text-xs opacity-70">
                        {stat.avg_duration_ms != null ? Math.round(stat.avg_duration_ms) : '—'}
                    </span>
                    <span class="text-right text-2xs text-surface-z6">
                        {new Date(stat.last_used_at).toLocaleDateString()}
                    </span>
                </div>
            {/each}
        </div>
    {/if}
{/snippet}

<style>
    .tool-card:hover {
        background: oklch(var(--color-surface-z2) / 1);
    }
    .tool-card.selected {
        background: oklch(var(--color-surface-z2) / 1);
    }

    .param-input:focus {
        border-color: oklch(var(--color-surface-z6) / 1);
    }
    .text-error {
        color: oklch(var(--color-primary-z5) / 1);
    }
</style>
