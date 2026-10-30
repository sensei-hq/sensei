<script lang="ts">
    import { DependencyDiagram, ErDiagram } from '@rokkit/graph';
    import { EntityView } from '@rokkit/graph/schema';
    import { List, Toolbar } from '@rokkit/ui';
    import { Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { DiagramHeaderState } from '../diagram-header.svelte.js';
    import DiagramHeader from '../DiagramHeader.svelte';
    import { codeMapHost } from '../codemap-host.svelte.js';
    import { SchemaState, type SchemaScope } from '../schema-state.svelte.js';

    let { data } = $props();

    const view = new SchemaState(senseiApi(appState.port), () => data.workspaceId);
    const diagram = DIAGRAM_VIEWS.find((v) => v.id === 'schema')!;
    const about = new DiagramHeaderState();
    const host = codeMapHost();

    $effect(() => {
        void data.workspaceId;
        void view.load();
    });

    const scopeItems = $derived(
        view.scopes.map((s) => ({
            label: s.label,
            value: s.value,
            active: view.scope === s.value,
            type: 'toggle' as const
        }))
    );
    const schemaItems = $derived(
        view.schemaOptions.map((o) => ({
            label: `${o.label} · ${o.tables}`,
            value: o.value ?? '',
            active: view.schema === o.value,
            type: 'toggle' as const
        }))
    );
    const input = $derived(view.graphInput);
</script>

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
            <div class="flex flex-wrap items-center gap-4">
                <Toolbar
                    label="Show"
                    width="fit"
                    compact
                    items={scopeItems}
                    onclick={(value) => view.setScope(value as SchemaScope)}
                />
                <Toolbar
                    label="Schema"
                    width="fit"
                    compact
                    items={schemaItems}
                    onclick={(value) => view.setSchema(value === '' ? null : String(value))}
                />
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
                            {#if view.selectedTable}
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
                    <aside class="min-w-0 min-h-0 overflow-auto" tabindex="0" aria-label="Entities" data-testid="schema-sidebar">
                        <List items={view.sidebar} collapsible value={view.selected ?? undefined} label="Entities"
                            onselect={(value) => typeof value === 'string' && !value.startsWith('schema:') && view.select(value)} />
                    </aside>
                {/if}
            </div>
        {/if}
    </div>
</Screen>
