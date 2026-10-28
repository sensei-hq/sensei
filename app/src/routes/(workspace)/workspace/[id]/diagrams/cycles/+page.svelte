<script lang="ts">
    import { Toolbar } from '@rokkit/ui';
    import { PageHeader, Screen } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { LayersState } from '../layering-state.svelte.js';
    import type { LayeringLevel } from '$lib/types.js';

    let { data } = $props();

    const view = new LayersState(senseiApi(appState.port), () => data.workspaceId);
    const question = DIAGRAM_VIEWS.find((v) => v.id === 'cycles')!;

    $effect(() => {
        void data.workspaceId;
        void view.level;
        void view.kinds;
        void view.load();
    });
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji={question.kanji} icon={question.icon} title="Cycles" description={question.question} />
    {/snippet}

    <div class="flex flex-col gap-4 p-4" data-testid="cycles-screen">
        <Toolbar
            label="Level"
            width="fit"
            compact
            items={view.levelItems}
            onclick={(value) => view.setLevel(value as LayeringLevel)}
        />

        {#if view.view === 'loading'}
            <p class="text-sm text-ink-soft" data-testid="cycles-loading">
                Finding the strongly-connected components — this reads every dependency in the workspace.
            </p>
        {:else if view.view === 'error'}
            <div class="p-4 border border-warning rounded" data-testid="cycles-error">
                <p class="text-sm text-warning">{view.failure}</p>
                <button
                    type="button"
                    class="mt-2 px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                    data-testid="cycles-retry"
                    onclick={() => view.load()}
                >
                    Retry
                </button>
            </div>
        {:else if view.view === 'empty'}
            <p class="text-sm text-ink-soft" data-testid="cycles-empty">
                Nothing indexed for this workspace yet — no dependency carries two placed ends.
            </p>
        {:else}
            <p class="text-sm text-ink-soft" data-testid="cycles-coverage">{view.coverageLine}</p>

            <!-- NO CYCLES IS A FINDING. An empty canvas under no caption reads as a
                 screen that failed to load, which is the one misreading that
                 matters on a screen whose whole subject is absence. -->
            {#if view.cycles.length === 0}
                <p class="text-sm text-ink" data-testid="cycles-none">
                    No unit depends on itself through another. Every dependency in this workspace
                    points down the layering.
                </p>
            {:else}
                <p class="text-sm text-ink" data-testid="cycles-summary">
                    {view.cycles.length}
                    {view.cycles.length === 1 ? 'cycle' : 'cycles'}, deepest-depended-on first.
                </p>

                <ul class="flex flex-col gap-4 list-none p-0 m-0" data-testid="cycles-list">
                    {#each view.cycles as cycle (cycle.component)}
                        <li
                            class="p-4 border border-paper-edge rounded"
                            data-testid={`cycle-${cycle.component}`}
                        >
                            <p class="text-xs uppercase tracking-wide text-ink-faint m-0">
                                Cycle · {cycle.members.length} units · layer {cycle.layer}
                            </p>

                            <ul class="flex flex-col list-none p-0 mt-2 mb-0">
                                {#each cycle.members as member (member)}
                                    <li class="text-sm text-ink">{member}</li>
                                {/each}
                            </ul>

                            <!-- One line per inner dependency, heaviest first as the
                                 daemon ships them, with the cut called out. The cut
                                 is marked by VALUE and not by position: it is the
                                 feedback order's choice, not the first row. -->
                            <ul class="flex flex-col list-none p-0 mt-3 mb-0">
                                {#each cycle.inner as dep (`${dep.source}→${dep.target}`)}
                                    {@const isCut =
                                        cycle.cut?.source === dep.source &&
                                        cycle.cut?.target === dep.target}
                                    <li
                                        class="text-sm"
                                        class:text-accent={isCut}
                                        class:text-ink-soft={!isCut}
                                    >
                                        {isCut ? 'cut · ' : ''}{dep.source} → {dep.target} (×{dep.occurrences})
                                    </li>
                                {/each}
                            </ul>

                            <p class="text-sm text-ink mt-3 mb-0" data-testid={`cycle-cut-${cycle.component}`}>
                                {view.cutLine(cycle)}
                            </p>
                        </li>
                    {/each}
                </ul>
            {/if}

            <!-- A unit depending on ITSELF is not a cycle between units, and folding
                 the two together would report a module as circular because two of
                 its own files refer to each other. -->
            {#if view.selfDependencies.length > 0}
                <section data-testid="cycles-self">
                    <h2 class="text-sm font-semibold text-ink m-0">Depends on itself</h2>
                    <p class="text-sm text-ink-soft mt-1 mb-2">
                        At module grain this is two of a module's own files referring to each other —
                        a fact about the module, not a loop between units.
                    </p>
                    <ul class="flex flex-col list-none p-0 m-0">
                        {#each view.selfDependencies as dep (dep.source)}
                            <li class="text-sm text-ink-soft">{dep.source} (×{dep.occurrences})</li>
                        {/each}
                    </ul>
                </section>
            {/if}
        {/if}
    </div>
</Screen>
