<script lang="ts">
    import { onMount, untrack } from "svelte";
    import { page } from "$app/state";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { ftrReadLabel } from "$lib/ftr.js";
    import EmptyState from "$lib/components/EmptyState.svelte";
    import ScreenState from "$lib/components/ScreenState.svelte";
    import SignalCard from "$lib/components/SignalCard.svelte";
    import { Eyebrow, Kanji, Screen } from "$lib/components";
    import { Button, Select } from "@rokkit/ui";
    import { mcp, toolVerdict } from "$lib/state/mcp.svelte.js";
    import { buildPlaygroundGroups, serversByFamily, type PlaygroundTool } from "./playground-groups.js";
    import { INSTRUMENT_TABS, kindMark } from "./instrument-marks";
    import { glyphMode } from "$lib/glyphs";
    import ToolHealthCard from "./ToolHealthCard.svelte";
    import { InstrumentsState } from "./instruments-state.svelte";
    import {
        collapsedFor, filterGroups, fmtDate, fmtDuration, fmtPayload, heroCopy as heroCopyFor,
        initialTab, pickToolDefaults as defaultsFor,
    } from "./instruments-view";
    import type { ToolHealthSource } from "$lib/types.js";

    // Presentation only: the screen's state is InstrumentsState, the tool
    // catalogue is the mcp store, and the pure view logic is instruments-view.
    const api = $derived(senseiApi(appState.port));

    // A deep link from the Sessions digest — /instruments?tab=replay&session={id} —
    // opens Replay with that session preselected once the list loads.
    const sessionParam = untrack(() => page.url.searchParams.get("session"));
    const ins = new InstrumentsState(initialTab(untrack(() => page.url.searchParams.get("tab"))));
    let honoredSessionParam = false;

    const tools = $derived(mcp.tools);
    const toolStats = $derived(mcp.toolStats);
    const toolSignals = $derived(mcp.toolSignals);
    const loading = $derived(mcp.catalogStatus === 'loading' || mcp.catalogStatus === 'idle');
    // The Health KPIs derive from the same rows as the grid, so they never drift.
    const toolsHealth = $derived(mcp.toolsHealth);
    const healthKpis = $derived(mcp.toolsHealthKpis);
    const selectedInsight = $derived(ins.selectedInsightTool ? mcp.insightFor(ins.selectedInsightTool) : undefined);

    // Sensei's own tools and every discovered server are groups in the left rail;
    // a discovered server's tools load when its group first expands.
    const discoveredServers = $derived(mcp.mcpServers);
    const discoveredByFamily = $derived(serversByFamily(discoveredServers));
    const playgroundGroups = $derived(
        buildPlaygroundGroups(tools, discoveredServers, mcp.serverToolManifests, mcp.serverToolErrors),
    );
    const filteredGroups = $derived(filterGroups(playgroundGroups, ins.toolSearch));
    const effectiveCollapsed = $derived(collapsedFor(filteredGroups, ins.collapsedGroups, ins.toolSearch));
    const visibleFlat = $derived(filteredGroups.flatMap((g) => g.tools));

    const focusedGroupAndTool = $derived.by(() => {
        const byId = new Map(playgroundGroups.flatMap((g) => g.tools.map((t) => [t.id, { group: g, tool: t }])));
        if (ins.selectedToolId && byId.has(ins.selectedToolId)) return byId.get(ins.selectedToolId)!;
        const first = visibleFlat[0];
        return first ? byId.get(first.id) ?? null : null;
    });
    const focusedGroup = $derived(focusedGroupAndTool?.group ?? null);
    const focusedTool = $derived(focusedGroupAndTool?.tool ?? null);
    // A sensei tool has a manifest (structured inputs + example); a discovered
    // server's tool does not, so its inputs read "No inputs".
    const selectedTool = $derived(
        focusedTool?.mcp === 'sensei' ? tools.find((t) => t.name === focusedTool.name) ?? null : null,
    );

    // Tabs and kind marks carry a kanji AND an icon (#133). The third tab reads
    // "Health"; its id stays 'insights' so bookmarks and test ids don't churn.
    const instrumentTabs = INSTRUMENT_TABS;
    const glyphs = $derived(glyphMode(appState.config));
    const kindTone = (kind: PlaygroundTool['kind']) =>
        kind === 'action' ? 'accent' : kind === 'query' ? 'success' : 'muted';
    const currentTab = $derived(instrumentTabs.find((t) => t.id === ins.tab) ?? instrumentTabs[0]);
    const heroCopy = $derived(heroCopyFor(ins.tab));

    function pickToolDefaults(tool: PlaygroundTool): Record<string, string> {
        const manifest = tool.mcp === 'sensei' ? tools.find((t) => t.name === tool.name) ?? null : null;
        return defaultsFor(manifest, appState.activeProjectId || null);
    }

    function openSource(source: ToolHealthSource) {
        ins.activeSource = source;
    }

    function toggleGroup(id: string) {
        const expanding = ins.collapsedGroups[id];
        ins.collapsedGroups = { ...ins.collapsedGroups, [id]: !ins.collapsedGroups[id] };
        const group = playgroundGroups.find((g) => g.id === id);
        if (expanding && group?.source === 'discovered' && group.serverId && !mcp.serverToolManifests[group.serverId]) {
            void mcp.loadServerTools(group.serverId);
        }
    }

    async function refreshDiscoveredServers() {
        ins.refreshingServers = true;
        try {
            ins.serversError = await mcp.refreshMcpServers();
        } finally {
            ins.refreshingServers = false;
        }
    }

    async function toggleServer(serverId: string, enabled: boolean) {
        ins.serversError = await mcp.toggleMcpServer(serverId, enabled);
    }

    function executeTool() {
        if (selectedTool) void ins.execute(api, selectedTool.name);
    }

    function selectReplaySession(sessionId: string) {
        void ins.selectReplaySession(mcp, sessionId);
    }

    // Default: sensei expanded, every discovered server collapsed.
    $effect(() => {
        for (const g of playgroundGroups) {
            if (!(g.id in untrack(() => ins.collapsedGroups))) {
                ins.collapsedGroups = { ...untrack(() => ins.collapsedGroups), [g.id]: g.source !== 'sensei' };
            }
        }
    });

    onMount(() => {
        void mcp.loadCatalog();
    });

    // Each tab's reads, the first time it opens. Depends on the tab ONLY: each
    // load checks its own status, and tracking that re-ran a failed read without
    // end. A failed read is retried from its error state.
    $effect(() => {
        const current = ins.tab;
        untrack(() => {
            if (current === 'playground') void mcp.loadMcpServers();
            if (current === 'replay') void ins.loadReplaySessions(api);
            if (current === 'insights') {
                void mcp.loadToolsHealth();
                void mcp.loadInsights();
                void ins.loadHolisticFtr(api);
            }
        });
    });

    // Preselect the deep-linked session once the list is read — at most once.
    $effect(() => {
        if (honoredSessionParam || !sessionParam || ins.tab !== 'replay') return;
        if (!ins.replayLoaded) return;
        honoredSessionParam = true;
        selectReplaySession(sessionParam);
    });
