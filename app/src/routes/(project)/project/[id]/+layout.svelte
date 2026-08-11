<script lang="ts">
    import { page } from '$app/state';
    import ProjectSidebar from './ProjectSidebar.svelte';
    import ProjectGlyph from '../../../(observatory)/projects/ProjectGlyph.svelte';
    import { projectIcon } from '../../../(observatory)/projects/buckets.js';
    import { apiBase } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';

    let { data, children } = $props();

    // The active section id from /project/{id}/{section} — exposed as a stable
    // data-section hook on <main> so e2e + the inspect harness can target any
    // section page by name without depending on per-page markup.
    const section = $derived(page.url.pathname.split('/')[3] ?? 'overview');

    // Resolve the icon the same way the project cards do: an image icon (e.g. a
    // repo logo .svg) renders as an <img>, a kanji icon as a glyph, else the 場
    // fallback — never the raw icon value (e.g. "rokkit.svg") shown as text.
    const icon = $derived(
        projectIcon(data.project ?? { id: data.projectId, icon: null }, apiBase(appState.port)),
    );
    // Keep an absent FTR as null (honest no-data → "—" in the sidebar), never 0.
    const ftr14d = $derived(data.ftrMetrics?.ftr14d ?? null);
</script>

<div data-component="project-shell" class="flex flex-col h-screen overflow-hidden bg-paper-soft text-ink">
    <!-- Titlebar / drag region. Left inset (pl-[80px]) clears the macOS overlay
         traffic lights that float top-left, so the project name is never hidden
         behind them. `data-tauri-drag-region` is Tauri's drag mechanism; it needs
         the `core:window:allow-start-dragging` capability (granted for project-*
         windows in capabilities/default.json) — without it startDragging is denied
         and the window won't move. Tauri's handler walks ancestors (closest) so
         clicking the glyph/name drags too. (No decorative accent stripe: it did
         not aid dragging and read as a stray highlighted top border.) -->
    <div data-component="project-titlebar" data-tauri-drag-region class="h-9 flex items-center gap-2 pl-[80px] pr-4 shrink-0">
        <ProjectGlyph {icon} />
        <span data-component="project-name" class="text-sm font-semibold">{data.project?.name ?? '…'}</span>
        <span class="text-xs text-ink-faint">· project window</span>
    </div>

    <div class="flex flex-1 overflow-hidden">
        <ProjectSidebar
            projectId={data.projectId}
            {icon}
            name={data.project?.name ?? '…'}
            client={data.project?.client ?? null}
            {ftr14d}
            sessions7d={data.ftrMetrics?.sessions7d ?? null}
        />

        <main data-component="project-main" data-section={section} class="flex-1 overflow-y-auto">
            {@render children()}
        </main>
    </div>
</div>
