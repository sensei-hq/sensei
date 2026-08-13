<script lang="ts">
    import { page } from '$app/state';
    import { Eyebrow } from '$lib/components';
    import ProjectGlyph from '../../../(observatory)/projects/ProjectGlyph.svelte';
    import type { ProjectIcon } from '../../../(observatory)/projects/buckets.js';
    import { SECTIONS, isSectionActive, healthRows, APP_VERSION } from './project-sidebar-view.js';

    interface Props {
        projectId: string;
        name: string;
        client?: string | null;
        icon: ProjectIcon;
        ftr14d: number | null; // 0..1, or null when there's no FTR data
        sessions7d?: number | null;
    }
    let { projectId, name, client = null, icon, ftr14d, sessions7d = null }: Props = $props();

    // FTR moves out of the identity slot and into the Health readout below —
    // the sidebar now leads with the project, matching the mock.
    const health = $derived(healthRows(ftr14d, sessions7d));
</script>

<aside
    data-component="project-sidebar"
    class="w-[180px] shrink-0 border-r border-paper-edge bg-paper flex flex-col py-3"
>
    <!-- Identity: project icon + name (was a bare FTR number) -->
    <div data-component="project-identity" class="px-4 pb-4 pt-2 flex flex-col gap-1.5">
        <div class="flex items-center gap-2">
            <ProjectGlyph {icon} />
            <Eyebrow>Project</Eyebrow>
        </div>
        <span data-component="sidebar-project-name" class="text-sm font-semibold leading-tight">{name}</span>
        {#if client}
            <span class="mono text-xs text-ink-mute">{client}</span>
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

    <!-- Health readout: FTR (demoted here) + recent sessions -->
    <div data-component="project-health" class="mt-auto px-4 pt-3 flex flex-col gap-1.5 border-t border-paper-edge">
        <Eyebrow>Health</Eyebrow>
        {#each health as row (row.label)}
            <div class="flex items-center justify-between text-xs">
                <span class="text-ink-soft">{row.label}</span>
                <span class="mono text-ink">{row.value}</span>
            </div>
        {/each}
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
