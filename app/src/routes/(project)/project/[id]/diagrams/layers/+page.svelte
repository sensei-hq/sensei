<script lang="ts">
    import { LayersDiagram, ViolationsControl } from '@rokkit/graph';
    import { PageHeader } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { KINDS, LAYERS_FIELDS, LEVELS, LayersState } from '../layering-state.svelte.js';

    let { data } = $props();

    const view = new LayersState(senseiApi(appState.port), () => data.projectId);
    const question = DIAGRAM_VIEWS.find((v) => v.id === 'layers')!;

    $effect(() => {
        // Named so the dependency is explicit rather than incidental to
        // whatever the body happens to read first.
        void data.projectId;
        void view.level;
        void view.kinds;
        void view.load();
    });
</script>

<PageHeader kanji={question.kanji} title="Layers" description={question.question} />

<div class="flex flex-col gap-4 p-4" data-testid="layers-screen">
    <div class="flex flex-wrap items-center gap-4">
        <div class="flex items-center gap-2" role="group" aria-label="Level">
            {#each LEVELS as level (level)}
                <button
                    type="button"
                    class="px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                    class:bg-primary={view.level === level}
                    class:text-on-primary={view.level === level}
                    aria-pressed={view.level === level}
                    data-testid={`layers-level-${level}`}
                    onclick={() => view.setLevel(level)}
                >
                    {level}
                </button>
            {/each}
        </div>

        <div class="flex items-center gap-2" role="group" aria-label="Edge kinds">
            {#each KINDS as kind (kind)}
                <button
                    type="button"
                    class="px-2 py-1 text-xs rounded border border-paper-edge bg-transparent cursor-pointer"
                    class:bg-primary={view.kinds.includes(kind)}
                    class:text-on-primary={view.kinds.includes(kind)}
                    aria-pressed={view.kinds.includes(kind)}
                    data-testid={`layers-kind-${kind}`}
                    onclick={() => view.toggleKind(kind)}
                >
                    {kind}
                </button>
            {/each}
        </div>

        <ViolationsControl
            showEdges={view.showEdges}
            onchange={(v) => (view.showEdges = v)}
        />
    </div>

    {#if view.view === 'loading'}
        <p class="text-sm text-ink-mute" data-testid="layers-loading">
            Ranking the call graph — this reads every dependency in the project.
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
        <p class="text-sm text-ink-mute" data-testid="layers-empty">
            Nothing indexed for this project yet — no dependency carries two placed ends.
        </p>
    {:else}
        <!-- Three lines, and none of them is decoration. What the layering IS,
             what it found, and what the picture leaves out. -->
        <p class="text-sm text-ink-mute" data-testid="layers-source">{view.sourceNote}</p>
        <p class="text-sm text-ink" data-testid="layers-breaks">{view.breaksNote}</p>
        <p class="text-sm text-ink-mute" data-testid="layers-coverage">{view.coverageLine}</p>

        <div class="h-[70vh]" data-testid="layers-diagram">
            <LayersDiagram
                nodes={view.nodes}
                edges={view.edges}
                fields={LAYERS_FIELDS}
                bind:showEdges={view.showEdges}
                legend
                onselect={(id) => (view.selected = id)}
            />
        </div>

        {#if view.selectedNode}
            <dl
                class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm p-3 border border-paper-edge rounded"
                data-testid="layers-selection"
            >
                <dt class="text-ink-mute">Unit</dt>
                <dd data-testid="layers-selection-label">{view.selectedNode.label}</dd>
                <dt class="text-ink-mute">Layer</dt>
                <dd>{view.selectedNode.layer ?? 'unplaced'}</dd>
                <dt class="text-ink-mute">Package</dt>
                <dd>{view.selectedNode.group || '—'}</dd>
                <dt class="text-ink-mute">Files</dt>
                <dd>{view.selectedNode.files}</dd>
                <!-- A component of one is not a cycle, and saying "component 4"
                     with no size beside it invites reading every unit as one. -->
                <dt class="text-ink-mute">Cycle</dt>
                <dd data-testid="layers-selection-cycle">
                    {(view.selectedNode.componentSize ?? 1) > 1
                        ? `in a cycle of ${view.selectedNode.componentSize}`
                        : 'not in a cycle'}
                </dd>
            </dl>
        {/if}
    {/if}
</div>
