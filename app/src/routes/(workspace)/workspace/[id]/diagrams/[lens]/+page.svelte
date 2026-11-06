<script lang="ts">
    // A diagram on the ScreenShell (D87, contract §4/§7/§8/§9): the shell lays out
    // the view's title, its info, the workspace's numbers, the controls and the
    // inspector, and remembers which bands are open under the view's id. The map
    // in the body is unchanged — one canvas, the route's lens on it.
    import { untrack } from 'svelte';
    import { ShellScreen } from '@sensei/ui';
    import { CodeMap, CodeMapControls, CodeMapSide, type Stat } from '@sensei/codemap';
    import type { InfoSlot, ShellStat } from '@rokkit/app';
    import { codeMapHost } from '../codemap-host.svelte.js';
    import { viewIdOf } from '../diagrams-nav.js';

    let { data } = $props();

    const host = codeMapHost();
    // The route names the lens; the map follows it. Untracked, so drilling into a
    // module does not re-run the switch and climb back out.
    $effect(() => {
        const [map, lens] = [host.state, data.lens];
        untrack(() => void map.showLens(host.port, lens));
    });

    const hrefFor = (lens: string) => `/workspace/${data.workspaceId}/diagrams/${lens}`;

    // The info band is the view's three columns (M17): what to use it for, how to
    // read it, what to do with it.
    const info = $derived<InfoSlot[]>([
        { label: 'Use it to', text: data.view.use },
        { label: 'How to read it', text: data.view.read },
        { label: 'What to do', text: data.view.todo },
    ]);

    // The old header's own tone names, as the shell's (#133).
    const TONE: Record<Stat['tone'], ShellStat['tone']> = {
        ink: 'ink', danger: 'danger', warning: 'warning', success: 'success', faint: 'muted',
    };
    const cardOf = (s: Stat): ShellStat => ({
        id: s.label.toLowerCase().replace(/\s+/g, '-'),
        title: s.label,
        value: s.value,
        tone: TONE[s.tone],
        icon: s.icon,
        href: s.lens ? hrefFor(s.lens) : undefined,
        attrs: { 'data-testid': `diagram-stat-${s.label}` },
    });
    /** The workspace's numbers, its repos beside its modules; null while reading. */
    const cards = $derived.by((): ShellStat[] | null => {
        const stats = host.stats;
        if (!stats) return null;
        const repos: Stat[] = data.workspace.repos_count == null ? [] : [{
            label: 'Repos',
            value: data.workspace.repos_count.toLocaleString('en-US'),
            tone: 'ink',
            icon: 'i-solar-folder-with-files-bold-duotone',
        }];
        return stats.flatMap((s) => (s.label === 'Modules' ? [s, ...repos] : [s])).map(cardOf);
    });
</script>

{#snippet controlsBand()}
    <div class="w-full" data-testid="diagram-controls">
        <CodeMapControls state={host.state} port={host.port} />
    </div>
{/snippet}

{#snippet sidePanel()}
    <div class="flex flex-col gap-6 p-4" data-testid="codemap-side">
        <CodeMapSide state={host.state} port={host.port} label={data.view.label} />
    </div>
{/snippet}

{#snippet budgetBand()}
    <p class="m-0 px-4 py-2 font-mono text-xs text-ink-mute" data-testid="diagram-budget">{host.budget}</p>
{/snippet}

<!-- Each diagram is its own screen; the map they share lives in the layout. The
     four toggles keep the header's old names and their `diagram-*-toggle` hooks. -->
{#key data.lens}
<ShellScreen
    viewId={viewIdOf(data.view)}
    mark={{ kanji: data.view.kanji, icon: data.view.icon }}
    title={data.view.label}
    description={data.view.shows}
    context={host.level}
    {info}
    stats={cards}
    search={false}
    labels={{ info: "What it's for", stats: 'Workspace numbers', filters: 'Controls', side: 'Side panel' }}
    toggleAttrs={{
        info: { 'data-testid': 'diagram-about-toggle' },
        stats: { 'data-testid': 'diagram-stats-toggle' },
        filters: { 'data-testid': 'diagram-controls-toggle' },
        side: { 'data-testid': 'diagram-side-toggle' },
    }}
    bandAttrs={{ stats: { 'data-testid': 'diagram-stats' } }}
    filters={controlsBand}
    footer={budgetBand}
    sidebar={sidePanel}>
    <div class="h-full min-h-0 flex flex-col gap-3 p-4" data-testid="codemap-screen" data-lens={data.lens}>
        {#if host.focus}
            <span class="self-start inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full border border-accent text-sm text-ink"
                data-testid="diagram-focus">
                <span class="i-solar-target-bold-duotone text-accent" aria-hidden="true"></span>
                <span class="text-ink-mute">Focus</span>
                <span class="font-mono max-w-80 truncate" title={host.focus}>{host.focus}</span>
                <button type="button" class="p-0 bg-transparent border-none cursor-pointer text-ink-mute hover:text-ink leading-none"
                    aria-label="Clear focus" title="Clear focus" onclick={() => host.clearFocus()}>
                    <span class="i-solar-close-circle-bold-duotone" aria-hidden="true"></span>
                </button>
            </span>
        {/if}
        <div class="flex-1 min-h-0">
            {#key data.workspaceId}
                <CodeMap state={host.state} port={host.port} controls={false} side={false} label={data.view.label} />
            {/key}
        </div>
    </div>
</ShellScreen>
{/key}
