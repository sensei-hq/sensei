<script lang="ts">
    import { page } from '$app/state';
    import { DIAGRAM_VIEWS, viewOf } from './diagrams-nav.js';

    let { children } = $props();

    const active = $derived(viewOf(page.url.pathname));
    const projectId = $derived(page.params.id);
</script>

<nav class="flex items-center gap-2 px-4 pt-4" aria-label="Diagram views">
    {#each DIAGRAM_VIEWS as view (view.id)}
        <a
            href={`/project/${projectId}/diagrams/${view.id}`}
            class="px-3 py-1 text-sm rounded border border-paper-edge no-underline text-ink"
            class:bg-primary={active === view.id}
            class:text-on-primary={active === view.id}
            aria-current={active === view.id ? 'page' : undefined}
            data-testid={`diagrams-tab-${view.id}`}
        >
            {view.label}
        </a>
    {/each}
</nav>

{@render children()}
