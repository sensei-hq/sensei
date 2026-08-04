<script lang="ts">
    import ProjectSidebar from './ProjectSidebar.svelte';
    import ProjectGlyph from '../../../(observatory)/projects/ProjectGlyph.svelte';
    import { projectIcon } from '../../../(observatory)/projects/buckets.js';
    import { apiBase } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';

    let { data, children } = $props();

    // Resolve the icon the same way the project cards do: an image icon (e.g. a
    // repo logo .svg) renders as an <img>, a kanji icon as a glyph, else the 場
    // fallback — never the raw icon value (e.g. "rokkit.svg") shown as text.
    const icon = $derived(
        projectIcon(data.project ?? { id: data.projectId, icon: null }, apiBase(appState.port)),
    );
    const ftr14d = $derived(data.ftrMetrics?.ftr14d ?? 0);
</script>

<div data-component="project-shell" class="flex flex-col h-screen overflow-hidden bg-paper-soft text-ink">
    <!-- Primary accent stripe (decorative 2px accent bar; also draggable so the
         very top edge moves the window). -->
    <div data-tauri-drag-region class="h-0.5 bg-accent shrink-0"></div>

    <!-- Titlebar / drag region. Left inset (pl-[80px]) clears the macOS overlay
         traffic lights that float top-left, so the project name is never hidden
         behind them. `data-tauri-drag-region` is Tauri's own drag mechanism —
         `-webkit-app-region: drag` (the .drag-region class) is Electron's and
         WKWebView applies it unreliably to secondary windows like this one; the
         attribute works for every window, and Tauri's handler walks ancestors
         (closest) so clicking the glyph/name drags too. -->
    <div data-component="project-titlebar" data-tauri-drag-region class="drag-region h-9 flex items-center gap-2 pl-[80px] pr-4 shrink-0">
        <ProjectGlyph {icon} />
        <span data-component="project-name" class="text-sm font-semibold">{data.project?.name ?? '…'}</span>
        <span class="text-xs text-ink-faint">· project window</span>
    </div>

    <div class="flex flex-1 overflow-hidden">
        <ProjectSidebar projectId={data.projectId} {ftr14d} />

        <main class="flex-1 overflow-y-auto">
            {@render children()}
        </main>
    </div>
</div>
