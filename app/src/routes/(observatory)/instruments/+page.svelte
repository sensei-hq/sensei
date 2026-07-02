<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import TabBar from "$lib/components/TabBar.svelte";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import { Eyebrow, PageHeader } from "$lib/components";
    import type { McpToolManifest, SessionToolCall } from "$lib/types.js";

    type Tool = McpToolManifest;
    type ToolStat = { tool_name: string; call_count: number; error_count: number; avg_duration_ms: number | null; last_used_at: string };
    type SessionRow = { id: string; task: string; startedAt: string; ftr?: number | null };

    let tools = $state<Tool[]>([]);
    let toolStats = $state<ToolStat[]>([]);
    let loading = $state(true);
    let tab = $state("playground");
    let kindFilter = $state<'all' | 'query' | 'action'>('all');
    let selectedTool = $state<Tool | null>(null);
    let toolResult = $state<string>("");
    let toolParams = $state<Record<string, string>>({});
    let executing = $state(false);

    // Replay tab state — populated lazily on first tab activation to keep the
    // Playground / Insights load path light.
    let replaySessions = $state<SessionRow[]>([]);
    let selectedSessionId = $state<string | null>(null);
    let sessionCalls = $state<SessionToolCall[]>([]);
    let replayLoading = $state(false);
    let selectedCall = $state<SessionToolCall | null>(null);

    const instrumentTabs: [string, string][] = [
        ["playground", "Playground"],
        ["replay", "Replay"],
        ["insights", "Insights"],
    ];

    const kindChips: Array<{ id: 'all' | 'query' | 'action'; label: string }> = [
        { id: 'all', label: 'All' },
        { id: 'query', label: 'Queries' },
        { id: 'action', label: 'Actions' },
    ];

    // Filter tools by the active kind chip. `all` keeps the full list; kind
    // filter narrows to matching tools. Filter is applied client-side so the
    // chip switches are instant with no daemon round-trip.
    const visibleTools = $derived(
        kindFilter === 'all' ? tools : tools.filter((t) => t.kind === kindFilter),
    );

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

    // Load the session list once the Replay tab is first opened. Subsequent
    // tab switches skip the fetch since the list rarely changes mid-session.
    async function ensureReplaySessionsLoaded() {
        if (replaySessions.length > 0) return;
        replayLoading = true;
        const api = senseiApi(appState.port);
        const data = await api.getSessions();
        replaySessions = (data.sessions ?? []).map((s) => ({
            id: s.id,
            task: s.task,
            startedAt: s.startedAt,
            ftr: s.ftr,
        }));
        replayLoading = false;
    }

    async function selectReplaySession(sessionId: string) {
        selectedSessionId = sessionId;
        selectedCall = null;
        sessionCalls = [];
        replayLoading = true;
        const api = senseiApi(appState.port);
        const timeline = await api.getSessionToolTimeline(sessionId, 200);
        sessionCalls = timeline.calls;
        replayLoading = false;
    }

    // Kick off session loading the first time Replay is visible.
    $effect(() => {
        if (tab === 'replay') {
            void ensureReplaySessionsLoaded();
        }
    });

    // Format request/response payloads for the detail pane — pretty JSON
    // when we have an object, raw string otherwise.
    function fmtPayload(value: unknown): string {
        if (value == null) return '—';
        if (typeof value === 'string') return value;
        try {
            return JSON.stringify(value, null, 2);
        } catch {
            return String(value);
        }
    }

    function fmtDuration(ms: number | null): string {
        if (ms == null) return '—';
        if (ms < 1000) return `${ms} ms`;
        return `${(ms / 1000).toFixed(2)} s`;
    }

    function fmtDate(iso: string): string {
        return new Date(iso).toLocaleString();
    }
</script>

