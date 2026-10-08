<script lang="ts">
    import { Graph, GraphState } from '@rokkit/graph';
    import { Toolbar } from '@rokkit/ui';
    import { PageHeader } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { ROOT_LABEL, WorldState } from '../world-state.svelte.js';
    import type { WorldGroupBy, WorldShadeBy } from '$lib/types.js';

    let { data } = $props();

    const view = new WorldState(senseiApi(appState.port), () => data.projectId);
    const question = DIAGRAM_VIEWS.find((v) => v.id === 'world')!;

    $effect(() => {
        void data.projectId;
        void view.groupBy;
        void view.load();
    });

    // ONE `GraphState`, UPDATED — not a new `Graph` per change. `shadeBy`,
    // `focusPath` and `levels` live on `GraphStateConfig`, so they can only be
    // set through a shared state, and a fresh one per toggle would re-run the
    // layout and lose the reader's zoom.
    //
    // `update` REPLACES the config, so it is always handed the whole of it
    // (`view.graphConfig`). Handing it nodes/shade/focus alone reverted the
    // layout to the default and drew the circle pack as a stack of cards (#219).
    // svelte-ignore state_referenced_locally
    const graph = new GraphState(view.graphConfig);

    $effect(() => {
        graph.update(view.graphConfig);
    });

    // The toolbars are data, built here rather than in state because they are
    // static copy with a live `active` — see `layering-state` for the case where
    // the items genuinely derive from the payload.
    const groupItems = $derived(
        view.groupings.map((g) => ({
            label: g.label,
            value: g.value,
            active: view.groupBy === g.value,
            type: 'toggle' as const
        }))
    );
    const shadeItems = $derived(
        view.shades.map((s) => ({
            label: s.label,
            value: s.value,
            active: view.shadeBy === s.value,
            type: 'toggle' as const
        }))
    );
</script>

<PageHeader kanji={question.kanji} title="World" description={question.question} />

<div class="flex flex-col gap-4 p-4" data-testid="world-screen">
    <div class="flex flex-wrap items-center gap-4">
        <Toolbar
            label="Group first by"
            width="fit"
            compact
            items={groupItems}
            onclick={(value) => view.setGroupBy(value as WorldGroupBy)}
        />
        <Toolbar
            label="Shade by"
            width="fit"
            compact
            items={shadeItems}
            onclick={(value) => view.setShadeBy(value as WorldShadeBy)}
        />
    </div>

    {#if view.view === 'loading'}
        <p class="text-sm text-ink-mute" data-testid="world-loading">
            Counting every indexed declaration — this reads the whole corpus, not one project.
        </p>
    {:else if view.view === 'error'}
        <div class="p-4 border border-warning rounded" data-testid="world-error">
            <p class="text-sm text-warning">{view.failure}</p>
            <button
                type="button"
                class="mt-2 px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                data-testid="world-retry"
                onclick={() => view.load()}
            >
                Retry
            </button>
        </div>
    {:else if view.view === 'empty'}
        <p class="text-sm text-ink-mute" data-testid="world-empty">
            Nothing indexed on this machine yet — no declaration has been walked.
        </p>
    {:else}
        <!-- THE PICTURE IS WIDER THAN THE PROJECT. Said first and unconditionally,
             because a view of every project silently reads as a view of yours. -->
        <p class="text-sm text-ink-mute" data-testid="world-viewing">{view.viewingNote}</p>
        <p class="text-sm text-ink-mute" data-testid="world-totals">{view.totalsLine}</p>

        <div class="flex flex-col gap-4 md:flex-row">
            <div class="h-[70vh] flex-1" data-testid="world-diagram">
                <!-- No `ondrill` props: with a supplied `state` Graph does not wire
                     them. The drill callbacks travel in `view.graphConfig`. -->
                <Graph
                    state={graph}
                    zoomable
                    label="All indexed code, nested by what contains it"
                />
            </div>

            <!-- IN VIEW. The panel answers for the circle you are standing in,
                 which at the root is the whole corpus. -->
            <aside
                class="w-full md:w-80 flex flex-col gap-3 p-3 border border-paper-edge rounded"
                data-testid="world-inview"
            >
                <div>
                    <p class="text-xs uppercase tracking-wide text-ink-faint m-0">In view</p>
                    <p class="text-base text-ink m-0" data-testid="world-inview-label">
                        {view.inView.label}
                    </p>
                    <p class="text-sm text-ink-mute m-0" data-testid="world-inview-weight">
                        {view.inView.weight.toLocaleString()} declarations
                    </p>
                </div>

                {#if view.focusPath.length > 0}
                    <button
                        type="button"
                        class="self-start px-2 py-1 text-xs rounded border border-paper-edge bg-transparent cursor-pointer"
                        data-testid="world-drill-up"
                        onclick={() => view.drillUp()}
                    >
                        ← out to {view.focusPath.length === 1
                            ? ROOT_LABEL
                            : view.focusPath[view.focusPath.length - 2]}
                    </button>
                {/if}

                <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm m-0">
                    <dt class="text-ink-mute">Documented</dt>
                    <dd class="m-0" data-testid="world-measure-documented">
                        {view.measureNote('documentedShare')}
                    </dd>
                    <dt class="text-ink-mute">Tests</dt>
                    <dd class="m-0">{view.measureNote('testShare')}</dd>
                    <dt class="text-ink-mute">Unresolved</dt>
                    <dd class="m-0" data-testid="world-measure-unresolved">
                        {view.measureNote('unresolvedShare')}
                    </dd>
                </dl>

                {#if view.largestInside.length > 0}
                    <div>
                        <p class="text-xs uppercase tracking-wide text-ink-faint m-0">
                            Largest inside
                        </p>
                        <ul class="flex flex-col list-none p-0 m-0" data-testid="world-largest">
                            {#each view.largestInside.slice(0, 8) as child (child.id)}
                                <li class="flex justify-between gap-3 text-sm">
                                    <button
                                        type="button"
                                        class="text-left text-ink bg-transparent border-0 p-0 cursor-pointer underline-offset-2 hover:underline"
                                        onclick={() => view.drillTo(child.path)}
                                    >
                                        {child.label}
                                    </button>
                                    <span class="text-ink-mute tabular-nums">
                                        {child.weight.toLocaleString()}
                                    </span>
                                </li>
                            {/each}
                        </ul>
                    </div>
                {/if}
            </aside>
        </div>
    {/if}
</div>
