<script lang="ts">
    // Every diagram's header (M17): its mark, name and question; an info toggle
    // opening what it shows, what to use it for and how to read it (three columns,
    // as the mockup has them); and a stats toggle opening the workspace's numbers,
    // the same on every diagram — the Cycles figure opens the Cycles diagram.
    import { PageHeader } from '$lib/components';
    import type { Stat } from '@sensei/codemap';
    import type { DiagramView } from './diagrams-nav.js';
    import type { DiagramHeaderState } from './diagram-header.svelte.js';

    let {
        view,
        about,
        stats,
        hrefFor,
    }: {
        view: DiagramView;
        about: DiagramHeaderState;
        /** The workspace's numbers; null while they are read. */
        stats: Stat[] | null;
        /** The route of a diagram, for a stat that one of them shows. */
        hrefFor: (lens: string) => string;
    } = $props();

    const TONE = { ink: 'text-ink', danger: 'text-danger', warning: 'text-warning', success: 'text-success', faint: 'text-ink-faint' };
    const ABOUT = $derived([
        { label: 'Shows', text: view.shows },
        { label: 'Use it to', text: view.use },
        { label: 'Read it', text: view.read },
    ]);
</script>

<div class="flex flex-col" data-testid="diagram-header">
    <PageHeader kanji={view.kanji} icon={view.icon} title={view.label} description={view.question}>
        {#snippet right()}
            <div class="flex items-center gap-1">
                <button
                    class="w-8 h-8 flex items-center justify-center rounded border border-paper-edge cursor-pointer text-ink-mute"
                    class:bg-paper-mute={about.open}
                    class:bg-transparent={!about.open}
                    aria-pressed={about.open}
                    aria-label="What it's for"
                    title="What it's for"
                    onclick={() => about.toggle()}
                    data-testid="diagram-about-toggle"><span class="i-solar-info-circle-bold-duotone text-lg" aria-hidden="true"></span></button>
                <button
                    class="w-8 h-8 flex items-center justify-center rounded border border-paper-edge cursor-pointer text-ink-mute"
                    class:bg-paper-mute={about.stats}
                    class:bg-transparent={!about.stats}
                    aria-pressed={about.stats}
                    aria-label="Workspace numbers"
                    title="Workspace numbers"
                    onclick={() => about.toggleStats()}
                    data-testid="diagram-stats-toggle"><span class="i-solar-chart-2-bold-duotone text-lg" aria-hidden="true"></span></button>
            </div>
        {/snippet}
    </PageHeader>
    {#if about.open}
        <div class="grid gap-6 px-6 pb-4 md:grid-cols-3" data-testid="diagram-about">
            {#each ABOUT as a (a.label)}
                <div class="flex flex-col gap-1">
                    <span class="text-xs uppercase tracking-wide text-ink-mute">{a.label}</span>
                    <p class="m-0 text-sm text-ink-soft">{a.text}</p>
                </div>
            {/each}
        </div>
    {/if}
    {#if about.stats}
        <dl class="m-0 flex flex-wrap gap-x-8 gap-y-3 px-6 pb-4" data-testid="diagram-stats">
            {#each stats ?? [] as s (s.label)}
                <div class="flex flex-col" data-testid={`diagram-stat-${s.label}`}>
                    <dt class="text-xs uppercase tracking-wide text-ink-mute">{s.label}</dt>
                    <dd class={`m-0 font-mono text-lg ${TONE[s.tone]}`}>
                        {#if s.lens}<a class={`no-underline hover:underline ${TONE[s.tone]}`} href={hrefFor(s.lens)}>{s.value}</a>{:else}{s.value}{/if}
                    </dd>
                </div>
            {:else}
                <p class="m-0 text-sm text-ink-faint">Reading the workspace…</p>
            {/each}
        </dl>
    {/if}
</div>
