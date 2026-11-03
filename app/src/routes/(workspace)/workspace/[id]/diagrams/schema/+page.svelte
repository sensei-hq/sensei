<script lang="ts">
    // Diagrams · Database · ER — dbd's viewer (user, 2026-11-02): the design's
    // Overview | Diagram | Entities, or one entity's page (a table, an object, an
    // enum) when one is selected, beside the sidebar on the right. rokkit's
    // @rokkit/graph/schema views draw every part; this page places them.
    import {
        ContentHeader,
        DiagramView,
        EntitiesView,
        EnumView,
        ObjectView,
        OverviewView,
        SchemaSidebar,
        TableView,
        toGraphInput,
    } from '@rokkit/graph/schema';
    import { Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { DiagramHeaderState } from '../diagram-header.svelte.js';
    import DiagramHeader from '../DiagramHeader.svelte';
    import { SchemaState, type DesignTabId } from '../schema-state.svelte.js';

    let { data } = $props();

    const view = new SchemaState(senseiApi(appState.port), () => data.workspaceId);
    const diagram = DIAGRAM_VIEWS.find((v) => v.id === 'schema')!;
    const about = new DiagramHeaderState();
    const name = $derived(data.workspace?.name ?? 'Database');
    const entities = $derived(view.model ? toGraphInput(view.model, 'er') : null);

    $effect(() => {
        void data.workspaceId;
        void view.load();
    });
</script>

<Screen fill>
    {#snippet header()}
    <DiagramHeader view={diagram} {about} stats={view.stats} budget={view.note}
        workspace={{ name: data.workspace.name, repos: data.workspace.repos_count ?? null }}
        hrefFor={(lens) => `/workspace/${data.workspaceId}/diagrams/${lens}`} />
    {/snippet}

    <div class="flex-1 min-h-0 flex flex-col p-4" data-testid="schema-screen">
        {#if view.view === 'loading'}
            <p class="text-sm text-ink-soft" data-testid="schema-loading">Reading the schema store.</p>
        {:else if view.view === 'error'}
            <div class="p-4 border border-warning rounded" data-testid="schema-error">
                <p class="text-sm text-warning">{view.failure}</p>
                <button type="button" class="mt-2 px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                    data-testid="schema-retry" onclick={() => view.load()}>Retry</button>
            </div>
        {:else if view.view === 'none'}
            <p class="text-sm text-ink-soft" data-testid="schema-none">
                No database structure is described in this workspace — no dbd design.yaml was found
                under its folders.
            </p>
        {:else if view.model}
            {@const model = view.model}
            <!-- dbd's layout with the sidebar on the right; each pane scrolls on its own. -->
            <div class="flex-1 min-h-0 grid gap-0" data-testid="schema-layout"
                style={about.side ? 'grid-template-columns: minmax(0, 1fr) 280px;' : ''}>
                <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                <div class="min-w-0 min-h-0 overflow-auto flex flex-col gap-4 pr-6" tabindex="0" role="region" aria-label="Database design">
                    {#if view.page && view.selected}
                        <nav class="flex items-center gap-2 text-sm">
                            <button class="p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:underline"
                                onclick={() => view.select(null)} data-testid="schema-back">‹ {name}</button>
                        </nav>
                        <div data-testid="schema-page" data-page={view.page}>
                            {#if view.page === 'table'}
                                <TableView {model} entityKey={view.selected} onselect={(k) => view.select(k)} />
                            {:else if view.page === 'object'}
                                <ObjectView {model} entityKey={view.selected} onselect={(k) => view.select(k)} />
                            {:else}
                                <EnumView {model} entityKey={view.selected} onselect={(k) => view.select(k)} />
                            {/if}
                        </div>
                    {:else}
                        <ContentHeader {model} {name} value={view.tab}
                            onchange={(v) => (view.tab = v as DesignTabId)} />
                        {#if view.tab === 'overview'}
                            <OverviewView {model} counts={false} />
                        {:else if view.tab === 'diagram'}
                            <div class="h-[70vh] shrink-0" data-testid="schema-diagram">
                                <DiagramView {model} value={view.selected} bind:density={view.density}
                                    bind:edgeStyle={view.edgeStyle} label="Tables and their keys"
                                    onselect={(k) => view.select(k)} />
                            </div>
                        {:else if entities}
                            <EntitiesView nodes={entities.nodes} edges={entities.edges} fields={entities.fields} />
                        {/if}
                    {/if}
                </div>
                {#if about.side}
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <aside class="min-w-0 min-h-0 overflow-auto" tabindex="0" aria-label="Entities" data-testid="schema-sidebar">
                        <SchemaSidebar {model} home={false} value={view.selected} side="right" onselect={(k) => view.select(k)} />
                    </aside>
                {/if}
            </div>
        {/if}
    </div>
</Screen>
