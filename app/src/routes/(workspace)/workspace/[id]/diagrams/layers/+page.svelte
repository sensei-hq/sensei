<script lang="ts">
    import { LayersDiagram } from '@rokkit/graph';
    import { Toolbar } from '@rokkit/ui';
    import { PageHeader, Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { LAYERS_FIELDS, LayersState } from '../layering-state.svelte.js';
    import type { LayeringLevel } from '$lib/types.js';

    let { data } = $props();

    const view = new LayersState(senseiApi(appState.port), () => data.workspaceId);
    const question = DIAGRAM_VIEWS.find((v) => v.id === 'layers')!;

    $effect(() => {
        // Named so the dependency is explicit rather than incidental to
        // whatever the body happens to read first.
        void data.workspaceId;
        void view.level;
        void view.kinds;
        void view.load();
    });
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji={question.kanji} icon={question.icon} title="Layers" description={question.question} />
    {/snippet}

    <div class="flex flex-col gap-4 p-4" data-testid="layers-screen">
        <div class="flex flex-wrap items-center gap-4">
            <Toolbar
                label="Level"
                width="fit"
                compact
                items={view.levelItems}
                onclick={(value) => view.setLevel(value as LayeringLevel)}
            />

            <Toolbar
                label="Edge kinds"
                width="fit"
                compact
                items={view.kindItems}
                onclick={(value) => view.toggleKind(value as string)}
            />

        </div>

        {#if view.view === 'loading'}
            <p class="text-sm text-ink-soft" data-testid="layers-loading">
                Ranking the call graph — this reads every dependency in the workspace.
            </p>
        {:else if view.view === 'error'}
            <div class="p-4 border border-warning rounded" data-testid="layers-error">
                <p class="text-sm text-warning">{view.failure}</p>
                <button
                    type="button"
                    class="mt-2 px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                    data-testid="layers-retry"
                    onclick={() => view.load()}
                >
                    Retry
                </button>
            </div>
        {:else if view.view === 'empty'}
            <p class="text-sm text-ink-soft" data-testid="layers-empty">
                Nothing indexed for this workspace yet — no dependency carries two placed ends.
            </p>
        {:else}
            <!-- Three lines, and none of them is decoration. What the layering IS,
                 what it found, and what the picture leaves out. -->
            <p class="text-sm text-ink-soft" data-testid="layers-source">{view.sourceNote}</p>
            <p class="text-sm text-ink" data-testid="layers-breaks">{view.breaksNote}</p>
            <p class="text-sm text-ink-soft" data-testid="layers-coverage">{view.coverageLine}</p>

            <div class="h-[70vh]" data-testid="layers-diagram">
                <LayersDiagram
                    nodes={view.nodes}
                    edges={view.edges}
                    fields={LAYERS_FIELDS}
                    bind:showEdges={view.showEdges}
                    controls
                    legend
                    onselect={(id) => (view.selected = id)}
                />
            </div>

            {#if view.selectedNode}
                <dl
                    class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm p-3 border border-paper-edge rounded"
                    data-testid="layers-selection"
                >
                    <dt class="text-ink-soft">Unit</dt>
                    <dd data-testid="layers-selection-label">{view.selectedNode.label}</dd>
                    <dt class="text-ink-soft">Layer</dt>
                    <dd>{view.selectedNode.layer ?? 'unplaced'}</dd>
                    <dt class="text-ink-soft">Package</dt>
                    <dd>{view.selectedNode.group || '—'}</dd>
                    <dt class="text-ink-soft">Files</dt>
                    <dd>{view.selectedNode.files}</dd>
                    <!-- A component of one is not a cycle, and saying "component 4"
                         with no size beside it invites reading every unit as one. -->
                    <dt class="text-ink-soft">Cycle</dt>
                    <dd data-testid="layers-selection-cycle">
                        {(view.selectedNode.componentSize ?? 1) > 1
                            ? `in a cycle of ${view.selectedNode.componentSize}`
                            : 'not in a cycle'}
                    </dd>
                </dl>
            {/if}
        {/if}
    </div>
</Screen>
