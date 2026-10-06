<script lang="ts">
    import { StructureDiagram, BundleControl } from '@rokkit/graph';
    import { PageHeader, ToggleChip } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import {
        KINDS,
        LEVELS,
        STRUCTURE_FIELDS,
        StructureState
    } from './structure-state.svelte.js';

    let { data } = $props();

    const view = new StructureState(senseiApi(appState.port), () => data.projectId);

    $effect(() => {
        // Named so the dependency is explicit rather than incidental to
        // whatever the body happens to read first.
        void data.projectId;
        void view.level;
        void view.kinds;
        void view.load();
    });
</script>

<PageHeader kanji="構" title="Structure" description={data.project?.name ?? data.projectId} />

<div class="flex flex-col gap-4 p-4" data-testid="structure-screen">
    <div class="flex flex-wrap items-center gap-4">
        <div class="flex items-center gap-2" role="group" aria-label="Level">
            {#each LEVELS as level (level)}
                <ToggleChip
                    label={level}
                    pressed={view.level === level}
                    testid={`structure-level-${level}`}
                    onpress={() => view.setLevel(level)}
                />
            {/each}
        </div>

        <div class="flex items-center gap-2" role="group" aria-label="Edge kinds">
            {#each KINDS as kind (kind)}
                <ToggleChip
                    label={kind}
                    size="xs"
                    pressed={view.kinds.includes(kind)}
                    testid={`structure-kind-${kind}`}
                    onpress={() => view.toggleKind(kind)}
                />
            {/each}
        </div>

        <BundleControl
            bundleTension={view.bundleTension}
            onchange={(v) => (view.bundleTension = v)}
        />
    </div>

    {#if view.view === 'loading'}
        <p class="text-sm text-ink-mute" data-testid="structure-loading">Loading structure…</p>
    {:else if view.view === 'error'}
        <div class="p-4 border border-warning rounded" data-testid="structure-error">
            <p class="text-sm text-warning">{view.failure}</p>
            <button
                type="button"
                class="mt-2 px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                data-testid="structure-retry"
                onclick={() => view.load()}
            >
                Retry
            </button>
        </div>
    {:else if view.view === 'empty'}
        <p class="text-sm text-ink-mute" data-testid="structure-empty">
            Nothing indexed for this project yet — no files carry a resolvable structure.
        </p>
    {:else}
        <p class="text-sm text-ink-mute" data-testid="structure-coverage">{view.coverageLine}</p>

        <div class="h-[70vh]" data-testid="structure-diagram">
            <StructureDiagram
                nodes={view.nodes}
                edges={view.edges}
                fields={STRUCTURE_FIELDS}
                bundleTension={view.bundleTension}
                sizeBy="symbols"
                sizeScale="log"
                legend
                onselect={(id) => (view.selected = id)}
            />
        </div>

        {#if view.selectedNode}
            <dl
                class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm p-3 border border-paper-edge rounded"
                data-testid="structure-selection"
            >
                <dt class="text-ink-mute">Node</dt>
                <dd data-testid="structure-selection-label">{view.selectedNode.label}</dd>
                <dt class="text-ink-mute">Module</dt>
                <dd>{view.selectedNode.module || '—'}</dd>
                <dt class="text-ink-mute">Package</dt>
                <dd>{view.selectedNode.package || '—'}</dd>
                <dt class="text-ink-mute">Symbols</dt>
                <dd>{view.selectedNode.symbols}</dd>
                <dt class="text-ink-mute">Degree</dt>
                <dd data-testid="structure-selection-degree">
                    {view.degree.inbound} in · {view.degree.outbound} out ·
                    {view.degree.crossing} crossing
                </dd>
            </dl>
        {/if}
    {/if}
</div>
