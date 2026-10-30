<script lang="ts">
    import { DependencyDiagram, ErDiagram } from '@rokkit/graph';
    import { EntityView } from '@rokkit/graph/schema';
    import { Select, Tabs, Toggle, Tree } from '@rokkit/ui';
    import { DEFAULT_ICONS } from '@rokkit/graph/icons';
    import { Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { DiagramHeaderState } from '../diagram-header.svelte.js';
    import DiagramHeader from '../DiagramHeader.svelte';
    import type { ProxyItem } from '@rokkit/states';
    import { codeMapHost } from '../codemap-host.svelte.js';
    import { SchemaState, type SchemaScope } from '../schema-state.svelte.js';

    let { data } = $props();

    const view = new SchemaState(senseiApi(appState.port), () => data.workspaceId);
    const diagram = DIAGRAM_VIEWS.find((v) => v.id === 'schema')!;
    const about = new DiagramHeaderState();
    let entityTab = $state<'details' | 'diagram'>('details');
    const host = codeMapHost();

    $effect(() => {
        void data.workspaceId;
        void view.load();
    });

    const SCOPES = [
        { label: 'Tables & keys', value: 'er' },
        { label: 'Dependencies', value: 'dependencies' },
    ];
    // The list's schema dots take the diagram's colours for the mode in view.
    const colours = $derived(view.schemaColours(
        typeof document !== 'undefined' && document.documentElement.dataset.mode === 'dark' ? 'dark' : 'light',
    ));
    const input = $derived(view.graphInput);
</script>

{#snippet schemaGroup(proxy: ProxyItem)}
    <span class="inline-flex items-center gap-2 text-sm">
        <span class="inline-block w-2.5 h-2.5 rounded-full" style:background={colours.get(String(proxy.label))} aria-hidden="true"></span>
        {proxy.label}
        <span class="text-xs text-ink-mute">{proxy.get('badge')}</span>
    </span>
{/snippet}

{#snippet reachList(testid: string, sections: [string, string[]][])}
    <div class="flex flex-col gap-3 text-sm" data-testid={testid}>
        {#each sections as [label, items] (label)}
            <div class="flex flex-col gap-1">
                <span class="text-xs uppercase tracking-wide text-ink-mute">{label}</span>
                {#if items.length}
                    <ul class="m-0 p-0 list-none flex flex-col gap-0.5">
                        {#each items as item (item)}<li class="font-mono text-xs text-ink break-all">{item}</li>{/each}
                    </ul>
                {:else}
                    <span class="text-xs text-ink-faint">None</span>
                {/if}
            </div>
        {/each}
    </div>
{/snippet}

<Screen fill>
    {#snippet header()}
    <DiagramHeader view={diagram} {about} stats={host.stats}
        workspace={{ name: data.workspace.name, repos: data.workspace.repos_count ?? null }}
        hrefFor={(lens) => `/workspace/${data.workspaceId}/diagrams/${lens}`}>
        {#snippet controls()}
            {#if view.view === 'graph'}
            <div class="flex flex-wrap items-center gap-x-6 gap-y-3" data-testid="schema-controls">
                <input class="w-56 px-3 py-1.5 text-sm rounded border border-paper-edge bg-paper text-ink"
                    type="search" placeholder="Filter entities" aria-label="Filter entities"
                    bind:value={view.query} data-testid="schema-filter" />
                <div class="w-56" data-testid="schema-pick">
                    <Select items={view.schemaChoices} value={view.schema ?? ''} filterable filterPlaceholder="Type a schema" size="sm"
                        onchange={(value) => view.setSchema(value === '' ? null : String(value))} />
                </div>
                <div class="flex items-center gap-2">
                    <span class="text-xs uppercase tracking-wide text-ink-mute">Show</span>
                    <Toggle size="sm" label="Show" options={SCOPES} value={view.scope}
                        onchange={(v) => view.setScope(v as SchemaScope)} />
                </div>
                <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Entity types">
                    <span class="text-xs uppercase tracking-wide text-ink-mute">Types</span>
                    {#each view.kindCounts as k (k.kind)}
                        <button class="flex items-center gap-1.5 px-2 py-1 text-xs rounded border cursor-pointer"
                            class:border-paper-edge={k.shown} class:border-transparent={!k.shown}
                            class:bg-paper-mute={k.shown} class:bg-transparent={!k.shown}
                            class:text-ink={k.shown} class:text-ink-mute={!k.shown}
                            aria-pressed={k.shown} onclick={() => view.toggleKind(k.kind)} data-testid={`schema-kind-${k.kind}`}>
                            <span class={DEFAULT_ICONS[k.kind]} aria-hidden="true"></span>{k.kind.replace('_', ' ')} · {k.count}
                        </button>
                    {/each}
                </div>
            </div>
            {:else}
                <p class="m-0 text-sm text-ink-faint">The controls apply once the schema is read.</p>
            {/if}
        {/snippet}
    </DiagramHeader>
    {/snippet}

    <div class="flex-1 min-h-0 flex flex-col gap-4 p-4" data-testid="schema-screen">
        {#if view.view === 'loading'}
            <p class="text-sm text-ink-soft" data-testid="schema-loading">Reading the schema store.</p>
        {:else if view.view === 'error'}
            <div class="p-4 border border-warning rounded" data-testid="schema-error">
                <p class="text-sm text-warning">{view.failure}</p>
                <button
                    type="button"
                    class="mt-2 px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                    data-testid="schema-retry"
                    onclick={() => view.load()}
                >
                    Retry
                </button>
            </div>
        {:else if view.view === 'none'}
            <p class="text-sm text-ink-soft" data-testid="schema-none">
                No database structure is described in this workspace — no dbd design.yaml was found
                under its folders.
            </p>
        {:else}
            <!-- dbd's /diagram, its sidebar on the right: the diagram — or the entity
                 selected — beside every entity by schema. Each pane scrolls on its own. -->
            <div class="flex-1 min-h-0 grid gap-6" style={about.side ? 'grid-template-columns: minmax(0, 1fr) 300px;' : ''}>
                <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                <div class="min-w-0 min-h-0 overflow-auto flex flex-col gap-4" tabindex="0" role="region" aria-label="Diagram">
                    {#if view.selected}
                        <div class="flex flex-col gap-4" data-testid="schema-entity">
                            <nav class="flex items-center gap-2 text-sm">
                                <button class="p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:underline"
                                    onclick={() => view.select(null)} data-testid="schema-back">‹ Diagram</button>
                                <span class="text-ink-faint">/</span>
                                <span class="font-mono text-ink">{view.selected}</span>
                            </nav>
                            <div class="w-64"><Tabs options={[{ label: 'Details', value: 'details' }, { label: 'Diagram', value: 'diagram' }]} bind:value={entityTab} /></div>
                            {#if entityTab === 'diagram' && view.entityGraph}
                                {@const g = view.entityGraph}
                                <div class="h-[60vh]" data-testid="schema-entity-diagram">
                                    {#if view.selectedTable}
                                        <ErDiagram nodes={g.nodes} edges={g.edges} fields={g.fields} value={view.selected}
                                            onselect={(id) => view.select(id)} controls legend label="The table and what it is keyed to" />
                                    {:else}
                                        <DependencyDiagram nodes={g.nodes} edges={g.edges} fields={g.fields} value={view.selected}
                                            onselect={(id) => view.select(id)} controls legend label="The object and what it reads, writes and calls" />
                                    {/if}
                                </div>
                            {:else if view.selectedTable}
                                <EntityView nodes={input.nodes} edges={input.edges} fields={input.fields} value={view.selected} />
                                {#if view.reach}
                                    {@const reach = view.reach}
                                    {@render reachList('schema-reach', [['References', reach.references], ['Referenced by', reach.referencedBy], ['Read or written by', reach.dependents]])}
                                {/if}
                            {:else if view.selectedEntity && view.entityReach}
                                {@const e = view.selectedEntity}
                                {@const reach = view.entityReach}
                                <p class="m-0 text-sm text-ink-mute">{e.kind.replace('_', ' ')}</p>
                                {#if e.note}<p class="m-0 text-sm text-ink-soft">{e.note}</p>{/if}
                                {@render reachList('schema-entity-reach', [['Reads', reach.reads], ['Writes', reach.writes], ['Calls', reach.calls]])}
                            {/if}
                        </div>
                    {:else}
                        <ul class="flex flex-col list-none p-0 m-0 text-sm text-ink-soft" data-testid="schema-coverage">
                            {#each view.coverage as line (line)}
                                <li>{line}</li>
                            {/each}
                        </ul>
                        <div class="h-[70vh] shrink-0" data-testid="schema-diagram">
                            {#if view.scope === 'er'}
                                <ErDiagram nodes={input.nodes} edges={input.edges} fields={input.fields} value={view.selected}
                                    onselect={(id) => view.select(id)} controls legend label="Tables and their foreign keys" />
                            {:else}
                                <DependencyDiagram nodes={input.nodes} edges={input.edges} fields={input.fields} value={view.selected}
                                    onselect={(id) => view.select(id)} controls legend label="Database objects and what they read, write and call" />
                            {/if}
                        </div>
                    {/if}
                </div>
                {#if about.side}
                    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                    <aside class="min-w-0 min-h-0 overflow-auto pl-6 border-l border-paper-edge" tabindex="0" aria-label="Entities" data-testid="schema-sidebar">
                        <Tree items={view.sidebar} value={view.selected ?? undefined} groupContent={schemaGroup}
                            onselect={(value) => typeof value === 'string' && !value.startsWith('schema:') && view.select(value)} />
                    </aside>
                {/if}
            </div>
        {/if}
    </div>
</Screen>
