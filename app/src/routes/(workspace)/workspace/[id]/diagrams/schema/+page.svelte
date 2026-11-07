<script lang="ts">
    // Diagrams · Database · ER — dbd's viewer (user, 2026-11-02): the design's
    // Overview | Diagram | Entities, or one entity's page (a table, an object, an
    // enum) when one is selected, beside the sidebar on the right. rokkit's
    // @rokkit/graph/schema views draw every part; this page places them.
    //
    // On the ScreenShell (D87 §9) like every diagram: the design's name and counts
    // are the shell's title and stats band, its tabs and pages stay in the body,
    // and its sidebar is the shell's sidebar slot.
    import {
        designTabs,
        EntitiesView,
        EnumView,
        ObjectView,
        OverviewView,
        SchemaSidebar,
        TableView,
        toGraphInput,
    } from '@rokkit/graph/schema';
    import { Tabs } from '@rokkit/ui';
    import { messages } from '@rokkit/states';
    import type { InfoSlot, ShellStat } from '@rokkit/app';
    import { ShellScreen } from '@sensei/ui';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import type { Stat } from '@sensei/codemap';
    import { DIAGRAM_VIEWS, viewIdOf } from '../diagrams-nav.js';
    import { statMark } from '../diagram-marks.js';
    import { SchemaState, type DesignTabId } from '../schema-state.svelte.js';
    import ErCanvas from '../ErCanvas.svelte';

    let { data } = $props();

    const view = new SchemaState(senseiApi(appState.port), () => data.workspaceId);
    const diagram = DIAGRAM_VIEWS.find((v) => v.id === 'schema')!;
    const name = $derived(data.workspace?.name ?? 'Database');
    const entities = $derived(view.model ? toGraphInput(view.model, 'er') : null);

    $effect(() => {
        void data.workspaceId;
        void view.load();
    });

    const info = $derived<InfoSlot[]>([
        { label: 'Use it to', text: diagram.use },
        { label: 'How to read it', text: diagram.read },
        { label: 'What to do', text: diagram.todo },
    ]);

    // The old header's own tone names, as the shell's (#133).
    const TONE: Record<Stat['tone'], ShellStat['tone']> = {
        ink: 'ink', danger: 'danger', warning: 'warning', success: 'success', faint: 'muted',
    };
    const cardOf = (s: Stat): ShellStat => ({
        id: s.label.toLowerCase().replace(/\s+/g, '-'),
        title: s.label,
        value: s.value,
        tone: TONE[s.tone],
        // Kanji mode draws the mark, icon mode the icon (N5a, #133).
        kanji: statMark(s.label),
        icon: s.icon,
        attrs: { 'data-testid': `diagram-stat-${s.label}` },
    });
    /** The design's counts; null while it is read. */
    const cards = $derived(view.stats?.map(cardOf) ?? null);
</script>

{#snippet nothing()}{/snippet}

{#snippet sidePanel()}
    {#if view.model}
        {@const model = view.model}
        <aside class="min-w-0 min-h-0 overflow-auto p-4" data-testid="schema-sidebar">
            <SchemaSidebar {model} home={false} value={view.selected} side="right" onselect={(k) => view.select(k)} />
        </aside>
    {/if}
{/snippet}

{#snippet budgetBand()}
    <p class="m-0 px-4 py-2 font-mono text-xs text-ink-mute" data-testid="diagram-budget">{view.note}</p>
{/snippet}

<!-- A read, a failure and no design are steady states: pass no sidebar snippet,
     never an empty one (L0's rule from activity-logs). The old page's aside lived
     inside its `{:else if view.model}` branch, which sat after those three. -->
<ShellScreen
    viewId={viewIdOf(diagram)}
    mark={{ kanji: diagram.kanji, icon: diagram.icon }}
    title={diagram.label}
    description={diagram.shows}
    {info}
    stats={cards}
    search={false}
    labels={{ info: "What it's for", stats: 'Workspace numbers', filters: 'Controls', side: 'Side panel' }}
    toggleAttrs={{
        info: { 'data-testid': 'diagram-about-toggle' },
        stats: { 'data-testid': 'diagram-stats-toggle' },
        side: { 'data-testid': 'diagram-side-toggle' },
    }}
    bandAttrs={{ stats: { 'data-testid': 'diagram-stats' } }}
    footer={budgetBand}
    sidebar={view.view === 'design' ? sidePanel : undefined}>
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
            <!-- dbd's layout: the design scrolls on its own; the sidebar is the shell's. -->
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div class="min-w-0 min-h-0 overflow-auto flex flex-col gap-4 pr-6" data-testid="schema-layout"
                tabindex="0" role="region" aria-label="Database design">
                {#if view.page && view.selected}
                    <nav class="flex items-center gap-2 text-sm">
                        <button class="p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:underline"
                            onclick={() => view.select(null)} data-testid="schema-back">‹ {name}</button>
                    </nav>
                    <div data-testid="schema-page" data-page={view.page} data-details-fit>
                        {#if view.page === 'table'}
                            <TableView {model} entityKey={view.selected} onselect={(k) => view.select(k)} />
                        {:else if view.page === 'object'}
                            <ObjectView {model} entityKey={view.selected} onselect={(k) => view.select(k)} />
                        {:else}
                            <EnumView {model} entityKey={view.selected} onselect={(k) => view.select(k)} />
                        {/if}
                    </div>
                {:else}
                    <!-- Only the tabs: the design's name and counts are the shell's. -->
                    <div data-testid="schema-tabs">
                        <Tabs options={designTabs()} value={view.tab} name={messages.graph.designTabs}
                            tabPanel={nothing} onchange={(v) => (view.tab = String(v) as DesignTabId)} />
                    </div>
                    {#if view.tab === 'overview'}
                        <OverviewView {model} counts={false} />
                    {:else if view.tab === 'diagram'}
                        <!-- The canvas is the app's own (user, 2026-11-06): tables and views on
                             one picture, a press walking into a neighbourhood. -->
                        <div class="h-[70vh] min-w-0 shrink-0" data-testid="schema-diagram">
                            <ErCanvas {model} focus={view.focus} bind:density={view.density}
                                bind:edgeStyle={view.edgeStyle} onselect={(k) => view.focusOn(k)} />
                        </div>
                    {:else if entities}
                        <EntitiesView nodes={entities.nodes} edges={entities.edges} fields={entities.fields} />
                    {/if}
                {/if}
            </div>
        {/if}
    </div>
</ShellScreen>

<style>
    /* The Details tab is six columns wide — field, type, settings, default, references,
       notes — and on a narrow window it widened the page instead of scrolling inside it
       (user, 2026-11-06). rokkit draws the field table in its own scroll box; this gives
       that box a ceiling and the flex parents the floor they need to shrink. */
    [data-details-fit] :global([data-graph-design-page]),
    [data-details-fit] :global([data-graph-section]) {
        min-width: 0;
    }
    [data-details-fit] :global([data-graph-design-scroll]) {
        max-width: 100%;
        overflow-x: auto;
    }
    [data-details-fit] :global([data-graph-design-table]) {
        width: 100%;
    }
</style>
