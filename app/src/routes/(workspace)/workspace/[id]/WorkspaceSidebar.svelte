<script lang="ts">
    import Kanji from '$lib/components/Kanji.svelte';
    import { page } from '$app/state';
    import { Eyebrow } from '$lib/components';
    import { ftrReadLabel } from '$lib/ftr.js';
    import MetricSparkline from '$lib/components/MetricSparkline.svelte';
    import WorkspaceGlyph from '../../../(observatory)/workspaces/WorkspaceGlyph.svelte';
    import type { WorkspaceIcon } from '../../../(observatory)/workspaces/buckets.js';
    import {
        SECTIONS,
        isSectionActive,
        ftrBand,
        ftrDelta,
        ftrSparkTone,
        APP_VERSION,
        type FtrBand,
    } from './workspace-sidebar-view.js';
    import { workspaceVisibilityLabel } from './overview/overview-view.svelte.js';

    interface Props {
        workspaceId: string;
        name: string;
        client?: string | null;
        icon: WorkspaceIcon;
        ftr14d: number | null; // 0..1, or null when there's no FTR data
        ftr14dPrev?: number | null;
        ftrTrend?: number[];
        sessions7d?: number | null;
        /** Why the FTR could not be read, or null — then the value says "unavailable" (#288). */
        ftrError?: string | null;
    }
    let {
        workspaceId,
        name,
        client = null,
        icon,
        ftr14d,
        ftr14dPrev = null,
        ftrTrend = [],
        sessions7d = null,
        ftrError = null,
    }: Props = $props();

    // FTR leads the Health readout below (a spark + delta), matching the mock; the
    // identity slot carries a small band-coloured status dot.
    const band = $derived(ftrBand(ftr14d));
    const delta = $derived(ftrDelta(ftr14d, ftr14dPrev));
    const sparkTone = $derived(ftrSparkTone(ftr14d, ftr14dPrev));
    // Static class maps so UnoCSS extracts the tokens (never a dynamic `bg-{band}`).
    const DOT: Record<FtrBand, string> = {
        success: 'bg-success',
        accent: 'bg-accent',
        warning: 'bg-warning',
        'ink-faint': 'bg-ink-faint',
    };
</script>

<aside
    data-component="workspace-sidebar"
    class="w-[180px] shrink-0 border-r border-paper-edge bg-paper flex flex-col py-3"
>
    <!-- Identity card: glyph · status dot · name, then the visibility chip.
         Mockups 02-ev.png / 02-rev3.png frame this as a bordered card with the
         dot beside the name, and carry no "Workspace" eyebrow — "THIS WORKSPACE"
         below already says it. -->
    <div class="px-3 pb-3 pt-1">
        <div
            data-component="workspace-identity"
            class="rounded-lg border border-paper-edge bg-paper-soft px-3 py-2.5 flex flex-col gap-2"
        >
            <div class="flex items-center gap-2 min-w-0">
                <WorkspaceGlyph {icon} />
                <span
                    data-workspace-status
                    data-band={band}
                    class="w-2 h-2 shrink-0 rounded-full {DOT[band]}"
                    title="FTR band"
                ></span>
                <span
                    data-component="sidebar-workspace-name"
                    class="text-sm font-semibold leading-tight truncate">{name}</span>
            </div>
            <!-- Always present: an internal workspace used to show no chip at all,
                 which read as missing data rather than "no client". -->
            <span
                data-component="sidebar-workspace-visibility"
                class="self-start mono text-xs uppercase text-ink-soft bg-paper border border-paper-edge rounded px-1.5 py-0.5"
                >{workspaceVisibilityLabel(client)}</span
            >
        </div>
    </div>

    <div class="px-4 pb-2"><Eyebrow>This workspace</Eyebrow></div>

    <nav class="flex flex-col" aria-label="Workspace sections">
        {#each SECTIONS as section (section.id)}
            {@const active = isSectionActive(page.url.pathname, workspaceId, section.id)}
            <a
                href="/workspace/{workspaceId}/{section.id}"
                class="proj-nav-item flex items-center gap-2.5 px-4 py-2 no-underline text-inherit text-sm transition-colors duration-fast"
                class:active
            >
                <Kanji char={section.kanji} size="inherit" tone="inherit" class="w-4 text-center" />
                <span>{section.label}</span>
            </a>
        {/each}
    </nav>

    <!-- Health readout: FTR·14d headline + delta + spark, then Sessions·7d -->
    <div data-component="workspace-health" class="mt-auto px-4 pt-3 flex flex-col gap-2 border-t border-paper-edge">
        <div class="flex items-baseline justify-between">
            <Eyebrow>FTR · 14d</Eyebrow>
            <span data-ftr-delta class="mono text-xs text-ink-soft">{delta}</span>
        </div>
        <div class="flex items-end justify-between gap-2">
            <span
                data-ftr-value
                class="display text-2xl font-light leading-none tabular-nums {band === 'warning' ? 'text-warning' : 'text-ink'}"
                title={ftrError}
                >{ftrReadLabel({ data: { ftr14d }, error: ftrError })}</span
            >
            {#if ftrTrend.length}
                <MetricSparkline series={ftrTrend} tone={sparkTone} />
            {/if}
        </div>
        <div class="flex items-center justify-between text-xs">
            <span class="text-ink-mute">Sessions · 7d</span>
            <span class="mono text-ink">{sessions7d == null ? '—' : sessions7d}</span>
        </div>
    </div>

    <!-- Build version — so it's obvious which app version is in front of you. -->
    <div
        data-component="sidebar-version"
        class="px-4 pt-2 flex items-center justify-between text-xs text-ink-faint"
    >
        <span>Version</span>
        <span class="mono">v{APP_VERSION}</span>
    </div>
</aside>

<style>
    .proj-nav-item:hover {
        background: var(--paper-soft);
    }
    .proj-nav-item.active {
        background: var(--accent-soft);
        color: var(--accent);
    }
</style>
