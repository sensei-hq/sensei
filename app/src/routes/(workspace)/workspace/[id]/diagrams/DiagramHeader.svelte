<script lang="ts">
    // Every diagram's header (M17): its mark, name and question, and — behind
    // "What it's for" — what it shows, what to use it for and how to read it.
    import { PageHeader } from '$lib/components';
    import type { DiagramView } from './diagrams-nav.js';
    import type { DiagramHeaderState } from './diagram-header.svelte.js';

    let { view, about }: { view: DiagramView; about: DiagramHeaderState } = $props();
</script>

<div class="flex flex-col gap-2" data-testid="diagram-header">
    <PageHeader kanji={view.kanji} icon={view.icon} title={view.label} description={view.question}>
        {#snippet right()}
            <button
                class="px-2 py-1 text-xs rounded border border-paper-edge bg-transparent text-ink-mute cursor-pointer"
                aria-expanded={about.open}
                onclick={() => about.toggle()}
                data-testid="diagram-about-toggle">What it's for {about.open ? '▴' : '▾'}</button>
        {/snippet}
    </PageHeader>
    {#if about.open}
        <dl class="m-0 grid gap-x-4 gap-y-1 text-xs text-ink-soft px-6 pb-2" style="grid-template-columns: auto 1fr;" data-testid="diagram-about">
            <dt class="text-ink-mute">Shows</dt><dd class="m-0">{view.shows}</dd>
            <dt class="text-ink-mute">Use it to</dt><dd class="m-0">{view.use}</dd>
            <dt class="text-ink-mute">Read it</dt><dd class="m-0">{view.read}</dd>
        </dl>
    {/if}
</div>
