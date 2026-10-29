<script lang="ts">
    import { DependencyDiagram, ErDiagram } from '@rokkit/graph';
    import { EntityView } from '@rokkit/graph/schema';
    import { Toolbar } from '@rokkit/ui';
    import { Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { DiagramHeaderState } from '../diagram-header.svelte.js';
    import DiagramHeader from '../DiagramHeader.svelte';
    import { SchemaState, type SchemaScope } from '../schema-state.svelte.js';

    let { data } = $props();

    const view = new SchemaState(senseiApi(appState.port), () => data.workspaceId);
    const diagram = DIAGRAM_VIEWS.find((v) => v.id === 'schema')!;
    const about = new DiagramHeaderState();

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

<Screen>
    {#snippet header()}
    <DiagramHeader view={diagram} {about} />
    {/snippet}

    <div class="flex flex-col gap-4 p-4" data-testid="schema-screen">
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

            <ul class="flex flex-col list-none p-0 m-0 text-sm text-ink-soft" data-testid="schema-coverage">
                {#each view.coverage as line (line)}
                    <li>{line}</li>
                {/each}
            </ul>

            <div class="flex flex-col gap-4 md:flex-row">
                <div class="h-[70vh] flex-1" data-testid="schema-diagram">
                    {#if view.scope === 'er'}
                        <ErDiagram
                            nodes={input.nodes}
                            edges={input.edges}
                            fields={input.fields}
                            value={view.selected}
                            onselect={(id) => view.select(id)}
                            controls
                            legend
                            label="Tables and their foreign keys"
                        />
                    {:else}
                        <DependencyDiagram
                            nodes={input.nodes}
                            edges={input.edges}
                            fields={input.fields}
                            value={view.selected}
                            onselect={(id) => view.select(id)}
                            controls
                            legend
                            label="Database objects and what they read, write and call"
                        />
                    {/if}
                </div>

                <aside
                    class="w-full md:w-[300px] flex flex-col gap-3 p-3 border border-paper-edge rounded overflow-auto max-h-[70vh]"
                    data-testid="schema-entity"
                >
                    {#if view.selectedTable}
                        <EntityView
                            nodes={input.nodes}
                            edges={input.edges}
                            fields={input.fields}
                            value={view.selected}
                        />
                        {#if view.reach}
                            {@const reach = view.reach}
                            <div class="flex flex-col gap-2 text-xs" data-testid="schema-reach">
                                {#each [['References', reach.references], ['Referenced by', reach.referencedBy], ['Read or written by', reach.dependents]] as [label, items] (label)}
                                    <div class="flex flex-col gap-1">
                                        <span class="uppercase tracking-wide text-ink-mute">{label}</span>
                                        {#if items.length}
                                            <ul class="m-0 p-0 list-none flex flex-col gap-0.5">
                                                {#each items as item (item)}<li class="font-mono text-ink break-all">{item}</li>{/each}
                                            </ul>
                                        {:else}
                                            <span class="text-ink-faint">None</span>
                                        {/if}
                                    </div>
                                {/each}
                            </div>
                        {/if}
                    {:else}
                        <p class="text-sm text-ink-soft m-0">
                            Select a table to see its columns, keys and notes.
                        </p>
                    {/if}
                </aside>
            </div>
        {/if}
    </div>
</Screen>
