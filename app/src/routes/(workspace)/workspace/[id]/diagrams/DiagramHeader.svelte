<script lang="ts">
    // Every diagram's header (M17 mockup): its mark, name, level and what it
    // shows; four toggles, each opening one band — the info (use it to · how to
    // read it · what to do), the workspace's numbers (the same on every diagram;
    // Cycles opens the Cycles diagram), the controls the page passes — and the
    // side panel beside the diagram.
    import type { Snippet } from 'svelte';
    import { PageHeader } from '$lib/components';
    import type { Stat } from '@sensei/codemap';
    import type { DiagramView } from './diagrams-nav.js';
    import type { DiagramHeaderState } from './diagram-header.svelte.js';

    let {
        view,
        about,
        level = null,
        workspace,
        stats,
        budget = null,
        hrefFor,
        controls,
    }: {
        view: DiagramView;
        about: DiagramHeaderState;
        /** Where the diagram is, e.g. `L0 · modules`; none for a diagram without levels. */
        level?: string | null;
        workspace: { name: string; repos: number | null };
        /** The workspace's numbers; null while they are read. */
        stats: Stat[] | null;
        /** What the diagram draws of what there is. */
        budget?: string | null;
        /** The route of a diagram, for a stat one of them shows. */
        hrefFor: (lens: string) => string;
        controls?: Snippet;
    } = $props();

    const TONE = { ink: 'text-ink', danger: 'text-danger', warning: 'text-warning', success: 'text-success', faint: 'text-ink-faint' };
    const ABOUT = $derived([
        { label: 'Use it to', text: view.use, accent: false },
        { label: 'How to read it', text: view.read, accent: false },
        { label: 'What to do', text: view.todo, accent: true },
    ]);
    // The workspace is the one open, so it is not named here: its repos sit beside
    // its modules, as numbers like the rest.
    const cards = $derived(
        stats
            ? stats.flatMap((s) =>
                  s.label === 'Modules' && workspace.repos != null
                      ? [s, { label: 'Repos', value: workspace.repos.toLocaleString('en-US'), tone: 'ink' as const }]
                      : [s],
              )
            : [],
    );
    const toggles = $derived([
        { id: 'about', label: "What it's for", icon: 'i-solar-info-circle-bold-duotone', on: about.open, flip: () => about.toggle() },
        { id: 'stats', label: 'Workspace numbers', icon: 'i-solar-chart-2-bold-duotone', on: about.stats, flip: () => about.toggleStats() },
        ...(controls ? [{ id: 'controls', label: 'Controls', icon: 'i-solar-tuning-4-bold-duotone', on: about.controls, flip: () => about.toggleControls() }] : []),
        { id: 'side', label: 'Side panel', icon: 'i-solar-sidebar-minimalistic-bold-duotone', on: about.side, flip: () => about.toggleSide() },
    ]);
</script>

{#snippet title()}
    <span class="inline-flex items-baseline gap-3">
        {view.label}
        {#if level}<span class="font-mono text-sm font-normal text-ink-mute" data-testid="diagram-level">{level}</span>{/if}
    </span>
{/snippet}

<div class="flex flex-col divide-y divide-paper-edge" data-testid="diagram-header">
    <PageHeader kanji={view.kanji} icon={view.icon} {title} description={view.shows}>
        {#snippet right()}
            <div class="flex items-center gap-2">
                {#each toggles as t (t.id)}
                    <button
                        class="w-10 h-10 flex items-center justify-center rounded-lg border border-paper-edge cursor-pointer"
                        class:bg-paper-mute={t.on}
                        class:text-ink={t.on}
                        class:bg-transparent={!t.on}
                        class:text-ink-mute={!t.on}
                        aria-pressed={t.on}
                        aria-label={t.label}
                        title={t.label}
                        onclick={t.flip}
                        data-testid={`diagram-${t.id}-toggle`}><span class={`${t.icon} text-xl`} aria-hidden="true"></span></button>
                {/each}
            </div>
        {/snippet}
    </PageHeader>
    {#if about.open}
        <div class="grid gap-8 px-6 py-5 md:grid-cols-3" data-testid="diagram-about">
            {#each ABOUT as a (a.label)}
                <div class="flex flex-col gap-2">
                    <span class="text-xs uppercase tracking-wide" class:text-accent={a.accent} class:text-ink-mute={!a.accent}>{a.label}</span>
                    <p class="m-0 text-sm text-ink-soft">{a.text}</p>
                </div>
            {/each}
        </div>
    {/if}
    {#if about.stats}
        <div class="flex flex-col gap-3 px-6 py-5" data-testid="diagram-stats">
            <ul class="m-0 p-0 list-none flex flex-wrap gap-2">
                {#each cards as s (s.label)}
                    <li class="flex flex-col gap-0.5 min-w-24 px-3 py-2 rounded-lg border border-paper-edge bg-paper-soft" data-testid={`diagram-stat-${s.label}`}>
                        <span class={`font-mono text-sm ${TONE[s.tone]}`}>
                            {#if s.lens}<a class={`no-underline hover:underline ${TONE[s.tone]}`} href={hrefFor(s.lens)}>{s.value}</a>{:else}{s.value}{/if}
                        </span>
                        <span class="text-xs text-ink-mute">{s.label}</span>
                    </li>
                {/each}
            </ul>
            {#if !stats}<p class="m-0 text-sm text-ink-faint">Reading the workspace…</p>{/if}
            {#if budget}<p class="m-0 font-mono text-xs text-ink-mute" data-testid="diagram-budget">{budget}</p>{/if}
        </div>
    {/if}
    {#if controls && about.controls}
        <div class="px-6 py-4" data-testid="diagram-controls">{@render controls()}</div>
    {/if}
</div>
