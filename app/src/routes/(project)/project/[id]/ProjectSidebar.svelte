<script lang="ts">
    import { page } from '$app/state';
    import { Eyebrow } from '$lib/components';
    import { ftrPctLabel } from '$lib/ftr.js';
    import MetricSparkline from '$lib/components/MetricSparkline.svelte';
    import ProjectGlyph from '../../../(observatory)/projects/ProjectGlyph.svelte';
    import type { ProjectIcon } from '../../../(observatory)/projects/buckets.js';
    import {
        SECTIONS,
        isSectionActive,
        ftrBand,
        ftrDelta,
        ftrSparkTone,
        APP_VERSION,
        type FtrBand,
    } from './project-sidebar-view.js';

    interface Props {
        projectId: string;
        name: string;
        client?: string | null;
        icon: ProjectIcon;
        ftr14d: number | null; // 0..1, or null when there's no FTR data
        ftr14dPrev?: number | null;
        ftrTrend?: number[];
        sessions7d?: number | null;
    }
    let {
        projectId,
        name,
        client = null,
        icon,
        ftr14d,
        ftr14dPrev = null,
        ftrTrend = [],
        sessions7d = null,
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
    data-component="project-sidebar"
    class="w-[180px] shrink-0 border-r border-paper-edge bg-paper flex flex-col py-3"
>
    <!-- Identity: project icon + name + a band-coloured status dot -->
    <div data-component="project-identity" class="px-4 pb-4 pt-2 flex flex-col gap-1.5">
        <div class="flex items-center gap-2">
            <ProjectGlyph {icon} />
            <Eyebrow>Project</Eyebrow>
            <span
                data-project-status
                data-band={band}
                class="ml-auto w-2 h-2 rounded-full {DOT[band]}"
                title="FTR band"
            ></span>
        </div>
        <span data-component="sidebar-project-name" class="text-sm font-semibold leading-tight">{name}</span>
        {#if client}
            <span
                class="self-start mono text-xs text-ink-mute bg-paper-soft border border-paper-edge rounded px-1.5 py-0.5"
                >{client}</span
            >
        {/if}
    </div>

    <div class="px-4 pb-2"><Eyebrow>This project</Eyebrow></div>

    <nav class="flex flex-col" aria-label="Project sections">
        {#each SECTIONS as section (section.id)}
            {@const active = isSectionActive(page.url.pathname, projectId, section.id)}
            <a
                href="/project/{projectId}/{section.id}"
                class="proj-nav-item flex items-center gap-2.5 px-4 py-2 no-underline text-inherit text-sm transition-colors duration-fast"
                class:active
            >
                <span class="kanji w-4 text-center" aria-hidden="true">{section.kanji}</span>
                <span>{section.label}</span>
            </a>
        {/each}
    </nav>

    <!-- Health readout: FTR·14d headline + delta + spark, then Sessions·7d -->
    <div data-component="project-health" class="mt-auto px-4 pt-3 flex flex-col gap-2 border-t border-paper-edge">
        <div class="flex items-baseline justify-between">
            <Eyebrow>FTR · 14d</Eyebrow>
            <span data-ftr-delta class="mono text-xs text-ink-mute">{delta}</span>
        </div>
        <div class="flex items-end justify-between gap-2">
            <span
                data-ftr-value
                class="display text-2xl font-light leading-none tabular-nums {band === 'warning' ? 'text-warning' : 'text-ink'}"
                >{ftrPctLabel(ftr14d)}</span
            >
            {#if ftrTrend.length}
                <MetricSparkline series={ftrTrend} tone={sparkTone} />
            {/if}
        </div>
        <div class="flex items-center justify-between text-xs">
            <span class="text-ink-soft">Sessions · 7d</span>
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
