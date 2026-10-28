<script lang="ts">
    import { ftrBandView } from '$lib/ftr';
    import { PageHeader, Screen, ScreenState } from '$lib/components';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import { invalidateAll } from '$app/navigation';
    import { page } from '$app/state';
    import type { WorkspaceMcpToolStat, WorkspaceService } from '$lib/types.js';
    import ToolBubbles from './ToolBubbles.svelte';
    import { WorkspaceServicesState } from './services-state.svelte.js';

    let { data } = $props();
    const workspaceId = $derived(page.params.id ?? '');

    // Kind chip filter mirrors the observatory Playground so users move
    // between the two pages with the same mental model.
    let kindFilter = $state<'all' | 'query' | 'action'>('all');
    const kindChips: Array<{ id: 'all' | 'query' | 'action'; label: string }> = [
        { id: 'all', label: 'All' },
        { id: 'query', label: 'Queries' },
        { id: 'action', label: 'Actions' },
    ];

    const stats: WorkspaceMcpToolStat[] = $derived(data.mcpToolStats);
    const services: WorkspaceService[] = $derived(data.services);
    const visibleStats = $derived(
        kindFilter === 'all' ? stats : stats.filter((t) => t.kind === kindFilter),
    );

    const totalCalls = $derived(stats.reduce((a, t) => a + (t.calls || 0), 0));
    const totalErrors = $derived(stats.reduce((a, t) => a + (t.errors || 0), 0));

    // Per-service toggle: in flight while its write is out (the row disables,
    // so no double-flip), and a refusal is said on the row (#304).
    const servicesState = new WorkspaceServicesState();
    const toggleService = (service: WorkspaceService) =>
        servicesState.toggle(senseiApi(appState.port), workspaceId, service, invalidateAll);

    function fmtInt(n: number | null | undefined): string {
        return n == null ? '—' : n.toLocaleString();
    }
    function fmtPct(n: number | null | undefined): string {
        return n == null ? '—' : `${Math.round(n * 100)}%`;
    }
    function fmtMs(n: number | null | undefined): string {
        if (n == null) return '—';
        if (n < 1000) return `${Math.round(n)} ms`;
        return `${(n / 1000).toFixed(2)} s`;
    }
</script>