<PageHeader kanji="具" eyebrow="Instruments" title="Instruments" />
<div class="max-w-[960px] mx-auto px-12 pt-8 pb-16">

    <TabBar tabs={instrumentTabs} bind:active={tab} class="mb-7" />

    {#if tab === "playground"}
        {#if loading}
            <p class="text-sm text-ink-soft">Loading tools...</p>
        {:else if tools.length === 0}
            <EmptyState
                kanji="具"
                title="No MCP tools available."
                description="Tools appear when the sensei daemon is running and MCP services are configured. Check your instruments in the setup wizard."
            />
        {:else}
            <!-- Kind chips — filter tools by query vs action -->
            <div class="flex gap-2 mb-4" role="tablist" aria-label="Tool kind filter">
                {#each kindChips as chip}
                    {@const active = kindFilter === chip.id}
                    <button
                        class="px-3 py-1 rounded-full border text-xs cursor-pointer transition-colors duration-fast"
                        class:bg-primary={active}
                        class:text-on-primary={active}
                        class:border-primary={active}
                        class:bg-transparent={!active}
                        class:text-ink-soft={!active}
                        class:border-paper-mute={!active}
                        role="tab"
                        aria-selected={active}
                        onclick={() => (kindFilter = chip.id)}
                    >
                        {chip.label}
                    </button>
                {/each}
            </div>

            <div class="grid grid-cols-[260px_1fr] gap-6">
                <!-- Tool list -->
                <div class="flex flex-col gap-0.5">
                    {#each visibleTools as tool (tool.name)}
                        <button
                            class="tool-card text-left px-3.5 py-2.5 rounded-md bg-transparent border-none cursor-pointer transition-colors duration-fast"
                            class:selected={selectedTool?.name === tool.name}
                            onclick={() => {
                                selectedTool = tool;
                                toolParams = {};
                                toolResult = "";
                            }}
                        >
                            <span
                                class="block text-sm font-medium text-ink font-mono"
                                >{tool.name}</span
                            >
                            <span class="block text-xs text-ink-soft mt-0.5"
                                >{tool.summary}</span
                            >
                        </button>
                    {/each}
                    {#if visibleTools.length === 0}
                        <p class="text-xs text-ink-soft px-3 py-2">
                            No {kindFilter === 'query' ? 'queries' : 'actions'} match this filter.
                        </p>
                    {/if}
                </div>

                <!-- Tool detail + execution -->
                <div
                    class="p-6 bg-paper-mute border border-paper-mute rounded-lg"
                >
                    {#if selectedTool}
                        <h3 class="text-base font-mono m-0 mb-1.5">
                            {selectedTool.name}
                        </h3>
                        <p
                            class="text-sm text-ink-mute m-0 mb-5 leading-normal"
                        >
                            {selectedTool.summary}
                        </p>

                        {#if selectedTool.inputs.length > 0}
                            <div class="flex flex-col gap-3 mb-5">
                                {#each selectedTool.inputs as input}
                                    <div class="flex flex-col gap-1">
                                        <label
                                            class="text-xs text-ink-soft font-mono"
                                            for="param-{input.key}">{input.label}{input.required ? ' *' : ''}</label
                                        >
                                        {#if input.kind === 'enum' && input.options}
                                            <select
                                                id="param-{input.key}"
                                                class="param-input px-3 py-2 border border-paper-mute rounded-md bg-paper-soft text-ink text-sm font-mono outline-none"
                                                bind:value={toolParams[input.key]}
                                            >
                                                {#each input.options as option}
                                                    <option value={option}>{option}</option>
                                                {/each}
                                            </select>
                                        {:else}
                                            <input
                                                id="param-{input.key}"
                                                class="param-input px-3 py-2 border border-paper-mute rounded-md bg-paper-soft text-ink text-sm font-mono outline-none"
                                                type={input.kind === 'number' ? 'number' : 'text'}
                                                placeholder={input.placeholder ?? input.default ?? input.key}
                                                bind:value={toolParams[input.key]}
                                            />
                                        {/if}
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
                                <p class="m-0 mb-2"><Eyebrow>Response</Eyebrow></p>
                                <pre
                                    class="px-4 py-4 bg-paper-soft border border-paper-mute rounded-md text-xs font-mono text-ink overflow-auto max-h-[400px] whitespace-pre-wrap break-all m-0">{toolResult}</pre>
                            </div>
                        {/if}
                    {:else}
                        <p class="text-sm text-ink-soft">
                            Select a tool to try it.
                        </p>
                    {/if}
                </div>
            </div>
        {/if}
    {:else if tab === "replay"}
        {#if replayLoading && replaySessions.length === 0}
            <p class="text-sm text-ink-soft">Loading sessions…</p>
        {:else if replaySessions.length === 0}
            <EmptyState
                kanji="録"
                title="No sessions recorded yet"
                description="Tool calls from your assistant sessions appear here once sensei has captured at least one session."
            />
        {:else}
            <div class="grid grid-cols-[220px_260px_1fr] gap-6">
                <!-- Session picker -->
                <div class="flex flex-col gap-0.5 max-h-[560px] overflow-auto">
                    <div class="text-xs uppercase tracking-wide text-ink-mute px-3 py-2">Sessions</div>
                    {#each replaySessions as session (session.id)}
                        <button
                            class="tool-card text-left px-3.5 py-2 rounded-md bg-transparent border-none cursor-pointer transition-colors duration-fast"
                            class:selected={selectedSessionId === session.id}
                            onclick={() => selectReplaySession(session.id)}
                        >
                            <span class="block text-sm font-medium text-ink truncate">{session.task}</span>
                            <span class="block text-xs text-ink-soft mt-0.5">
                                {fmtDate(session.startedAt)}
                                {#if session.ftr != null}· FTR {(session.ftr * 100).toFixed(0)}%{/if}
                            </span>
                        </button>
                    {/each}
                </div>

                <!-- Call list -->
                <div class="flex flex-col gap-0.5 max-h-[560px] overflow-auto">
                    <div class="text-xs uppercase tracking-wide text-ink-mute px-3 py-2">Calls</div>
                    {#if selectedSessionId == null}
                        <p class="text-xs text-ink-soft px-3">Pick a session.</p>
                    {:else if replayLoading}
                        <p class="text-xs text-ink-soft px-3">Loading timeline…</p>
                    {:else if sessionCalls.length === 0}
                        <p class="text-xs text-ink-soft px-3">No tool calls in this session.</p>
                    {:else}
                        {#each sessionCalls as call (call.callId)}
                            <button
                                class="tool-card text-left px-3.5 py-2 rounded-md bg-transparent border-none cursor-pointer transition-colors duration-fast flex items-center gap-2"
                                class:selected={selectedCall?.callId === call.callId}
                                onclick={() => (selectedCall = call)}
                            >
                                <span class="block text-sm font-mono text-ink truncate flex-1">{call.toolName}</span>
                                {#if call.inFlight}
                                    <span class="text-xs text-warning">in-flight</span>
                                {:else if call.success === false}
                                    <span class="text-xs text-danger">✗</span>
                                {:else if call.success === true}
                                    <span class="text-xs text-success">✓</span>
                                {/if}
                                <span class="text-xs text-ink-soft">{fmtDuration(call.durationMs)}</span>
                            </button>
                        {/each}
                    {/if}
                </div>

                <!-- Call detail -->
                <div class="p-6 bg-paper-mute border border-paper-mute rounded-lg max-h-[560px] overflow-auto">
                    {#if !selectedCall}
                        <p class="text-sm text-ink-soft">Select a call to see request + response.</p>
                    {:else}
                        <h3 class="text-base font-mono m-0 mb-1.5">{selectedCall.toolName}</h3>
                        <p class="text-xs text-ink-mute m-0 mb-4">
                            {fmtDate(selectedCall.startedAt)} · {fmtDuration(selectedCall.durationMs)}
                            {#if selectedCall.inFlight} · in-flight{/if}
                        </p>

                        <p class="m-0 mb-1"><Eyebrow>Request</Eyebrow></p>
                        <pre class="px-3 py-2 bg-paper-soft border border-paper-mute rounded-md text-xs font-mono text-ink overflow-auto whitespace-pre-wrap break-all m-0 mb-4">{fmtPayload(selectedCall.request)}</pre>

                        <p class="m-0 mb-1"><Eyebrow>Response</Eyebrow></p>
                        {#if selectedCall.response == null}
                            <p class="text-xs text-ink-soft m-0">No response captured yet.</p>
                        {:else}
                            <pre class="px-3 py-2 bg-paper-soft border border-paper-mute rounded-md text-xs font-mono text-ink overflow-auto whitespace-pre-wrap break-all m-0">{fmtPayload(selectedCall.response)}</pre>
                        {/if}
                    {/if}
                </div>
            </div>
        {/if}
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
            <div class="grid grid-cols-[1fr_80px_80px_100px_120px] gap-3 px-3 py-2 text-xs text-ink-soft tracking-wide uppercase">
                <span>Tool</span>
                <span class="text-right">Calls</span>
                <span class="text-right">Errors</span>
                <span class="text-right">Avg ms</span>
                <span class="text-right">Last used</span>
            </div>
            {#each toolStats as stat (stat.tool_name)}
                {@const errorRate = stat.call_count > 0 ? stat.error_count / stat.call_count : 0}
                <div class="grid grid-cols-[1fr_80px_80px_100px_120px] gap-3 px-3 py-2.5 border-b border-paper-mute text-sm items-center">
                    <span class="font-mono text-xs">{stat.tool_name}</span>
                    <span class="text-right font-mono text-xs">{stat.call_count}</span>
                    <span class="text-right font-mono text-xs" class:text-error={errorRate > 0.1}>
                        {stat.error_count}
                        {#if errorRate > 0}
                            <span class="text-xs opacity-50">({Math.round(errorRate * 100)}%)</span>
                        {/if}
                    </span>
                    <span class="text-right font-mono text-xs opacity-70">
                        {stat.avg_duration_ms != null ? Math.round(stat.avg_duration_ms) : '—'}
                    </span>
                    <span class="text-right text-xs text-ink-soft">
                        {new Date(stat.last_used_at).toLocaleDateString()}
                    </span>
                </div>
            {/each}
        </div>
    {/if}
{/snippet}

<style>
    .tool-card:hover {
        background: var(--paper-mute);
    }
    .tool-card.selected {
        background: var(--paper-mute);
    }

    .param-input:focus {
        border-color: var(--ink-soft);
    }
    .text-error {
        color: var(--accent);
    }
</style>
