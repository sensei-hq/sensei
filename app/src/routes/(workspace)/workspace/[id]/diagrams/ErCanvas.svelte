<script lang="ts">
    // The Database · ER canvas (user, 2026-11-06): one picture of the design — tables
    // and views — and a press that walks into what the pressed object touches. A
    // table's neighbours are its references; a view's are what it reads and what reads
    // it, which is every dependency edge in or out. The graph is the app's own
    // (`er-graph.ts`), because rokkit's ER scope draws tables and foreign keys only and
    // its dependency scope draws no references at all.
    import { ErDiagram, Neighborhood } from '@rokkit/graph';
    import type { Density, EdgeStyle } from '@rokkit/graph';
    import type { DesignModel } from '@rokkit/graph/schema';
    import { toErGraph } from './er-graph.js';

    let {
        model,
        focus = null,
        density = $bindable('keys'),
        edgeStyle = $bindable('curved'),
        onselect = undefined,
    }: {
        model: DesignModel;
        /** The object whose neighbourhood is drawn; null is the whole design. */
        focus?: string | null;
        density?: Density;
        edgeStyle?: EdgeStyle;
        /** A press: an object to centre on, or null for the design. */
        onselect?: (key: string | null) => void;
    } = $props();

    const graph = $derived(toErGraph(model));
    const name = $derived(focus ?? '');
</script>

<div class="min-w-0 min-h-0 h-full flex flex-col gap-2" data-testid="er-canvas" data-focus={focus ?? ''}>
    {#if focus}
        <div class="flex min-h-0 flex-1 flex-col gap-2" data-testid="er-neighbourhood" data-centre={focus}>
            <nav class="flex items-center gap-2 text-sm">
                <button
                    type="button"
                    class="p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:underline"
                    data-testid="er-design-back"
                    onclick={() => onselect?.(null)}>‹ the design</button
                >
                <span class="text-xs text-ink-faint">what {name} touches, and what touches it</span>
            </nav>
            <div class="min-h-0 flex-1">
                <Neighborhood
                    nodes={graph.nodes}
                    edges={graph.edges}
                    fields={graph.fields}
                    {focus}
                    depth={1}
                    bind:edgeStyle
                    controls
                    legend
                    label={`${name} and its neighbours`}
                    onselect={(id) => id && id !== focus && onselect?.(id)}
                />
            </div>
        </div>
    {:else}
        <ErDiagram
            nodes={graph.nodes}
            edges={graph.edges}
            fields={graph.fields}
            bind:density
            bind:edgeStyle
            controls
            legend
            label="Tables, views and what they reach"
            onselect={(id) => id && onselect?.(id)}
        />
    {/if}
</div>