<Screen>
    {#snippet header()}
    <PageHeader title="Instruments">
        {#snippet right()}
            {#if !data.error}
                <span class="text-sm text-ink-soft">
                    {stats.length} tools · {fmtInt(totalCalls)} calls · {fmtInt(totalErrors)} errors
                </span>
            {/if}
        {/snippet}
    </PageHeader>
    {/snippet}

    <div class="px-6 py-6">
        {#if data.error}
            <ScreenState status="error" error={data.error} onretry={() => invalidateAll()} />
        {:else}
        <!-- Services (MCP servers, inference providers) — per-workspace toggle. -->
        {#if services.length > 0}
            <section class="mb-8">
                <h3 class="text-sm font-medium m-0 mb-3">Services</h3>
                <ul class="list-none m-0 p-0 flex flex-col gap-1">
                    {#each services as service (service.id)}
                        <li class="flex items-center gap-3 py-2 px-2 rounded-md border border-paper-edge">
                            <div class="flex-1 min-w-0">
                                <div class="text-sm font-medium text-ink truncate">{service.displayName}</div>
                                <div class="text-xs text-ink-mute truncate">
                                    <span class="font-mono">{service.name}</span>
                                    <span class="opacity-60"> · {service.protocol} · {service.kind}</span>
                                    {#if service.publisher} · {service.publisher}{/if}
                                </div>
                            </div>
                            {#if service.toolsCount > 0}
                                <span class="text-xs text-ink-mute">{service.toolsCount} tool{service.toolsCount === 1 ? '' : 's'}</span>
                            {/if}
                            <button
                                type="button"
                                class="toggle text-xs px-3 py-1 rounded-full border cursor-pointer"
                                class:enabled={service.enabledForWorkspace}
                                disabled={servicesState.isToggling(service.id)}
                                aria-pressed={service.enabledForWorkspace}
                                data-testid={`service-toggle-${service.name}`}
                                onclick={() => toggleService(service)}
                            >{service.enabledForWorkspace ? 'On' : 'Off'}</button>
                        </li>
                        {#if servicesState.toggleError(service.id)}
                            <li class="text-xs text-danger px-2" data-testid={`service-toggle-error-${service.name}`}>
                                {service.displayName}: {servicesState.toggleError(service.id)}
                            </li>
                        {/if}
                    {/each}
                </ul>
            </section>
        {/if}

        <!-- Which tools the assistants used, built-ins included, sized by calls
             (#291 — it hung off the retired unused_tools metric and could not be
             reached). Its own read: a failure here does not blank the page. -->
        <section class="mb-8">
            <h3 class="text-sm font-medium m-0 mb-3">Tools used</h3>
            {#if data.toolUsageError}
                <p class="text-xs text-danger m-0" data-testid="tools-used-error">
                    Could not read which tools were used: {data.toolUsageError}
                </p>
            {:else if data.toolUsage.length > 0}
                <div data-testid="tools-used"><ToolBubbles tools={data.toolUsage} /></div>
            {:else}
                <p class="text-xs text-ink-mute m-0">No tool calls captured for this workspace yet.</p>
            {/if}
        </section>

        <!-- MCP tool aggregation — the new lens per T2 Slice F. -->
        <section class="mb-8">
            <h3 class="text-sm font-medium m-0 mb-3">MCP tools</h3>

            <div class="flex gap-2 mb-3" role="tablist" aria-label="Tool kind filter">
                {#each kindChips as chip}
                    {@const active = kindFilter === chip.id}
                    <button
                        class="px-3 py-1 rounded-full border text-xs cursor-pointer transition-colors duration-fast"
                        class:bg-primary={active}
                        class:text-on-primary={active}
                        class:border-primary={active}
                        class:bg-transparent={!active}
                        class:text-ink-mute={!active}
                        class:border-paper-edge={!active}
                        role="tab"
                        aria-selected={active}
                        onclick={() => (kindFilter = chip.id)}
                    >{chip.label}</button>
                {/each}
            </div>

            {#if visibleStats.length === 0}
                <p class="text-sm text-ink-mute">No tools match this filter.</p>
            {:else}
                <div class="grid grid-cols-[1fr_60px_60px_80px_80px_100px] gap-3 px-3 py-2 text-xs text-ink-mute tracking-wide uppercase">
                    <span>Tool</span>
                    <span class="text-right">Calls</span>
                    <span class="text-right">Errors</span>
                    <span class="text-right">Avg</span>
                    <span class="text-right">FTR</span>
                    <span class="text-right">Last used</span>
                </div>
                {#each visibleStats as t (t.id)}
                    <div class="grid grid-cols-[1fr_60px_60px_80px_80px_100px] gap-3 px-3 py-2 border-b border-paper-edge text-sm items-center">
                        <div class="min-w-0">
                            <div class="font-mono text-xs truncate">{t.name}</div>
                            <div class="text-xs text-ink-mute truncate">{t.summary}</div>
                        </div>
                        <span class="text-right font-mono text-xs">{fmtInt(t.calls)}</span>
                        <span class="text-right font-mono text-xs" class:text-danger={t.errors > 0}>
                            {fmtInt(t.errors)}
                        </span>
                        <span class="text-right font-mono text-xs opacity-70">{fmtMs(t.avgDurationMs)}</span>
                        <!-- The shared FTR band (lib/ftr): 0.8 / 0.6, as every screen. -->
                        <span class="text-right font-mono text-xs {ftrBandView(t.ftr).text}">
                            {fmtPct(t.ftr)}
                        </span>
                        <span class="text-right text-xs text-ink-mute">
                            {t.lastUsedAt ? new Date(t.lastUsedAt).toLocaleDateString() : '—'}
                        </span>
                    </div>
                {/each}
            {/if}
        </section>

        <!-- Extensions (skills / commands / agents) — historical, still shown. -->
        {#if data.tools.length > 0}
            <section>
                <h3 class="text-sm font-medium m-0 mb-3">Extensions</h3>
                <ul class="list-none m-0 p-0">
                    {#each data.tools as tool (tool.id)}
                        <li class="tool-row flex items-center gap-2.5 py-2 border-b border-paper-edge text-sm">
                            <span class="font-semibold flex-1">{tool.name}</span>
                            <span class="opacity-50 text-xs">{tool.kind}</span>
                            <span class="scope-badge text-xs px-1.5 py-px rounded-md font-mono"
                                  class:global={tool.scope === "global"}
                                  class:proj={tool.scope === "workspace"}>
                                [{tool.scope}]
                            </span>
                        </li>
                    {/each}
                </ul>
            </section>
        {/if}
        {/if}
    </div>
</Screen>

<style>
    .tool-row:last-child {
        border-bottom: none;
    }
    .scope-badge.global {
        background: var(--paper-mute);
        opacity: 0.7;
    }
    .scope-badge.proj {
        background: var(--accent-soft);
        color: var(--accent);
    }
    /* Enable-toggle pill — On is bright, Off is muted. Token-only. */
    .toggle {
        background: transparent;
        color: var(--ink-soft);
        border-color: var(--paper-edge);
    }
    .toggle.enabled {
        background: var(--success-soft);
        color: var(--success);
        border-color: var(--success-edge);
    }
    .toggle:disabled {
        opacity: 0.6;
        cursor: default;
    }
</style>
