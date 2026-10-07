<script lang="ts">
    import { Neighborhood } from '@rokkit/graph';
    import { Toolbar } from '@rokkit/ui';
    import { replaceState } from '$app/navigation';
    import { page } from '$app/state';
    import { PageHeader } from '$lib/components';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { DIAGRAM_VIEWS } from '../diagrams-nav.js';
    import { MAX_DEPTH, MIN_DEPTH, NeighbourhoodState } from '../neighbourhood-state.svelte.js';

    let { data } = $props();

    const view = new NeighbourhoodState(senseiApi(appState.port), () => data.projectId);
    const question = DIAGRAM_VIEWS.find((v) => v.id === 'neighbourhood')!;

    // The URL's focus, once. After that the page owns it and writes it back, so a
    // walk can be linked to and survives a reload.
    // svelte-ignore state_referenced_locally
    if (data.focus) void view.setFocus(data.focus);

    $effect(() => {
        const focus = view.focus;
        if (!focus || page.url.searchParams.get('focus') === focus) return;
        const url = new URL(page.url);
        url.searchParams.set('focus', focus);
        replaceState(url, {});
    });

    const depthItems = $derived(
        Array.from({ length: MAX_DEPTH - MIN_DEPTH + 1 }, (_, i) => i + MIN_DEPTH).map((d) => ({
            label: d === 1 ? '1 hop' : `${d} hops`,
            value: String(d),
            active: view.depth === d,
            type: 'toggle' as const
        }))
    );
</script>

<PageHeader kanji={question.kanji} title="Neighbourhood" description={question.question} />

<div class="flex flex-col gap-4 p-4" data-testid="neighbourhood-screen">
    <div class="flex flex-wrap items-start gap-4">
        <div class="flex flex-col gap-2 w-full md:w-96">
            <label class="text-xs uppercase tracking-wide text-ink-faint" for="neighbourhood-search">
                Centre on a function
            </label>
            <input
                id="neighbourhood-search"
                type="search"
                class="px-3 py-1 text-sm rounded border border-paper-edge bg-paper text-ink"
                placeholder="Name or signature, two characters or more"
                data-testid="neighbourhood-search"
                value={view.query}
                oninput={(e) => view.search(e.currentTarget.value)}
            />
            {#if view.searchFailure}
                <p class="text-sm text-warning m-0" data-testid="neighbourhood-search-error">
                    {view.searchFailure}
                </p>
            {:else if view.matches.length > 0}
                <ul
                    class="flex flex-col list-none p-0 m-0 max-h-64 overflow-auto border border-paper-edge rounded"
                    data-testid="neighbourhood-matches"
                >
                    {#each view.matches as m (m.id)}
                        <li>
                            <button
                                type="button"
                                class="w-full text-left px-3 py-1 text-sm bg-transparent border-0 cursor-pointer hover:bg-paper-soft"
                                onclick={() => view.setFocus(m.id)}
                            >
                                <span class="text-ink">{m.name}</span>
                                <span class="text-ink-mute">
                                    · {m.file_path}{m.line_start ? `:${m.line_start}` : ''}
                                </span>
                            </button>
                        </li>
                    {/each}
                </ul>
            {:else if view.query.trim().length >= 2}
                <p class="text-sm text-ink-mute m-0" data-testid="neighbourhood-no-matches">
                    No function or method in this project matches “{view.query.trim()}”.
                </p>
            {/if}
        </div>

        {#if view.focus}
            <Toolbar
                label="Depth"
                width="fit"
                compact
                items={depthItems}
                onclick={(value) => view.setDepth(Number(value))}
            />
        {/if}
    </div>

    {#if view.view === 'pick'}
        <p class="text-sm text-ink-mute" data-testid="neighbourhood-pick">
            Pick a function to see what calls it and what it calls. There is no default — a picture
            of a symbol you did not choose answers a question nobody asked.
        </p>
    {:else if view.view === 'loading'}
        <p class="text-sm text-ink-mute" data-testid="neighbourhood-loading">
            Walking {view.depth === 1 ? 'one ring' : `${view.depth} rings`} out…
        </p>
    {:else if view.view === 'error'}
        <div class="p-4 border border-warning rounded" data-testid="neighbourhood-error">
            <p class="text-sm text-warning">{view.failure}</p>
            <button
                type="button"
                class="mt-2 px-3 py-1 text-sm rounded border border-paper-edge bg-transparent cursor-pointer"
                data-testid="neighbourhood-retry"
                onclick={() => view.load()}
            >
                Retry
            </button>
        </div>
    {:else}
        {#if view.focusCard}
            <p class="text-sm text-ink m-0" data-testid="neighbourhood-focus">
                <span class="font-mono">{view.focusCard.label}</span>
                <span class="text-ink-mute">
                    · {view.focusCard.group ?? 'no package'}{view.focusCard.module
                        ? ` / ${view.focusCard.module}`
                        : ''} · {view.focusCard.file ?? 'no file'}{view.focusCard.line
                        ? `:${view.focusCard.line}`
                        : ''}
                </span>
            </p>
        {/if}

        <ul class="flex flex-col gap-1 list-none p-0 m-0" data-testid="neighbourhood-coverage">
            {#each view.coverageNotes as note (note)}
                <li class="text-sm text-ink-mute">{note}</li>
            {/each}
        </ul>

        {#if view.refusal}
            <p class="text-sm text-warning m-0" data-testid="neighbourhood-refusal">{view.refusal}</p>
        {/if}

        {#if view.view === 'alone'}
            <p class="text-sm text-ink-mute" data-testid="neighbourhood-alone">
                Nothing in this project calls it, and it calls nothing the graph could place.
            </p>
        {:else}
            <div class="h-[70vh]" data-testid="neighbourhood-diagram">
                <Neighborhood
                    nodes={view.nodes}
                    edges={view.edges}
                    focus={view.focus}
                    depth={view.depth}
                    maxDepth={MAX_DEPTH}
                    legend
                    label={`What calls ${view.focusCard?.label ?? 'this'}, and what it calls`}
                    onselect={(id) => {
                        if (id) void view.recentre(id);
                    }}
                />
            </div>
        {/if}
    {/if}
</div>
