<script lang="ts">
    import { setContext, untrack, type Snippet } from 'svelte';
    import { PlotState } from '@rokkit/chart';

    // A themed, responsive host for @rokkit/chart's composable Plot.* geoms.
    // It owns the PlotState (scales + an optional fixed y-domain) and provides
    // it via the 'plot-state' context the geoms read, then renders those geoms
    // (passed as `children`) inside an SVG whose colours come from the app's
    // named tokens — currentColor for theming plus explicit `var(--token)`
    // strokes/fills on each geom, never raw hex. Width is scale-free via the
    // viewBox, so there is no ResizeObserver and the chart renders under SSR and
    // jsdom the same as in the app.
    type Margin = { top: number; right: number; bottom: number; left: number };
    type Row = Record<string, unknown>;

    let {
        rows,
        x,
        y,
        yDomain = undefined,
        width,
        height,
        margin,
        class: klass = '',
        preserveAspectRatio = 'xMidYMid meet',
        svgWidth = undefined,
        svgHeight = undefined,
        dataComponent = undefined,
        dataTone = undefined,
        style = undefined,
        children,
    }: {
        /** Scale-defining rows — one object per slot, `y` may be null (filtered). */
        rows: Row[];
        x: string;
        y: string;
        /** Fixed y-domain `[min, max]`; omit to auto-scale to the data (min..max). */
        yDomain?: [number, number];
        /** Logical viewBox dimensions (CSS scales the rendered size). */
        width: number;
        height: number;
        margin: Margin;
        class?: string;
        preserveAspectRatio?: string;
        /** Explicit px width/height attrs; omit the one CSS should drive. */
        svgWidth?: number;
        svgHeight?: number;
        dataComponent?: string;
        dataTone?: string;
        /** Inline style on the <svg> — used to pass chart CSS vars (e.g. --chart-highlight-color). */
        style?: string;
        children: Snippet;
    } = $props();

    function config() {
        return { data: rows, channels: { x, y }, width, height, margin, yDomain };
    }

    // Mirror @rokkit's own Plot.Root: build the state untracked, then keep it in
    // sync with an effect (which the constructor's initial values seed for SSR).
    const state = untrack(() => new PlotState(config()));
    setContext('plot-state', state);

    $effect(() => {
        state.update(config());
    });
</script>

<svg
    data-component={dataComponent}
    data-tone={dataTone}
    width={svgWidth}
    height={svgHeight}
    viewBox="0 0 {width} {height}"
    {preserveAspectRatio}
    class={klass}
    {style}
    aria-hidden="true"
>
    <g transform="translate({margin.left}, {margin.top})">
        {@render children()}
    </g>
</svg>
