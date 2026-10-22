<script lang="ts">
    import { DependencyDiagram, ErDiagram } from '@rokkit/graph';
    import { EntityView } from '@rokkit/graph/schema';
    import { Toolbar } from '@rokkit/ui';
    import { PageHeader, Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { SchemaState, type SchemaScope } from '../schema-state.svelte.js';

    let { data } = $props();

    const view = new SchemaState(senseiApi(appState.port), () => data.workspaceId);
    const question = DIAGRAM_VIEWS.find((v) => v.id === 'schema')!;

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
    <PageHeader kanji={question.kanji} icon={question.icon} title="Schema" description={question.question} />
    {/snippet}

    <div class="flex flex-col gap-4 p-4" data-testid="schema-screen">
        {#if view.view === 'loading'}
            <p class="text-sm text-ink-mute" data-testid="schema-loading">Reading the schema store.</p>
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
            <p class="text-sm text-ink-mute" data-testid="schema-none">
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

            <ul class="flex flex-col list-none p-0 m-0 text-sm text-ink-mute" data-testid="schema-coverage">
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
                    class="w-full md:w-96 flex flex-col gap-3 p-3 border border-paper-edge rounded overflow-auto max-h-[70vh]"
                    data-testid="schema-entity"
                >
                    {#if view.selectedTable}
                        <EntityView
                            nodes={input.nodes}
                            edges={input.edges}
                            fields={input.fields}
                            value={view.selected}
                        />
                    {:else}
                        <p class="text-sm text-ink-mute m-0">
                            Select a table to see its columns, keys and notes.
                        </p>
                    {/if}
                </aside>
            </div>
        {/if}
    </div>
</Screen>
