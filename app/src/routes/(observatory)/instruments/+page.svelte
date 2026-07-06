<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import TabBar from "$lib/components/TabBar.svelte";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import SignalCard from "$lib/components/SignalCard.svelte";
    import { Eyebrow, PageHeader } from "$lib/components";
    import { mcp } from "$lib/state/mcp.svelte.js";
    import type {
        McpToolManifest, SessionToolCall, SessionReplayCall,
    } from "$lib/types.js";

    type Tool = McpToolManifest;
    type SessionRow = { id: string; task: string; startedAt: string; ftr?: number | null };

    // All catalogue data lives on the mcp store — page is pure presentation.
    const tools = $derived(mcp.tools);
    const toolStats = $derived(mcp.toolStats);
    const toolSignals = $derived(mcp.toolSignals);
    const loading = $derived(mcp.catalogStatus === 'loading' || mcp.catalogStatus === 'idle');

    let tab = $state("playground");
    let kindFilter = $state<'all' | 'query' | 'action'>('all');
    let selectedTool = $state<Tool | null>(null);
    let toolResult = $state<string>("");
    let toolParams = $state<Record<string, string>>({});
    let executing = $state(false);

    // Replay tab state — session list stays local (small, page-scoped);
    // timelines cache in the mcp store keyed by session id. Calls carry the
    // #90 verdict / confidence / verdictReason on the new Replay endpoint
    // (#84 Slice C), so we widen the type to SessionReplayCall.
    let replaySessions = $state<SessionRow[]>([]);
    let selectedSessionId = $state<string | null>(null);
    let sessionCalls = $state<SessionReplayCall[]>([]);
    let sessionSummary = $state<{ used: number; partial: number; ignored: number; total: number }>({
        used: 0, partial: 0, ignored: 0, total: 0,
    });
    let replayLoading = $state(false);
    let selectedCall = $state<SessionReplayCall | null>(null);

    // Insights tab — the newer cached tool_insights table is loaded lazily
    // when the tab is first opened. Selecting a tool row expands its
    // metrics detail.
    let selectedInsightTool = $state<string | null>(null);
    const selectedInsight = $derived(
        selectedInsightTool ? mcp.insightFor(selectedInsightTool) : undefined,
    );

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

    onMount(() => {
        void mcp.loadCatalog();
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
        sessionSummary = { used: 0, partial: 0, ignored: 0, total: 0 };
        replayLoading = true;
        // #84 T2 Slice C — Replay endpoint with #90 verdicts joined. First
        // open triggers `classify: true` so verdicts populate in one round-
        // trip; subsequent selects hit the cache.
        const cache = await mcp.loadSessionReplay(sessionId, { classify: true });
        sessionCalls = cache.calls;
        sessionSummary = cache.summary;
        replayLoading = false;
    }

    // Kick off session loading the first time Replay is visible; and load
    // the tool_insights cache the first time Insights is visible.
    $effect(() => {
        if (tab === 'replay') {
            void ensureReplaySessionsLoaded();
        }
        if (tab === 'insights') {
            void mcp.loadInsights();
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
            <div class="flex gap-2 mb-4" role="tablist" aria-label="Tool kind filter" data-testid="kind-chips">
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
                        data-testid={`kind-chip-${chip.id}`}
                        onclick={() => (kindFilter = chip.id)}
                    >
                        {chip.label}
                    </button>
                {/each}
            </div>

            <div class="grid grid-cols-[260px_1fr] gap-6" data-testid="playground-body">
                <!-- Tool list -->
                <div class="flex flex-col gap-0.5" data-testid="tool-list">
                    {#each visibleTools as tool (tool.name)}
                        <button
                            class="tool-card text-left px-3.5 py-2.5 rounded-md bg-transparent border-none cursor-pointer transition-colors duration-fast"
                            class:selected={selectedTool?.name === tool.name}
                            data-testid={`tool-row-${tool.name}`}
                            data-tool-kind={tool.kind}
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
                    <div class="flex items-baseline justify-between px-3 py-2">
                        <span class="text-xs uppercase tracking-wide text-ink-mute">Calls</span>
                        {#if selectedSessionId && sessionSummary.total > 0}
                            <!-- #84 T2 Slice C — session verdict summary. -->
                            <span class="text-[10px] text-ink-soft">
                                <span class="text-success">{sessionSummary.used}</span> ·
                                <span class="text-warning">{sessionSummary.partial}</span> ·
                                <span class="text-ink-mute">{sessionSummary.ignored}</span>
                                <span class="text-ink-mute">/ {sessionSummary.total}</span>
                            </span>
                        {/if}
                    </div>
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
                                <!-- #84 T2 Slice C — #90 verdict badge. Colour by verdict, no
                                     badge when unclassified. -->
                                {#if call.verdict === 'used'}
                                    <span class="text-[10px] px-1.5 py-0.5 rounded bg-success-soft text-success uppercase tracking-wide" title={call.verdictReason ?? ''}>used</span>
                                {:else if call.verdict === 'partial'}
                                    <span class="text-[10px] px-1.5 py-0.5 rounded bg-warning-soft text-warning uppercase tracking-wide" title={call.verdictReason ?? ''}>partial</span>
                                {:else if call.verdict === 'ignored'}
                                    <span class="text-[10px] px-1.5 py-0.5 rounded bg-paper-soft text-ink-mute uppercase tracking-wide" title={call.verdictReason ?? ''}>ignored</span>
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
        <!-- Signal cards on top — sorted by variant priority in the daemon. -->
        {#if toolSignals.length > 0}
            <div class="mb-6" data-testid="tool-signals">
                <p class="text-xs uppercase tracking-wide text-ink-mute mb-2">Signals</p>
                <div class="grid gap-2 grid-cols-1 md:grid-cols-2">
                    {#each toolSignals as sig (sig.tool_name + sig.variant)}
                        <div data-testid={`signal-card-${sig.variant}`} data-tool={sig.tool_name}>
                            <SignalCard
                                variant={sig.variant}
                                title={sig.title}
                                detail={sig.detail}
                                toolName={sig.tool_name}
                            />
                        </div>
                    {/each}
                </div>
            </div>
        {/if}

        <div class="flex flex-col gap-1" data-testid="insights-table">
            <div class="grid grid-cols-[1fr_80px_80px_100px_120px] gap-3 px-3 py-2 text-xs text-ink-soft tracking-wide uppercase">
                <span>Tool</span>
                <span class="text-right">Calls</span>
                <span class="text-right">Errors</span>
                <span class="text-right">Avg ms</span>
                <span class="text-right">Last used</span>
            </div>
            {#each toolStats as stat (stat.tool_name)}
                {@const errorRate = stat.call_count > 0 ? stat.error_count / stat.call_count : 0}
                {@const expanded = selectedInsightTool === stat.tool_name}
                <button
                    type="button"
                    class="w-full text-left grid grid-cols-[1fr_80px_80px_100px_120px] gap-3 px-3 py-2.5 border-b border-paper-mute text-sm items-center bg-transparent border-l-0 border-r-0 border-t-0 cursor-pointer"
                    class:bg-paper-mute={expanded}
                    data-testid={`insights-row-${stat.tool_name}`}
                    aria-expanded={expanded}
                    onclick={() => (selectedInsightTool = expanded ? null : stat.tool_name)}
                >
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
                </button>
                {#if expanded}
                    {@const insight = selectedInsight}
                    <div class="px-3 pb-3 pt-1 border-b border-paper-mute bg-paper-soft" data-testid={`insights-detail-${stat.tool_name}`}>
                        {#if insight}
                            <div class="flex items-center gap-3 mb-2">
                                <span class="text-xs uppercase tracking-wide text-ink-mute">Snapshot</span>
                                <span class="text-xs text-ink-soft font-mono">
                                    computed {new Date(insight.computedAt).toLocaleString()}
                                </span>
                                {#if insight.variant}
                                    <span class="text-xs uppercase tracking-wide"
                                        class:text-danger={insight.variant === 'warn'}
                                        class:text-warning={insight.variant === 'opportunity'}
                                        class:text-ink-mute={insight.variant === 'unused'}
                                        class:text-success={insight.variant === 'win'}>
                                        {insight.variant}
                                    </span>
                                {/if}
                            </div>
                            <div class="grid grid-cols-2 gap-x-6 gap-y-1 text-xs">
                                <span class="text-ink-mute">Error rate</span>
                                <span class="font-mono">
                                    {insight.metrics.errorRate != null ? `${(insight.metrics.errorRate * 100).toFixed(1)}%` : '—'}
                                </span>
                                <span class="text-ink-mute">Avg duration</span>
                                <span class="font-mono">
                                    {insight.metrics.avgDurationMs != null ? fmtDuration(insight.metrics.avgDurationMs) : '—'}
                                </span>
                                <span class="text-ink-mute">Calls</span>
                                <span class="font-mono">{insight.metrics.callCount ?? 0}</span>
                                <span class="text-ink-mute">Errors</span>
                                <span class="font-mono">{insight.metrics.errorCount ?? 0}</span>
                            </div>

                            {#if (insight.metrics.verdictTotal ?? 0) > 0}
                                <!-- #84 T2 Slice D — 14d verdict split from
                                     aggregate_tool_insights. Only render when
                                     the window has at least one classified
                                     verdict; unused tools would otherwise
                                     show a zero-bar. -->
                                <div class="mt-3 pt-3 border-t border-paper-mute">
                                    <div class="flex items-baseline justify-between mb-1.5">
                                        <span class="text-xs uppercase tracking-wide text-ink-mute">
                                            Usage split · {insight.metrics.verdictWindowDays ?? 14}d
                                        </span>
                                        <span class="text-xs text-ink-soft font-mono">
                                            {insight.metrics.verdictTotal} classified
                                        </span>
                                    </div>
                                    <!-- Segmented bar: used | partial | ignored. Percentages come
                                         from the aggregator so client stays a pure renderer. -->
                                    <div class="flex h-2 rounded overflow-hidden bg-paper-mute" title={
                                        `used ${((insight.metrics.usedPct ?? 0) * 100).toFixed(0)}% · partial ${((insight.metrics.partialPct ?? 0) * 100).toFixed(0)}% · ignored ${((insight.metrics.ignoredPct ?? 0) * 100).toFixed(0)}%`
                                    }>
                                        {#if (insight.metrics.usedPct ?? 0) > 0}
                                            <div class="bg-success" style="width: {((insight.metrics.usedPct ?? 0) * 100).toFixed(1)}%"></div>
                                        {/if}
                                        {#if (insight.metrics.partialPct ?? 0) > 0}
                                            <div class="bg-warning" style="width: {((insight.metrics.partialPct ?? 0) * 100).toFixed(1)}%"></div>
                                        {/if}
                                        {#if (insight.metrics.ignoredPct ?? 0) > 0}
                                            <div class="bg-ink-mute" style="width: {((insight.metrics.ignoredPct ?? 0) * 100).toFixed(1)}%"></div>
                                        {/if}
                                    </div>
                                    <div class="flex gap-3 mt-1.5 text-[10px]">
                                        <span><span class="inline-block w-2 h-2 rounded-sm bg-success align-middle mr-1"></span>used {insight.metrics.usedCount ?? 0}</span>
                                        <span><span class="inline-block w-2 h-2 rounded-sm bg-warning align-middle mr-1"></span>partial {insight.metrics.partialCount ?? 0}</span>
                                        <span><span class="inline-block w-2 h-2 rounded-sm bg-ink-mute align-middle mr-1"></span>ignored {insight.metrics.ignoredCount ?? 0}</span>
                                    </div>
                                </div>
                            {/if}
                            {#if insight.title && insight.detail}
                                <div class="mt-2 text-xs text-ink-soft leading-normal">
                                    <span class="font-medium">{insight.title}.</span> {insight.detail}
                                </div>
                            {/if}
                        {:else}
                            <p class="text-xs text-ink-soft m-0">
                                No cached snapshot yet — the AggregateToolInsights task
                                writes one per scheduler tick.
                            </p>
                        {/if}
                    </div>
                {/if}
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
