<script lang="ts">
    import { onMount, untrack } from "svelte";
    import { page } from "$app/state";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import TabBar from "$lib/components/TabBar.svelte";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import SignalCard from "$lib/components/SignalCard.svelte";
    import { Eyebrow, PageHeader } from "$lib/components";
    import { Button, Select } from "@rokkit/ui";
    import { mcp, toolVerdict } from "$lib/state/mcp.svelte.js";
    import ToolHealthCard from "./ToolHealthCard.svelte";
    import type {
        McpToolManifest, SessionToolCall, SessionReplayCall, ToolHealthSource,
    } from "$lib/types.js";

    type Tool = McpToolManifest;
    type SessionRow = { id: string; task: string; startedAt: string; ftr?: number | null };

    // All catalogue data lives on the mcp store — page is pure presentation.
    const tools = $derived(mcp.tools);
    const toolStats = $derived(mcp.toolStats);
    const toolSignals = $derived(mcp.toolSignals);
    const loading = $derived(mcp.catalogStatus === 'loading' || mcp.catalogStatus === 'idle');

    // Health (insights) tab — L1 source grid + derived header KPIs. The grid
    // rows come connected-first from the store; the KPIs derive from the same
    // rows so the header never drifts from the cards below it.
    const toolsHealth = $derived(mcp.toolsHealth);
    const healthKpis = $derived(mcp.toolsHealthKpis);
    // Holistic 14-day first-try rate for the L1 KPI header (一 first-try).
    // Sourced from GET /api/observatory/ftr, page-local like the replay list.
    // null until the fetch resolves so the card shows "—" rather than a
    // fabricated 0%.
    let holisticFtr = $state<number | null>(null);
    // null = L1 overview grid; a source = L2 per-tool drill for that card.
    let activeSource = $state<ToolHealthSource | null>(null);
    function openSource(source: ToolHealthSource) {
        activeSource = source;
    }

    // Honor a deep-link from the Sessions digest (Slot 6):
    // /instruments?tab=replay&session={id}. The session is preselected once
    // the replay list loads (see the effect below). Session-id resolves
    // through GET /api/sessions/{id} inside loadSessionReplay.
    const tabParam = untrack(() => page.url.searchParams.get("tab"));
    const sessionParam = untrack(() => page.url.searchParams.get("session"));
    let tab = $state(
        tabParam === "replay" || tabParam === "insights" ? tabParam : "playground",
    );
    let honoredSessionParam = false;
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

    /**
     * Pre-fill common tool-input keys so the user isn't forced to type
     * the active project name on every tool click. `repoId` defaults to
     * the currently-active project. Anything else stays empty and shows
     * placeholder from the manifest.
     *
     * Fixes the "execute returns empty result" symptom: sensei tools
     * that take a `repoId` return `{"callees":[]}` etc. when the input
     * doesn't match a real project name. Curl verification: valid
     * repoId=sensei with name=extract_deps → 18 callees; wrong project
     * name → empty. Defaulting to the active project makes the
     * happy-path work without user interaction.
     */
    function pickToolDefaults(tool: PlaygroundTool): Record<string, string> {
        const source = tool.mcp === 'sensei'
            ? tools.find((t) => t.name === tool.name)
            : null;
        const params: Record<string, string> = {};
        if (!source) return params;
        for (const input of source.inputs ?? []) {
            if (input.key === 'repoId') {
                // Prefer the user's active project; otherwise fall back to
                // the input's placeholder (usually 'sensei') so the field
                // isn't blank on first click. Empty repoId → empty result.
                params[input.key] = appState.activeProjectId
                    || input.placeholder
                    || input.default
                    || 'sensei';
            } else if (input.default != null) {
                params[input.key] = String(input.default);
            } else if (input.placeholder) {
                // For sensei's own tools the placeholder is a concrete
                // known-good example (e.g. 'process_event' for a symbol
                // arg). Pre-filling makes execute-on-first-click work
                // without the user having to guess.
                params[input.key] = input.placeholder;
            }
        }
        return params;
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

    // Load the holistic 14-day FTR once, lazily, for the Health L1 header.
    // Page-local (mirrors ensureReplaySessionsLoaded) — the mcp store owns
    // tool data, this is observatory-wide.
    async function ensureHolisticFtrLoaded() {
        if (holisticFtr != null) return;
        const api = senseiApi(appState.port);
        const resp = await api.getObservatoryFtr();
        holisticFtr = resp.ftr14d;
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
            // L1 grid + L2 drill data. loadCatalog (onMount) already primed
            // toolStats/toolSignals for the L2 pane.
            void mcp.loadToolsHealth();
            void mcp.loadInsights();
            void ensureHolisticFtrLoaded();
        }
    });

    // Preselect the deep-linked session once the replay list is available.
    // Runs at most once — re-fires only while waiting for the list to load.
    $effect(() => {
        if (honoredSessionParam || !sessionParam || tab !== 'replay') return;
        if (replaySessions.length === 0) {
            void ensureReplaySessionsLoaded();
            return;
        }
        honoredSessionParam = true;
        void selectReplaySession(sessionParam);
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
<!-- Sticky hero + tabs. Full width — no max-w wrapper — so the panels
     below fill the observatory main pane edge-to-edge (fixes the L/R
     gap the user hit). Sticky top-0 keeps them visible while the body
     scrolls. -->
<div class="sticky top-0 z-20 bg-paper">
    <div class="flex items-end gap-4 pt-5 pb-4 px-7 border-b border-paper-mute" data-testid="instruments-hero">
        <div class="kanji text-4xl text-accent leading-none">{currentTab.kanji}</div>
        <div class="flex-1 min-w-0">
            <div class="text-xs uppercase tracking-[0.18em] text-ink-mute mb-1">
                Instruments · {currentTab.label}
            </div>
            <h1 class="display text-xl font-normal m-0 text-ink">{heroCopy.tagline}</h1>
            <p class="text-sm text-ink-soft leading-normal mt-1 mb-0 max-w-[680px]">
                {heroCopy.sub}
            </p>
        </div>
    </div>
    <div class="flex px-7 border-b border-paper-mute" role="tablist" aria-label="Instruments tabs" data-testid="instrument-tabs">
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
            </button>
        {/each}
    </div>
</div>

<!-- Body is full width; Playground/Replay use their own internal grids,
     Health gets horizontal padding so cards breathe but still reach the
     edges (no centered max-w wrapper any more). -->
<div>

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
                                            onclick={() => { selectedToolId = tool.id; toolResult = ''; toolParams = pickToolDefaults(tool); }}
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
                                            <Select
                                                items={input.options}
                                                value={toolParams[input.key]}
                                                onchange={(v) => (toolParams[input.key] = v as string)}
                                            />
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
                            <Button
                                variant="primary"
                                size="sm"
                                onclick={executeTool}
                                disabled={running || !selectedTool}
                                data-testid="tool-execute"
                            >
                                {isAction ? 'Run →' : 'Query →'}
                            </Button>
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
        <!-- Replay body — matches instruments.jsx InstrumentsReplay (used
             by InstrumentsReplaySimple with simple=true). 300px session
             picker sidebar + main pane with summary strip + timeline+
             detail grid. Timeline entries use the mockup's [22px | 38px |
             1fr | auto] grid (index / offset / tool name / dot+ms). -->
        {#if replayLoading && replaySessions.length === 0}
            <p class="text-sm text-ink-soft px-7 py-5">Loading sessions…</p>
        {:else if replaySessions.length === 0}
            <EmptyState
                kanji="録"
                title="No sessions recorded yet"
                description="Tool calls from your assistant sessions appear here once sensei has captured at least one session."
            />
        {:else}
            <div class="grid grid-cols-[300px_1fr] min-h-[600px] border-t border-paper-mute">
                <!-- ── Left rail — session picker ─────────────────── -->
                <aside class="bg-paper-soft border-r border-paper-mute overflow-auto">
                    <div class="text-xs uppercase tracking-wide text-ink-mute pt-3 pb-2 px-3">
                        sessions
                    </div>
                    {#each replaySessions as session (session.id)}
                        {@const on = selectedSessionId === session.id}
                        <button
                            class="w-full text-left bg-transparent border-none cursor-pointer py-2 px-3"
                            class:bg-paper={on}
                            style="border-left: {on ? '2px solid var(--accent)' : '2px solid transparent'};"
                            onclick={() => selectReplaySession(session.id)}
                            data-testid={`replay-session-${session.id}`}
                        >
                            <div class="flex items-baseline gap-1">
                                <span class="font-mono text-xs" class:text-ink={on} class:text-ink-soft={!on}>
                                    {session.id.substring(0, 8)}
                                </span>
                                {#if session.ftr != null}
                                    {@const isFtr = session.ftr >= 1}
                                    <span
                                        class="text-xs uppercase tracking-wide"
                                        class:text-success={isFtr}
                                        class:text-warning={!isFtr}
                                    >
                                        {isFtr ? 'ftr' : `${((1 - (session.ftr ?? 0)) * 100).toFixed(0)}%c`}
                                    </span>
                                {/if}
                            </div>
                            <div class="text-xs text-ink-soft leading-normal mt-1 line-clamp-2">
                                {session.task}
                            </div>
                            <div class="text-xs text-ink-faint mt-1">
                                {fmtDate(session.startedAt)}
                            </div>
                        </button>
                    {/each}
                </aside>

                <!-- ── Detail ─────────────────────────────────────── -->
                <main class="overflow-auto pt-5 pb-6 px-6">
                    {#if selectedSessionId == null}
                        <p class="text-sm text-ink-soft">Pick a session on the left.</p>
                    {:else if replayLoading}
                        <p class="text-sm text-ink-soft">Loading timeline…</p>
                    {:else}
                        <!-- Session summary strip -->
                        <div class="grid grid-cols-[auto_auto_auto_auto_1fr] items-baseline gap-5 mb-4 pb-3 border-b border-paper-mute">
                            <div>
                                <div class="display text-base mb-1">
                                    {replaySessions.find((s) => s.id === selectedSessionId)?.task ?? selectedSessionId}
                                </div>
                                <div class="font-mono text-xs text-ink-mute">{selectedSessionId}</div>
                            </div>
                            <div>
                                <div class="text-xs uppercase tracking-wide text-ink-faint">calls</div>
                                <div class="display text-base mt-1">{sessionCalls.length}</div>
                            </div>
                            <div>
                                <div class="text-xs uppercase tracking-wide text-ink-faint">used</div>
                                <div class="display text-base text-success mt-1">{sessionSummary.used}</div>
                            </div>
                            <div>
                                <div class="text-xs uppercase tracking-wide text-ink-faint">ignored</div>
                                <div class="display text-base text-ink-mute mt-1">{sessionSummary.ignored}</div>
                            </div>
                            <span></span>
                        </div>

                        <!-- Timeline + detail grid — matches mockup's 0.95fr 1.4fr split -->
                        <div class="grid grid-cols-[0.95fr_1.4fr] gap-5" style="min-height: 400px;">
                            <!-- Timeline -->
                            <div>
                                <div class="text-xs uppercase tracking-wide text-ink-mute mb-2">
                                    timeline ({sessionCalls.length})
                                </div>
                                {#if sessionCalls.length === 0}
                                    <p class="text-xs text-ink-soft">No tool calls in this session.</p>
                                {:else}
                                    {#each sessionCalls as call, i (call.callId)}
                                        {@const on = selectedCall?.callId === call.callId}
                                        {@const kindColor = call.verdict === 'used' ? 'var(--success)' : call.verdict === 'partial' ? 'var(--warning)' : call.verdict === 'ignored' ? 'var(--ink-mute)' : 'var(--accent)'}
                                        <button
                                            class="w-full grid grid-cols-[22px_38px_1fr_auto] gap-2 items-center py-2 pl-3 pr-2 text-left border rounded-sm cursor-pointer"
                                            class:bg-paper-soft={on}
                                            class:border-paper-edge={on}
                                            class:border-transparent={!on}
                                            style="background: {on ? 'var(--paper-soft)' : 'transparent'};"
                                            onclick={() => (selectedCall = call)}
                                        >
                                            <span class="font-mono text-xs text-ink-mute" style='font-feature-settings: "tnum";'>{i + 1}</span>
                                            <span class="font-mono text-xs text-ink-mute truncate" style='font-feature-settings: "tnum";'>
                                                {call.startedAtMs ? new Date(call.startedAtMs).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' }) : '—'}
                                            </span>
                                            <span class="font-mono text-xs text-ink truncate">{call.toolName}</span>
                                            <span class="flex items-center gap-1">
                                                <span class="rounded-full inline-block" style="width: 7px; height: 7px; background: {kindColor};" title={call.verdict ?? 'unclassified'}></span>
                                                <span class="font-mono text-xs text-ink-faint" style='font-feature-settings: "tnum";'>
                                                    {call.durationMs != null ? `${call.durationMs}ms` : '—'}
                                                </span>
                                            </span>
                                        </button>
                                    {/each}
                                {/if}
                            </div>

                            <!-- Detail: request + response panels matching CallDetailSimple -->
                            <div class="flex flex-col gap-3">
                                {#if !selectedCall}
                                    <p class="text-sm text-ink-soft">Select a call to see request + response.</p>
                                {:else}
                                    {@const call = selectedCall}
                                    {@const kindGlyph = call.verdict === 'used' ? '用' : call.verdict === 'partial' ? '部' : call.verdict === 'ignored' ? '無' : call.success === false ? '✗' : '問'}
                                    {@const kindColor = call.verdict === 'used' ? 'text-success' : call.verdict === 'partial' ? 'text-warning' : call.verdict === 'ignored' ? 'text-ink-mute' : 'text-accent'}
                                    <div class="flex items-baseline justify-between pb-3 border-b border-paper-mute">
                                        <div>
                                            <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">
                                                call #{call.callId} · {fmtDuration(call.durationMs)}
                                            </div>
                                            <div class="font-mono text-sm text-ink">{call.toolName}</div>
                                        </div>
                                        <span class="inline-flex items-center gap-1 py-1 px-2 text-xs bg-paper-soft border border-paper-edge rounded uppercase tracking-wide {kindColor}">
                                            <span class="kanji text-xs">{kindGlyph}</span>
                                            {call.verdict ?? (call.success === false ? 'error' : 'query')}
                                        </span>
                                    </div>

                                    <div>
                                        <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">request</div>
                                        <pre class="font-mono text-xs leading-normal py-3 px-3 m-0 bg-paper-soft border border-paper-mute rounded whitespace-pre-wrap overflow-auto max-h-[220px] text-ink-soft">{fmtPayload(call.request)}</pre>
                                    </div>

                                    <div>
                                        <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">
                                            response · {call.durationMs != null ? `${call.durationMs}ms` : '—'}
                                        </div>
                                        {#if call.response == null}
                                            <p class="text-xs text-ink-soft m-0 italic">No response captured yet.</p>
                                        {:else}
                                            <pre
                                                class="font-mono text-xs leading-normal py-3 px-3 m-0 bg-paper-soft border border-paper-mute rounded whitespace-pre-wrap overflow-auto max-h-[220px] text-ink"
                                                style="border-left-width: 2px; border-left-color: {call.verdict === 'used' ? 'var(--success)' : call.verdict === 'partial' ? 'var(--warning)' : call.verdict === 'ignored' ? 'var(--ink-mute)' : 'var(--accent)'};"
                                            >{fmtPayload(call.response)}</pre>
                                        {/if}
                                        {#if call.verdictReason}
                                            <div class="text-xs text-ink-faint italic mt-1">
                                                {call.verdictReason}
                                            </div>
                                        {/if}
                                    </div>
                                {/if}
                            </div>
                        </div>
                    {/if}
                </main>
            </div>
        {/if}
    {:else}
        <!-- Health tab — L1 source grid by default; clicking a card drills
             into the existing per-tool (L2) signals + table. L2 signals are
             still all-source today, so the drill header labels that honestly
             rather than faking per-source scoping. -->
        <div class="px-7 py-6">
            {#if activeSource}
                <button
                    type="button"
                    class="bg-transparent border-none cursor-pointer p-0 mb-4 text-sm text-ink-mute"
                    onclick={() => (activeSource = null)}
                    data-testid="tool-health-back"
                >← all sources</button>
                <div class="flex items-baseline gap-2 mb-4">
                    <h3 class="display text-base font-normal m-0">{activeSource.name}</h3>
                    <span class="font-mono text-xs text-ink-mute">
                        · all sources · scoping per source coming
                    </span>
                </div>
                {@render ToolInsights()}
            {:else}
                {@render ToolHealthGrid()}
            {/if}
        </div>
    {/if}
</div>

{#snippet KpiCard(kanji: string, label: string, value: string, hint: string)}
    <div class="py-3 px-3 bg-paper-soft border border-paper-mute rounded-md">
        <div class="flex items-center gap-1 mb-1">
            <span class="kanji text-sm text-accent">{kanji}</span>
            <span class="text-xs uppercase tracking-wide text-ink-mute">{label}</span>
        </div>
        <div class="display text-xl" style='font-feature-settings: "tnum";'>{value}</div>
        <div class="text-xs text-ink-mute mt-1">{hint}</div>
    </div>
{/snippet}

{#snippet ToolHealthGrid()}
    <!-- L1 — source grid. One card per registered source (builtin tool set
         or probed MCP server) from GET /api/instruments/tools-health, with
         KPI header derived from the same rows. The loading branch leads so
         the empty state can't flash during the first fetch. -->
    {#if mcp.toolsHealthStatus === 'loading' || mcp.toolsHealthStatus === 'idle'}
        <p class="text-sm text-ink-soft" data-testid="tool-health-loading">Loading tool sources…</p>
    {:else if toolsHealth.length === 0}
        <EmptyState
            kanji="健"
            title="No tool sources discovered yet"
            description="Sources appear once sensei has scanned your assistant configs and probed each MCP. Built-in tools and every installed MCP show up here with the share of registered tools actually in use."
        />
    {:else}
        {@const k = healthKpis}
        <!-- ── KPI header — derived from the grid rows ───────────────── -->
        <div class="grid grid-cols-4 gap-3 mb-5">
            {@render KpiCard(
                '接', 'servers connected',
                `${k.connected} of ${k.total}`,
                `${k.total - k.connected} disconnected`,
            )}
            {@render KpiCard(
                '具', 'tool coverage',
                k.coverage != null ? `${Math.round(k.coverage * 100)}%` : '—',
                `${k.sumInvoked} of ${k.sumRegistered} invoked`,
            )}
            {@render KpiCard(
                '計', 'total tool calls',
                k.totalCalls.toLocaleString(),
                'across all sources · 14d',
            )}
            {@render KpiCard(
                '一', 'first-try rate',
                holisticFtr != null ? `${Math.round(holisticFtr * 100)}%` : '—',
                'holistic · 14d',
            )}
        </div>

        <!-- ── Source cards ──────────────────────────────────────────── -->
        <div class="flex items-baseline justify-between mb-3">
            <h3 class="display text-base font-normal m-0">
                Sources
                <span class="text-sm text-ink-mute ml-2">· what share of each source's tools is in use</span>
            </h3>
            <span class="font-mono text-xs text-ink-mute">open a source to drill in →</span>
        </div>
        <div
            class="grid gap-3"
            style="grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));"
            data-testid="tool-health-grid"
        >
            {#each toolsHealth as source (source.source_type + ':' + source.source_key)}
                <ToolHealthCard
                    {source}
                    active={activeSource?.source_type === source.source_type
                        && activeSource?.source_key === source.source_key}
                    onopen={openSource}
                />
            {/each}
        </div>
    {/if}
{/snippet}

{#snippet ToolInsights()}
    <!-- Health body — matches instruments.jsx InstrumentsHealth. KPI
         row (4 cards) on top, then Signals grid, then per-tool usage
         table. Verdict split bar layers into each expanded row (Slice D). -->
    {#if toolStats.length === 0}
        <EmptyState
            kanji="健"
            title="No tool usage data yet"
            description="Tool usage statistics appear after your assistant sessions call sensei tools. Start a session to begin tracking."
        />
    {:else}
        {@const totalCalls = toolStats.reduce((s, t) => s + t.call_count, 0)}
        {@const totalErrors = toolStats.reduce((s, t) => s + t.error_count, 0)}
        {@const warnTools = toolSignals.filter((s) => s.variant === 'warn').length}
        <!-- Dormant = signals-side "unused" (no calls in window), not just
             zero call_count on the aggregate row. The aggregate keeps
             historical rows; the signal derives from last_used_at. -->
        {@const dormantTools = toolSignals.filter((s) => s.variant === 'unused').length}

        <!-- ── KPI row — 4 cards, one per key metric ────────────────── -->
        <div class="grid grid-cols-4 gap-3 mb-5">
            <div class="py-3 px-3 bg-paper-soft border border-paper-mute rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <span class="kanji text-sm text-accent">計</span>
                    <span class="text-xs uppercase tracking-wide text-ink-mute">total calls</span>
                </div>
                <div class="display text-xl" style='font-feature-settings: "tnum";'>{totalCalls.toLocaleString()}</div>
                <div class="text-xs text-ink-mute mt-1">{toolStats.length} tools</div>
            </div>
            <div class="py-3 px-3 bg-paper-soft border border-paper-mute rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <span class="kanji text-sm text-accent">誤</span>
                    <span class="text-xs uppercase tracking-wide text-ink-mute">errors</span>
                </div>
                <div class="display text-xl" style='font-feature-settings: "tnum";' class:text-warning={totalErrors > 0}>
                    {totalErrors.toLocaleString()}
                </div>
                <div class="text-xs text-ink-mute mt-1">
                    {totalCalls > 0 ? `${((totalErrors / totalCalls) * 100).toFixed(1)}% rate` : '—'}
                </div>
            </div>
            <div class="py-3 px-3 bg-paper-soft border border-paper-mute rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <span class="kanji text-sm text-accent">警</span>
                    <span class="text-xs uppercase tracking-wide text-ink-mute">signals</span>
                </div>
                <div class="display text-xl" style='font-feature-settings: "tnum";' class:text-warning={warnTools > 0}>
                    {toolSignals.length}
                </div>
                <div class="text-xs text-ink-mute mt-1">
                    {warnTools} need attention
                </div>
            </div>
            <div class="py-3 px-3 bg-paper-soft border border-paper-mute rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <span class="kanji text-sm text-accent">眠</span>
                    <span class="text-xs uppercase tracking-wide text-ink-mute">dormant</span>
                </div>
                <div class="display text-xl" style='font-feature-settings: "tnum";' class:text-warning={dormantTools > 0}>
                    {dormantTools}
                </div>
                <div class="text-xs text-ink-mute mt-1">0 calls</div>
            </div>
        </div>

        <!-- ── Signals — cards grid ─────────────────────────────────── -->
        {#if toolSignals.length > 0}
            <div class="mb-5" data-testid="tool-signals">
                <div class="flex items-baseline justify-between mb-3">
                    <h3 class="display text-base font-normal m-0">
                        Signals
                        <span class="text-sm text-ink-mute ml-2">· what the data suggests you change</span>
                    </h3>
                    <span class="font-mono text-xs text-ink-mute">{toolSignals.length} signals</span>
                </div>
                <div class="grid gap-3" style="grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));">
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

        <!-- ── Per-tool usage table ─────────────────────────────────── -->
        <div class="flex items-baseline justify-between mb-3">
            <h3 class="display text-base font-normal m-0">Per-tool usage</h3>
            <span class="font-mono text-xs text-ink-mute">{toolStats.length} tools · sorted by calls</span>
        </div>
        <div class="flex flex-col gap-1 border border-paper-mute rounded-md overflow-hidden bg-paper" data-testid="insights-table">
            <div class="grid grid-cols-[1fr_80px_80px_100px_120px_90px] gap-3 px-3 py-2 text-xs text-ink-soft tracking-wide uppercase">
                <span>Tool</span>
                <span class="text-right">Calls</span>
                <span class="text-right">Errors</span>
                <span class="text-right">Avg ms</span>
                <span class="text-right">Last used</span>
                <span class="text-right">Verdict</span>
            </div>
            {#each toolStats as stat (stat.tool_name)}
                {@const errorRate = stat.call_count > 0 ? stat.error_count / stat.call_count : 0}
                {@const expanded = selectedInsightTool === stat.tool_name}
                {@const verdict = toolVerdict(stat.call_count, stat.error_count)}
                <button
                    type="button"
                    class="w-full text-left grid grid-cols-[1fr_80px_80px_100px_120px_90px] gap-3 px-3 py-2.5 border-b border-paper-mute text-sm items-center bg-transparent border-l-0 border-r-0 border-t-0 cursor-pointer"
                    class:bg-paper-mute={expanded}
                    data-testid={`insights-row-${stat.tool_name}`}
                    aria-expanded={expanded}
                    onclick={() => (selectedInsightTool = expanded ? null : stat.tool_name)}
                >
                    <span class="font-mono text-xs">{stat.tool_name}</span>
                    <span class="text-right font-mono text-xs">{stat.call_count}</span>
                    <span class="text-right font-mono text-xs" class:text-danger={errorRate > 0.1}>
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
                    <span class="flex justify-end">
                        <span
                            class="inline-flex items-center py-0.5 px-2 text-xs bg-paper-soft border border-paper-edge rounded uppercase tracking-wide"
                            class:text-success={verdict === 'healthy'}
                            class:text-ink-soft={verdict === 'ok'}
                            class:text-warning={verdict === 'warn'}
                            class:text-ink-mute={verdict === 'unused'}
                            data-testid={`verdict-chip-${stat.tool_name}`}
                            data-verdict={verdict}
                        >{verdict}</span>
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