</script>

<!-- #84 T2 UI — Instruments custom shell matching
     docs/mockups/Sensei/lib/instruments.jsx. Hero header (kanji + eyebrow +
     tagline + sub) sits above a kanji-decorated tab nav; the three tabs
     read as one-line questions (Playground: what can these instruments do?
     · Replay: what did the assistant do? · Health: what should we change?)
     matching the mockup's phrasing.
     Full hero — kanji + eyebrow (Instruments · <tab>) + tagline + sub -->
<!-- Hero + tabs stay in place over the scrolling body (#293). Full width — no
     max-w wrapper — so the panels below fill the observatory main pane
     edge-to-edge (fixes the L/R gap the user hit). -->
<Screen>
    {#snippet header()}
    <div class="bg-paper">
        <div class="flex items-end gap-4 pt-5 pb-4 px-7 border-b border-paper-edge" data-testid="instruments-hero">
            <Kanji char={currentTab.kanji} icon={currentTab.icon} mode={glyphs} size="4xl" />
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
        <div class="flex px-7 border-b border-paper-edge" role="tablist" aria-label="Instruments tabs" data-testid="instrument-tabs">
            {#each instrumentTabs as t}
                {@const on = t.id === ins.tab}
                <button
                    class="flex items-center gap-2 py-3 px-4 bg-transparent border-none cursor-pointer"
                    class:text-ink={on}
                    class:text-ink-soft={!on}
                    style="border-bottom: {on ? '2px solid var(--ink)' : '2px solid transparent'}; margin-bottom: -1px;"
                    role="tab"
                    aria-selected={on}
                    data-testid={`instrument-tab-${t.id}`}
                    onclick={() => (ins.tab = t.id)}
                >
                    <Kanji char={t.kanji} icon={t.icon} mode={glyphs} size="sm" tone={on ? 'accent' : 'muted'} />
                    <span class="display text-sm">{t.label}</span>
                </button>
            {/each}
        </div>
    </div>
    {/snippet}
    <!-- Body is full width; Playground/Replay use their own internal grids,
         Health gets horizontal padding so cards breathe but still reach the
         edges (no centered max-w wrapper any more). -->
    <div>

        {#if ins.tab === "playground"}
            {#if mcp.catalogError}
                <!-- The tool catalog could not be read (#288) — sensei's own tools
                     would otherwise just be missing from the rail. -->
                <div class="px-7 pt-4">
                    <ScreenState status="error" error={`Could not read the tool catalog: ${mcp.catalogError}`} onretry={() => void mcp.loadCatalog(true)} />
                </div>
            {/if}
            <!-- #84 T2 Playground — matches docs/mockups/Sensei/lib/instruments-simple.jsx
                 (InstrumentsPlaygroundSimple). Left rail: search + collapsible MCP
                 groups → tools. Right: tool detail with inputs form + response
                 preview. Sensei's own tools sit as the first (always-installed)
                 group; every discovered MCP server (Slice A) is its own group
                 with tools hydrated lazily on expand via Slice B's probe. -->
            {@const running = ins.executing}
            <div class="grid grid-cols-[300px_1fr] min-h-[600px] border-t border-paper-edge" data-testid="playground-body">
                <!-- ── Left rail — search + MCP tree ────────────────────────────── -->
                <aside class="bg-paper-soft border-r border-paper-edge flex flex-col overflow-hidden">
                    <!-- Search box -->
                    <div class="flex items-center gap-2 py-3 px-3 border-b border-paper-edge">
                        <Kanji char="探" size="inherit" tone="inherit" class="text-xs text-ink-mute" />
                        <input
                            type="text"
                            class="border-none outline-none bg-transparent text-sm text-ink flex-1"
                            placeholder="search tools…"
                            bind:value={ins.toolSearch}
                            data-testid="tool-search"
                        />
                        {#if ins.toolSearch}
                            <button class="text-xs text-ink-faint bg-transparent border-none cursor-pointer" onclick={() => (ins.toolSearch = '')} aria-label="Clear search">×</button>
                        {/if}
                    </div>

                    <!-- MCP groups — sensei first, then the discovered servers by assistant -->
                    {#snippet groupBlock(group: (typeof playgroundGroups)[number], server?: (typeof discoveredServers)[number])}
                        {@const collapsed = effectiveCollapsed[group.id] ?? (group.source !== 'sensei')}
                        <div class="mb-1" data-testid={`mcp-group-${group.id}`}>
                            <!-- Expand and enable are siblings: a button inside a button is invalid HTML. -->
                            <div class="flex items-center" data-testid={server ? `server-row-${server.mcp_key}` : undefined}>
                                <button
                                    class="flex-1 min-w-0 grid grid-cols-[14px_18px_1fr_auto_auto] items-center gap-2 py-2 px-3 text-left bg-transparent border-none cursor-pointer text-ink-soft"
                                    onclick={() => toggleGroup(group.id)}
                                    aria-expanded={!collapsed}
                                    data-testid={`mcp-group-toggle-${group.id}`}
                                >
                                    <span
                                        class="font-mono text-xs text-ink-mute transition-transform duration-fast"
                                        style="transform: {collapsed ? 'none' : 'rotate(90deg)'};"
                                    >▶</span>
                                    <Kanji char={group.kanji} icon={group.icon} mode={glyphs} size="sm" />
                                    <span class="text-sm truncate">{group.name}</span>
                                    <span></span>
                                    <span class="font-mono text-xs text-ink-faint">{group.tools.length}</span>
                                </button>
                                {#if server}
                                    <button
                                        class="mr-3 px-2 py-0.5 text-xs font-mono uppercase tracking-wide rounded border border-paper-edge bg-transparent cursor-pointer"
                                        class:text-success={server.enabled}
                                        class:text-ink-mute={!server.enabled}
                                        role="switch"
                                        aria-checked={server.enabled}
                                        aria-label={`${server.enabled ? 'Disable' : 'Enable'} ${server.mcp_key}`}
                                        onclick={() => void toggleServer(server.id, !server.enabled)}
                                        data-testid={`server-toggle-${server.mcp_key}`}
                                    >{server.enabled ? 'on' : 'off'}</button>
                                {/if}
                            </div>

                            <!-- Tool lines -->
                            {#if !collapsed && group.error}
                                <p class="text-xs text-danger px-3 py-2 m-0" data-testid={`group-error-${group.id}`}>Could not read its tools: {group.error}</p>
                            {/if}
                            {#if !collapsed}
                                <div>
                                    {#each group.tools as tool (tool.id)}
                                        {@const active = focusedTool?.id === tool.id}
                                        {@const isAction = tool.kind === 'action'}
                                        {@const mark = kindMark(tool.kind)}
                                        <button
                                            class="w-full grid grid-cols-[32px_14px_1fr] gap-1 py-1 pl-1 pr-3 text-left bg-transparent border-none cursor-pointer"
                                            style="border-left: {active ? '2px solid var(--accent)' : '2px solid transparent'};"
                                            onclick={() => ins.pickTool(tool.id, pickToolDefaults(tool))}
                                            data-testid={`tool-row-${tool.name}`}
                                            data-tool-kind={tool.kind}
                                        >
                                            <span></span>
                                            <Kanji char={mark.kanji} icon={mark.icon} mode={glyphs} size="xs" tone={kindTone(tool.kind)} />
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
                    {/snippet}

                    <div class="overflow-auto flex-1 py-1">
                        {#if filteredGroups.length === 0 && ins.toolSearch}
                            <div class="text-center text-sm text-ink-faint py-4 px-3">
                                no tools match.
                            </div>
                        {/if}
                        {#each filteredGroups.filter((g) => g.source === 'sensei') as group (group.id)}
                            {@render groupBlock(group)}
                        {/each}

                        <section class="mt-3 pt-3 border-t border-paper-edge" data-testid="discovered-servers">
                            <div class="px-3 pb-1"><Eyebrow>Discovered MCP servers</Eyebrow></div>
                            {#if ins.serversError}
                                <p class="px-3 py-1 text-xs text-danger" data-testid="servers-error">{ins.serversError}</p>
                            {/if}
                            {#if mcp.mcpServersStatus === 'error'}
                                <p class="px-3 py-1 text-xs text-danger" data-testid="servers-load-error">
                                    Could not read the discovered servers from the daemon.
                                </p>
                            {:else if discoveredServers.length === 0 && mcp.mcpServersStatus === 'ready'}
                                <p class="px-3 py-2 text-xs text-ink-soft">
                                    No MCP servers discovered yet. Configure one in Claude Code, Cursor or Zed, then scan.
                                </p>
                            {/if}
                            {#each discoveredByFamily as fam (fam.family)}
                                {@const shown = fam.servers
                                    .map((srv) => ({ server: srv, group: filteredGroups.find((g) => g.id === srv.id) }))
                                    .filter((x) => x.group)}
                                {#if shown.length > 0}
                                    <div class="mb-1" data-testid={`servers-family-${fam.family}`}>
                                        <div class="px-3 pt-2 pb-1 text-xs uppercase tracking-wide text-ink-faint">{fam.label}</div>
                                        {#each shown as { server, group } (server.id)}
                                            {@render groupBlock(group!, server)}
                                        {/each}
                                    </div>
                                {/if}
                            {/each}
                        </section>
                    </div>

                    <!-- Footer — MCP + tool counts + refresh -->
                    <div class="border-t border-paper-edge flex justify-between items-center gap-2 py-2 px-3 text-xs text-ink-faint">
                        <span>{playgroundGroups.length} MCPs · {playgroundGroups.reduce((n, g) => n + g.tools.length, 0)} tools</span>
                        <button
                            class="text-xs text-ink-mute bg-transparent border-none cursor-pointer"
                            onclick={refreshDiscoveredServers}
                            disabled={ins.refreshingServers}
                            data-testid="refresh-servers"
                        >{ins.refreshingServers ? 'scanning…' : '+ scan'}</button>
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
                        {@const mark = kindMark(tool.kind)}
                        {@const kindHint = isAction ? 'performs an operation' : 'returns information'}

                        <!-- Heading -->
                        <div class="mb-4">
                            <div class="flex items-center flex-wrap gap-2 mb-2">
                                <span class="inline-flex items-center gap-1 font-mono text-xs text-ink-mute"><Kanji char={group.kanji} icon={group.icon} mode={glyphs} size="xs" tone="muted" /> {group.name.toLowerCase()}</span>
                                <span class="text-xs text-ink-faint">·</span>
                                <span
                                    class="inline-flex items-center gap-1 py-1 px-2 text-xs bg-paper-soft border border-paper-edge rounded uppercase tracking-wide"
                                    class:text-accent={isAction}
                                    class:text-success={tool.kind === 'query'}
                                    class:text-ink-mute={tool.kind === 'other'}
                                >
                                    <Kanji char={mark.kanji} icon={mark.icon} mode={glyphs} size="xs" tone={kindTone(tool.kind)} />
                                    {kindLabel}
                                </span>
                            </div>
                            <h2 class="font-mono text-lg font-normal m-0 text-ink">{tool.name}</h2>
                            <p class="text-sm text-ink-soft leading-normal mt-1 mb-0 max-w-[700px]">
                                {tool.summary || 'No description.'}
                            </p>
                        </div>

                        <!-- Inputs panel -->
                        <div class="bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 mb-3">
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
                                                    value={ins.toolParams[input.key]}
                                                    onchange={(v) => (ins.toolParams[input.key] = v as string)}
                                                />
                                            {:else}
                                                <input
                                                    type={input.kind === 'number' ? 'number' : 'text'}
                                                    class="py-1 px-2 text-sm border border-paper-edge rounded bg-paper text-ink font-mono outline-none"
                                                    placeholder={input.placeholder ?? input.default ?? ''}
                                                    bind:value={ins.toolParams[input.key]}
                                                />
                                            {/if}
                                        </label>
                                    {/each}
                                </div>
                            {/if}

                            <div class="flex items-center gap-2 mt-3 pt-2 border-t border-paper-edge">
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
                                {:else if ins.toolError}
                                    <span class="font-mono text-xs text-danger" data-testid="tool-error">{ins.toolError}</span>
                                {:else if ins.toolResult}
                                    <span class="font-mono text-xs text-success">200 ok</span>
                                {/if}
                            </div>
                        </div>

                        <!-- Response panel -->
                        <div>
                            <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">
                                Response{!ins.toolResult ? ' · preview' : ''}
                            </div>
                            <pre
                                class="font-mono text-xs leading-normal py-3 px-3 m-0 bg-paper-soft border border-paper-edge rounded whitespace-pre-wrap overflow-auto max-h-[360px]"
                                class:text-ink={ins.toolResult}
                                class:text-ink-soft={!ins.toolResult}
                                class:opacity-70={!ins.toolResult}
                                class:border-l-accent={isAction}
                                class:border-l-success={tool.kind === 'query'}
                                style="border-left-width: 2px; border-left-color: {isAction ? 'var(--accent)' : tool.kind === 'query' ? 'var(--success)' : 'var(--ink-mute)'};"
                            >{ins.toolResult || '—'}</pre>
                        </div>
                    {:else}
                        <div class="text-center text-sm text-ink-faint py-6">
                            Pick a tool to inspect it.
                        </div>
                    {/if}
                </main>
            </div>
        {:else if ins.tab === "replay"}
            <!-- Replay body — matches instruments.jsx InstrumentsReplay (used
                 by InstrumentsReplaySimple with simple=true). 300px session
                 picker sidebar + main pane with summary strip + timeline+
                 detail grid. Timeline entries use the mockup's [22px | 38px |
                 1fr | auto] grid (index / offset / tool name / dot+ms). -->
            {#if ins.replayLoading && ins.replaySessions.length === 0}
                <p class="text-sm text-ink-soft px-7 py-5">Loading sessions…</p>
            {:else if ins.replayError}
                <div class="px-7 py-5">
                    <ScreenState
                        status="error"
                        error={ins.replayError}
                        onretry={() => void ins.retryReplaySessions(api)}
                    />
                </div>
            {:else if ins.replaySessions.length === 0}
                <EmptyState
                    kanji="録"
                    title="No sessions recorded yet"
                    description="Tool calls from your assistant sessions appear here once sensei has captured at least one session."
                />
            {:else}
                <div class="grid grid-cols-[300px_1fr] min-h-[600px] border-t border-paper-edge">
                    <!-- ── Left rail — session picker ─────────────────── -->
                    <aside class="bg-paper-soft border-r border-paper-edge overflow-auto">
                        <div class="text-xs uppercase tracking-wide text-ink-mute pt-3 pb-2 px-3">
                            sessions
                        </div>
                        {#each ins.replaySessions as session (session.id)}
                            {@const on = ins.selectedSessionId === session.id}
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
                        {#if ins.selectedSessionId == null}
                            <p class="text-sm text-ink-soft">Pick a session on the left.</p>
                        {:else if ins.replayLoading}
                            <p class="text-sm text-ink-soft">Loading timeline…</p>
                        {:else}
                            <!-- Session summary strip -->
                            <div class="grid grid-cols-[auto_auto_auto_auto_1fr] items-baseline gap-5 mb-4 pb-3 border-b border-paper-edge">
                                <div>
                                    <div class="display text-base mb-1">
                                        {ins.replaySessions.find((r) => r.id === ins.selectedSessionId)?.task ?? ins.selectedSessionId}
                                    </div>
                                    <div class="font-mono text-xs text-ink-mute">{ins.selectedSessionId}</div>
                                </div>
                                <div>
                                    <div class="text-xs uppercase tracking-wide text-ink-faint">calls</div>
                                    <div class="display text-base mt-1">{ins.sessionCalls.length}</div>
                                </div>
                                <div>
                                    <div class="text-xs uppercase tracking-wide text-ink-faint">used</div>
                                    <div class="display text-base text-success mt-1">{ins.sessionSummary.used}</div>
                                </div>
                                <div>
                                    <div class="text-xs uppercase tracking-wide text-ink-faint">ignored</div>
                                    <div class="display text-base text-ink-mute mt-1">{ins.sessionSummary.ignored}</div>
                                </div>
                                <span></span>
                            </div>

                            <!-- Timeline + detail grid — matches mockup's 0.95fr 1.4fr split -->
                            <div class="grid grid-cols-[0.95fr_1.4fr] gap-5" style="min-height: 400px;">
                                <!-- Timeline -->
                                <div>
                                    <div class="text-xs uppercase tracking-wide text-ink-mute mb-2">
                                        timeline ({ins.sessionCalls.length})
                                    </div>
                                    {#if ins.replayDetailError}
                                        <p class="text-xs text-danger" data-testid="replay-detail-error">Could not read this session's tool calls: {ins.replayDetailError}</p>
                                    {:else if ins.sessionCalls.length === 0}
                                        <p class="text-xs text-ink-soft">No tool calls in this session.</p>
                                    {:else}
                                        {#each ins.sessionCalls as call, i (call.callId)}
                                            {@const on = ins.selectedCall?.callId === call.callId}
                                            {@const kindColor = call.verdict === 'used' ? 'var(--success)' : call.verdict === 'partial' ? 'var(--warning)' : call.verdict === 'ignored' ? 'var(--ink-mute)' : 'var(--accent)'}
                                            <button
                                                class="w-full grid grid-cols-[22px_38px_1fr_auto] gap-2 items-center py-2 pl-3 pr-2 text-left border rounded-sm cursor-pointer"
                                                class:bg-paper-soft={on}
                                                class:border-paper-edge={on}
                                                class:border-transparent={!on}
                                                style="background: {on ? 'var(--paper-soft)' : 'transparent'};"
                                                onclick={() => (ins.selectedCall = call)}
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
                                    {#if !ins.selectedCall}
                                        <p class="text-sm text-ink-soft">Select a call to see request + response.</p>
                                    {:else}
                                        {@const call = ins.selectedCall}
                                        {@const kindGlyph = call.verdict === 'used' ? '用' : call.verdict === 'partial' ? '部' : call.verdict === 'ignored' ? '無' : call.success === false ? '✗' : '問'}
                                        {@const kindColor = call.verdict === 'used' ? 'text-success' : call.verdict === 'partial' ? 'text-warning' : call.verdict === 'ignored' ? 'text-ink-mute' : 'text-accent'}
                                        <div class="flex items-baseline justify-between pb-3 border-b border-paper-edge">
                                            <div>
                                                <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">
                                                    call #{call.callId} · {fmtDuration(call.durationMs)}
                                                </div>
                                                <div class="font-mono text-sm text-ink">{call.toolName}</div>
                                            </div>
                                            <span class="inline-flex items-center gap-1 py-1 px-2 text-xs bg-paper-soft border border-paper-edge rounded uppercase tracking-wide {kindColor}">
                                                <Kanji char={kindGlyph} size="inherit" tone="inherit" class="text-xs" />
                                                {call.verdict ?? (call.success === false ? 'error' : 'query')}
                                            </span>
                                        </div>

                                        <div>
                                            <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">request</div>
                                            <pre class="font-mono text-xs leading-normal py-3 px-3 m-0 bg-paper-soft border border-paper-edge rounded whitespace-pre-wrap overflow-auto max-h-[220px] text-ink-soft">{fmtPayload(call.request)}</pre>
                                        </div>

                                        <div>
                                            <div class="text-xs uppercase tracking-wide text-ink-mute mb-1">
                                                response · {call.durationMs != null ? `${call.durationMs}ms` : '—'}
                                            </div>
                                            {#if call.response == null}
                                                <p class="text-xs text-ink-soft m-0 italic">No response captured yet.</p>
                                            {:else}
                                                <pre
                                                    class="font-mono text-xs leading-normal py-3 px-3 m-0 bg-paper-soft border border-paper-edge rounded whitespace-pre-wrap overflow-auto max-h-[220px] text-ink"
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
                {#if ins.activeSource}
                    <button
                        type="button"
                        class="bg-transparent border-none cursor-pointer p-0 mb-4 text-sm text-ink-mute"
                        onclick={() => (ins.activeSource = null)}
                        data-testid="tool-health-back"
                    >← all sources</button>
                    <div class="flex items-baseline gap-2 mb-4">
                        <h3 class="display text-base font-normal m-0">{ins.activeSource.name}</h3>
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
</Screen>

{#snippet KpiCard(kanji: string, label: string, value: string, hint: string)}
    <div class="py-3 px-3 bg-paper-soft border border-paper-edge rounded-md">
        <div class="flex items-center gap-1 mb-1">
            <Kanji char={kanji} size="inherit" tone="inherit" class="text-sm text-accent" />
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
    {:else if mcp.toolsHealthError}
        <ScreenState status="error" error={mcp.toolsHealthError} onretry={() => void mcp.loadToolsHealth(true)} />
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
                ftrReadLabel(ins.holisticFtr),
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
                    active={ins.activeSource?.source_type === source.source_type
                        && ins.activeSource?.source_key === source.source_key}
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
    {#if mcp.catalogError || mcp.insightsError}
        <ScreenState
            status="error"
            error={mcp.catalogError ?? mcp.insightsError}
            onretry={() => { void mcp.loadCatalog(true); void mcp.loadInsights(true); }}
        />
    {:else if toolStats.length === 0}
        <EmptyState
            kanji="健"
            title="No tool usage data yet"
            description="Tool usage statistics appear after your assistant sessions call sensei tools. Start a session to begin tracking."
        />
    {:else}
        {@const totalCalls = toolStats.reduce((n, t) => n + t.call_count, 0)}
        {@const totalErrors = toolStats.reduce((n, t) => n + t.error_count, 0)}
        {@const warnTools = toolSignals.filter((sig) => sig.variant === 'warn').length}
        <!-- Dormant = signals-side "unused" (no calls in window), not just
             zero call_count on the aggregate row. The aggregate keeps
             historical rows; the signal derives from last_used_at. -->
        {@const dormantTools = toolSignals.filter((sig) => sig.variant === 'unused').length}

        <!-- ── KPI row — 4 cards, one per key metric ────────────────── -->
        <div class="grid grid-cols-4 gap-3 mb-5">
            <div class="py-3 px-3 bg-paper-soft border border-paper-edge rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <Kanji char="計" size="inherit" tone="inherit" class="text-sm text-accent" />
                    <span class="text-xs uppercase tracking-wide text-ink-mute">total calls</span>
                </div>
                <div class="display text-xl" style='font-feature-settings: "tnum";'>{totalCalls.toLocaleString()}</div>
                <div class="text-xs text-ink-mute mt-1">{toolStats.length} tools</div>
            </div>
            <div class="py-3 px-3 bg-paper-soft border border-paper-edge rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <Kanji char="誤" size="inherit" tone="inherit" class="text-sm text-accent" />
                    <span class="text-xs uppercase tracking-wide text-ink-mute">errors</span>
                </div>
                <div class="display text-xl" style='font-feature-settings: "tnum";' class:text-warning={totalErrors > 0}>
                    {totalErrors.toLocaleString()}
                </div>
                <div class="text-xs text-ink-mute mt-1">
                    {totalCalls > 0 ? `${((totalErrors / totalCalls) * 100).toFixed(1)}% rate` : '—'}
                </div>
            </div>
            <div class="py-3 px-3 bg-paper-soft border border-paper-edge rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <Kanji char="警" size="inherit" tone="inherit" class="text-sm text-accent" />
                    <span class="text-xs uppercase tracking-wide text-ink-mute">signals</span>
                </div>
                <div class="display text-xl" style='font-feature-settings: "tnum";' class:text-warning={warnTools > 0}>
                    {toolSignals.length}
                </div>
                <div class="text-xs text-ink-mute mt-1">
                    {warnTools} need attention
                </div>
            </div>
            <div class="py-3 px-3 bg-paper-soft border border-paper-edge rounded-md">
                <div class="flex items-center gap-1 mb-1">
                    <Kanji char="眠" size="inherit" tone="inherit" class="text-sm text-accent" />
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
        <div class="flex flex-col gap-1 border border-paper-edge rounded-md overflow-hidden bg-paper" data-testid="insights-table">
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
                {@const expanded = ins.selectedInsightTool === stat.tool_name}
                {@const verdict = toolVerdict(stat.call_count, stat.error_count)}
                <button
                    type="button"
                    class="w-full text-left grid grid-cols-[1fr_80px_80px_100px_120px_90px] gap-3 px-3 py-2.5 border-b border-paper-edge text-sm items-center bg-transparent border-l-0 border-r-0 border-t-0 cursor-pointer"
                    class:bg-paper-mute={expanded}
                    data-testid={`insights-row-${stat.tool_name}`}
                    aria-expanded={expanded}
                    onclick={() => (ins.selectedInsightTool = expanded ? null : stat.tool_name)}
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
                    <div class="px-3 pb-3 pt-1 border-b border-paper-edge bg-paper-soft" data-testid={`insights-detail-${stat.tool_name}`}>
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
                                <div class="mt-3 pt-3 border-t border-paper-edge">
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
                                    <div class="flex gap-3 mt-1.5 text-xs">
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
