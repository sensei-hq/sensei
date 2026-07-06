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
    // `selectedTool` is $derived from `selectedToolId` + the sensei catalog
    // below; kept reactive so a catalog reload doesn't strand a stale ref.
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

    // #84 T2 Slice A/B — discovered MCP servers state.
    // Sensei's own tools + every discovered server merge into a single
    // "groups" list in the sidebar left rail per instruments-simple.jsx.
    // Each group is expandable; a discovered server's expand triggers
    // loadServerTools() to hydrate the tool manifest lazily.
    const discoveredServers = $derived(mcp.mcpServers);

    // Group tools by MCP for the sidebar. Sensei is always first and open by
    // default; discovered servers default to collapsed. Search auto-expands
    // any group that has hits so the results aren't hidden behind a
    // chevron.
    type PlaygroundTool = {
        id: string;
        mcp: string;
        name: string;
        kind: 'query' | 'action' | 'other';
        summary: string;
    };
    type PlaygroundGroup = {
        id: string;
        kanji: string;
        name: string;
        installed: boolean;
        source: 'sensei' | 'discovered';
        serverId?: string;
        tools: PlaygroundTool[];
    };

    let collapsedGroups = $state<Record<string, boolean>>({});
    let toolSearch = $state('');
    let refreshingServers = $state(false);

    function inferKind(name: string, declared: string | undefined): 'query' | 'action' | 'other' {
        if (declared === 'action' || declared === 'query') return declared;
        // Fallback classifier for tool names from third-party MCPs whose
        // catalog doesn't declare a kind. Verbs that mutate → action;
        // getters / list / search → query.
        const n = name.toLowerCase();
        if (/^(get|list|search|find|read|show|describe|inspect|explain|status|check)/.test(n)) return 'query';
        if (/^(set|create|delete|update|write|run|call|start|stop|restart|install|remove)/.test(n)) return 'action';
        return 'other';
    }

    const playgroundGroups = $derived.by<PlaygroundGroup[]>(() => {
        const senseiTools: PlaygroundTool[] = tools.map((t) => ({
            id: `sensei:${t.name}`,
            mcp: 'sensei',
            name: t.name,
            kind: t.kind === 'action' || t.kind === 'query' ? t.kind : 'other',
            summary: t.summary,
        }));
        const groups: PlaygroundGroup[] = [
            { id: 'sensei', kanji: '具', name: 'Sensei', installed: true, source: 'sensei', tools: senseiTools },
        ];
        for (const server of discoveredServers) {
            const manifest = mcp.serverToolManifests[server.id];
            const rawTools = (manifest?.tools ?? []) as Array<{ name?: string; description?: string }>;
            const serverTools: PlaygroundTool[] = rawTools
                .filter((t) => !!t.name)
                .map((t) => ({
                    id: `${server.id}:${t.name}`,
                    mcp: server.id,
                    name: t.name as string,
                    kind: inferKind(t.name as string, undefined),
                    summary: t.description ?? '',
                }));
            groups.push({
                id: server.id,
                // Small kanji per family — reasonable defaults; real mockup uses per-MCP glyphs.
                kanji: server.acp_family === 'claude' ? '書' : server.acp_family === 'zed' ? '禅' : server.acp_family === 'cursor' ? '刀' : '器',
                name: server.mcp_key,
                installed: server.enabled,
                source: 'discovered',
                serverId: server.id,
                tools: serverTools,
            });
        }
        return groups;
    });

    // Search-filtered view of groups. Ql matches on tool name OR summary.
    const filteredGroups = $derived.by(() => {
        const ql = toolSearch.trim().toLowerCase();
        if (!ql) return playgroundGroups;
        return playgroundGroups
            .map((g) => ({
                ...g,
                tools: g.tools.filter((t) =>
                    t.name.toLowerCase().includes(ql) || t.summary.toLowerCase().includes(ql)
                ),
            }))
            .filter((g) => g.tools.length > 0);
    });

    // Effective collapse state: search auto-expands any group with hits.
    const effectiveCollapsed = $derived.by(() => {
        const ql = toolSearch.trim().toLowerCase();
        if (ql) {
            return Object.fromEntries(filteredGroups.map((g) => [g.id, false]));
        }
        return collapsedGroups;
    });

    // Flat list of visible tools for focus-pick fallback.
    const visibleFlat = $derived(filteredGroups.flatMap((g) => g.tools));

    let selectedToolId = $state<string | null>(null);
    const focusedGroupAndTool = $derived.by(() => {
        const byId = new Map(playgroundGroups.flatMap((g) => g.tools.map((t) => [t.id, { group: g, tool: t }])));
        if (selectedToolId && byId.has(selectedToolId)) {
            return byId.get(selectedToolId)!;
        }
        const first = visibleFlat[0];
        if (first && byId.has(first.id)) return byId.get(first.id)!;
        return null;
    });
    const focusedGroup = $derived(focusedGroupAndTool?.group ?? null);
    const focusedTool = $derived(focusedGroupAndTool?.tool ?? null);

    // Bridge to legacy state used by executeTool. `selectedTool` mirrors
    // the sensei-manifest Tool shape (structured inputs + example) when
    // focus lands on a sensei tool; for discovered-server tools we don't
    // have the manifest yet, so the inputs block renders as "No inputs".
    const selectedTool = $derived(
        focusedTool?.mcp === 'sensei' ? tools.find((t) => t.name === focusedTool.name) ?? null : null,
    );

    function toggleGroup(id: string) {
        // Discovered server: trigger lazy tool load on first expand.
        const isNowExpanded = collapsedGroups[id];
        collapsedGroups = { ...collapsedGroups, [id]: !collapsedGroups[id] };
        const group = playgroundGroups.find((g) => g.id === id);
        if (isNowExpanded && group?.source === 'discovered' && group.serverId && !mcp.serverToolManifests[group.serverId]) {
            void mcp.loadServerTools(group.serverId);
        }
    }

    async function refreshDiscoveredServers() {
        refreshingServers = true;
        try {
            await mcp.refreshMcpServers();
        } finally {
            refreshingServers = false;
        }
    }

    // Default: sensei expanded, all discovered servers collapsed.
    $effect(() => {
        for (const g of playgroundGroups) {
            if (!(g.id in collapsedGroups)) {
                collapsedGroups = { ...collapsedGroups, [g.id]: g.source !== 'sensei' };
            }
        }
    });

    // Custom tab set matching the mockup (docs/mockups/Sensei/lib/instruments.jsx).
    // Each tab has a kanji glyph + label + one-line hint. The third tab is
    // named "Health" (renamed from Insights per the mockup) — it's a
    // toolset-health surface, not the Observatory's top-level Insights view.
    // 'insights' is kept as the historical id so bookmarks / e2e testids
    // don't churn; the visible label is "Health".
    const instrumentTabs: Array<{ id: string; kanji: string; label: string; hint: string }> = [
        { id: 'playground', kanji: '具', label: 'Playground', hint: 'what can these instruments do?' },
        { id: 'replay',     kanji: '録', label: 'Replay',     hint: 'what did the assistant do?' },
        { id: 'insights',   kanji: '健', label: 'Health',     hint: 'what should we change?' },
    ];

    // Kind chips carry a kanji glyph per the mockup (全 all, 作 actions, 問 queries).
    const kindChips: Array<{ id: 'all' | 'query' | 'action'; label: string; kanji: string }> = [
        { id: 'all',    label: 'All',     kanji: '全' },
        { id: 'action', label: 'Actions', kanji: '作' },
        { id: 'query',  label: 'Queries', kanji: '問' },
    ];

    // Playground search — narrows the visible tool list on top of the kind
    // filter. Applied client-side so filter changes are instant.
    let toolQuery = $state('');

    const currentTab = $derived(instrumentTabs.find((t) => t.id === tab) ?? instrumentTabs[0]);
    // Per-tab hero copy (tagline + sub). Matches the phrasing in
    // docs/mockups/Sensei/lib/instruments.jsx.
    const heroCopy = $derived(
        tab === 'replay'   ? { tagline: 'Every instrument call, in order.',
                                sub: "Step through the tools the assistant reached for during a session. Pure request + response — what was asked, what came back, how long it took." } :
        tab === 'insights' ? { tagline: 'Toolset health.',
                                sub: "How your instruments are performing — usage, effectiveness signals, and where the assistant is under- or over-reaching." } :
        { tagline: "Sensei · what your instruments can do.",
          sub: "Sensei's own MCP + every server discovered from your assistant configs. Try each tool interactively; sensei never wraps third-party MCPs — it just surfaces them." }
    );

    // Filter tools by the active kind chip AND the search query. Both are
    // applied client-side so switches are instant with no daemon round-trip.
    const visibleTools = $derived.by(() => {
        const q = toolQuery.trim().toLowerCase();
        return tools.filter((t) =>
            (kindFilter === 'all' || t.kind === kindFilter) &&
            (q === '' || t.name.toLowerCase().includes(q) || (t.summary ?? '').toLowerCase().includes(q))
        );
    });

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
    // the tool_insights cache the first time Insights is visible. Playground
    // gets a lazy load of the discovered MCP servers panel.
    $effect(() => {
        if (tab === 'playground') {
            void mcp.loadMcpServers();
        }
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

<!-- #84 T2 UI — Instruments custom shell matching
     docs/mockups/Sensei/lib/instruments.jsx. Hero header (kanji + eyebrow +
     tagline + sub) sits above a kanji-decorated tab nav; the three tabs
     read as one-line questions (Playground: what can these instruments do?
     · Replay: what did the assistant do? · Health: what should we change?)
     matching the mockup's phrasing.
     Full hero — kanji + eyebrow (Instruments · <tab>) + tagline + sub -->
<div class="flex items-end gap-4 pt-5 pb-4 px-7 border-b border-paper-mute" data-testid="instruments-hero">
    <div class="kanji text-4xl text-accent leading-none">{currentTab.kanji}</div>
    <div class="flex-1 min-w-0">
        <div class="text-xs uppercase tracking-[0.18em] text-ink-mute mb-1">
            Instruments · {tab}
        </div>
        <h1 class="display text-xl font-normal m-0 text-ink">{heroCopy.tagline}</h1>
        <p class="text-sm text-ink-soft leading-normal mt-1 mb-0 max-w-[680px]">
            {heroCopy.sub}
        </p>
    </div>
</div>

<!-- Custom kanji-tabs strip. Renamed 'insights' label → 'Health' per mockup;
     the id stays for URL/testid stability. -->
<div class="flex px-7 border-b border-paper-mute bg-paper" role="tablist" aria-label="Instruments tabs" data-testid="instrument-tabs">
    {#each instrumentTabs as t}
        {@const on = t.id === tab}
        <button
            class="flex items-center gap-2 py-3 px-4 bg-transparent border-none cursor-pointer"
            class:text-ink={on}
            class:text-ink-soft={!on}
            style="border-bottom: {on ? '2px solid var(--ink)' : '2px solid transparent'}; margin-bottom: -1px;"
            role="tab"
            aria-selected={on}
            data-testid={`instrument-tab-${t.id}`}
            onclick={() => (tab = t.id)}
        >
            <span class="kanji text-sm" class:text-accent={on} class:text-ink-mute={!on}>{t.kanji}</span>
            <span class="display text-sm">{t.label}</span>
            <span class="text-xs text-ink-mute">· {t.hint}</span>
        </button>
    {/each}
</div>

<div class="max-w-[960px] mx-auto px-12 pt-8 pb-16">

    {#if tab === "playground"}
        <!-- #84 T2 Playground — matches docs/mockups/Sensei/lib/instruments-simple.jsx
             (InstrumentsPlaygroundSimple). Left rail: search + collapsible MCP
             groups → tools. Right: tool detail with inputs form + response
             preview. Sensei's own tools sit as the first (always-installed)
             group; every discovered MCP server (Slice A) is its own group
             with tools hydrated lazily on expand via Slice B's probe. -->
        {@const running = executing}
        <div class="grid grid-cols-[300px_1fr] min-h-[600px] border-t border-paper-mute" data-testid="playground-body">
            <!-- ── Left rail — search + MCP tree ────────────────────────────── -->
            <aside class="bg-paper-soft border-r border-paper-mute flex flex-col overflow-hidden">
                <!-- Search box -->
                <div class="flex items-center gap-2 py-3 px-3 border-b border-paper-mute">
                    <span class="kanji text-xs text-ink-mute">探</span>
                    <input
                        type="text"
                        class="border-none outline-none bg-transparent text-sm text-ink flex-1"
                        placeholder="search tools…"
                        bind:value={toolSearch}
                        data-testid="tool-search"
                    />
                    {#if toolSearch}
                        <button class="text-xs text-ink-faint bg-transparent border-none cursor-pointer" onclick={() => (toolSearch = '')} aria-label="Clear search">×</button>
                    {/if}
                </div>

                <!-- MCP groups list -->
                <div class="overflow-auto flex-1 py-1">
                    {#if filteredGroups.length === 0}
                        <div class="text-center text-sm text-ink-faint py-4 px-3">
                            no tools match.
                        </div>
                    {/if}
                    {#each filteredGroups as group (group.id)}
                        {@const collapsed = effectiveCollapsed[group.id] ?? (group.source !== 'sensei')}
                        <div class="mb-1" data-testid={`mcp-group-${group.id}`}>
                            <!-- Group header -->
                            <button
                                class="w-full grid grid-cols-[14px_18px_1fr_auto_auto] items-center gap-2 py-2 px-3 text-left bg-transparent border-none cursor-pointer text-ink-soft"
                                onclick={() => toggleGroup(group.id)}
                                aria-expanded={!collapsed}
                                data-testid={`mcp-group-toggle-${group.id}`}
                            >
                                <span
                                    class="font-mono text-xs text-ink-mute transition-transform duration-fast"
                                    style="transform: {collapsed ? 'none' : 'rotate(90deg)'};"
                                >▶</span>
                                <span class="kanji text-sm text-accent">{group.kanji}</span>
                                <span class="text-sm truncate">{group.name}</span>
                                {#if !group.installed}
                                    <span class="text-xs uppercase tracking-wide text-warning">off</span>
                                {:else}
                                    <span></span>
                                {/if}
                                <span class="font-mono text-xs text-ink-faint">{group.tools.length}</span>
                            </button>

                            <!-- Tool lines -->
                            {#if !collapsed}
                                <div>
                                    {#each group.tools as tool (tool.id)}
                                        {@const active = focusedTool?.id === tool.id}
                                        {@const isAction = tool.kind === 'action'}
                                        {@const kindGlyph = isAction ? '作' : tool.kind === 'query' ? '問' : '·'}
                                        <button
                                            class="w-full grid grid-cols-[32px_14px_1fr] gap-1 py-1 pl-1 pr-3 text-left bg-transparent border-none cursor-pointer"
                                            style="border-left: {active ? '2px solid var(--accent)' : '2px solid transparent'};"
                                            onclick={() => { selectedToolId = tool.id; toolResult = ''; toolParams = {}; }}
                                            data-testid={`tool-row-${tool.name}`}
                                            data-tool-kind={tool.kind}
                                        >
                                            <span></span>
                                            <span
                                                class="kanji text-xs"
                                                class:text-accent={isAction}
                                                class:text-success={tool.kind === 'query'}
                                                class:text-ink-mute={tool.kind === 'other'}
                                            >{kindGlyph}</span>
                                            <span
                                                class="font-mono text-xs truncate"
                                                class:text-ink={active}
                                                class:text-ink-soft={!active}
                                            >{tool.name}</span>
                                        </button>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                    {/each}
                </div>

                <!-- Footer — MCP + tool counts + refresh -->
                <div class="border-t border-paper-mute flex justify-between items-center gap-2 py-2 px-3 text-xs text-ink-faint">
                    <span>{playgroundGroups.length} MCPs · {playgroundGroups.reduce((s, g) => s + g.tools.length, 0)} tools</span>
                    <button
                        class="text-xs text-ink-mute bg-transparent border-none cursor-pointer"
                        onclick={refreshDiscoveredServers}
                        disabled={refreshingServers}
                        data-testid="refresh-servers"
                    >{refreshingServers ? 'scanning…' : '+ scan'}</button>
                </div>
            </aside>

            <!-- ── Detail ────────────────────────────────────────────────────── -->
            <main class="overflow-auto pt-5 pb-6 px-7">
                {#if focusedTool && focusedGroup}
                    {@const tool = focusedTool}
                    {@const group = focusedGroup}
                    {@const isAction = tool.kind === 'action'}
                    {@const kindColor = isAction ? 'accent' : tool.kind === 'query' ? 'success' : 'ink-mute'}
                    {@const kindLabel = isAction ? 'action' : tool.kind === 'query' ? 'query' : 'call'}
                    {@const kindGlyph = isAction ? '作' : tool.kind === 'query' ? '問' : '·'}
                    {@const kindHint = isAction ? 'performs an operation' : 'returns information'}

                    <!-- Heading -->
                    <div class="mb-4">
                        <div class="flex items-center flex-wrap gap-2 mb-2">
                            <span class="font-mono text-xs text-ink-mute">{group.kanji} {group.name.toLowerCase()}</span>
                            <span class="text-xs text-ink-faint">·</span>
                            <span
                                class="inline-flex items-center gap-1 py-1 px-2 text-xs bg-paper-soft border border-paper-edge rounded uppercase tracking-wide"
                                class:text-accent={isAction}
                                class:text-success={tool.kind === 'query'}
                                class:text-ink-mute={tool.kind === 'other'}
                            >
                                <span class="kanji text-xs">{kindGlyph}</span>
                                {kindLabel}
                            </span>
                        </div>
                        <h2 class="font-mono text-lg font-normal m-0 text-ink">{tool.name}</h2>
                        <p class="text-sm text-ink-soft leading-normal mt-1 mb-0 max-w-[700px]">
                            {tool.summary || 'No description.'}
                        </p>
                    </div>

                    <!-- Inputs panel -->
                    <div class="bg-paper-soft border border-paper-mute rounded-lg py-3 px-4 mb-3">
                        <div class="flex items-baseline justify-between mb-2">
                            <span class="text-xs uppercase tracking-wide text-ink-mute">Inputs</span>
                            <span class="text-xs text-ink-faint">{kindHint}</span>
                        </div>

                        {#if !selectedTool || (selectedTool.inputs?.length ?? 0) === 0}
                            <div class="text-sm text-ink-mute mb-2">
                                {selectedTool ? 'No inputs — just call it.' : 'Third-party MCP — sensei surfaces the tool but doesn\'t know its input schema yet.'}
                            </div>
                        {:else}
                            <div class="grid grid-cols-2 gap-y-2 gap-x-4">
                                {#each selectedTool.inputs as input}
                                    <label class="flex flex-col text-xs text-ink-soft">
                                        <span class="flex items-baseline gap-1">
                                            <span>{input.label}</span>
                                            {#if input.required}<span class="text-accent">*</span>{/if}
                                            <span class="font-mono text-xs text-ink-faint">{input.kind}</span>
                                        </span>
                                        {#if input.kind === 'enum' && input.options}
                                            <select
                                                class="py-1 px-2 text-sm border border-paper-edge rounded bg-paper text-ink font-mono outline-none"
                                                bind:value={toolParams[input.key]}
                                            >
                                                {#each input.options as option}
                                                    <option value={option}>{option}</option>
                                                {/each}
                                            </select>
                                        {:else}
                                            <input
                                                type={input.kind === 'number' ? 'number' : 'text'}
                                                class="py-1 px-2 text-sm border border-paper-edge rounded bg-paper text-ink font-mono outline-none"
                                                placeholder={input.placeholder ?? input.default ?? ''}
                                                bind:value={toolParams[input.key]}
                                            />
                                        {/if}
                                    </label>
                                {/each}
                            </div>
                        {/if}

                        <div class="flex items-center gap-2 mt-3 pt-2 border-t border-paper-mute">
                            <button
                                class="py-1 px-3 text-sm bg-ink text-paper border-none rounded cursor-pointer tracking-tight"
                                onclick={executeTool}
                                disabled={running || !selectedTool}
                            >
                                {isAction ? 'Run →' : 'Query →'}
                            </button>
                            <div class="flex-1"></div>
                            {#if running}
                                <span class="font-mono text-xs text-ink-mute">calling …</span>
                            {:else if toolResult}
                                <span class="font-mono text-xs text-success">200 ok</span>
                            {/if}
                        </div>
                    </div>

                    <!-- Response panel -->
                    <div>
                        <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">
                            Response{!toolResult ? ' · preview' : ''}
                        </div>
                        <pre
                            class="font-mono text-xs leading-normal py-3 px-3 m-0 bg-paper-soft border border-paper-mute rounded whitespace-pre-wrap overflow-auto max-h-[360px]"
                            class:text-ink={toolResult}
                            class:text-ink-soft={!toolResult}
                            class:opacity-70={!toolResult}
                            class:border-l-accent={isAction}
                            class:border-l-success={tool.kind === 'query'}
                            style="border-left-width: 2px; border-left-color: {isAction ? 'var(--accent)' : tool.kind === 'query' ? 'var(--success)' : 'var(--ink-mute)'};"
                        >{toolResult || '—'}</pre>
                    </div>
                {:else}
                    <div class="text-center text-sm text-ink-faint py-6">
                        Pick a tool to inspect it.
                    </div>
                {/if}
            </main>
        </div>
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
