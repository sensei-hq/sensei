<script lang="ts">
    import ProjectSidebar from './ProjectSidebar.svelte';

    let { data, children } = $props();

    const kanji = $derived(data.project?.icon?.value ?? '場');
    const ftr14d = $derived(data.ftrMetrics?.ftr14d ?? 0);
</script>

<div class="flex flex-col h-screen overflow-hidden bg-paper-soft text-ink">
    <!-- Primary accent stripe -->
    <div class="h-0.5 bg-accent shrink-0"></div>

    <!-- Titlebar / drag region -->
    <div class="drag-region h-9 flex items-center gap-2 px-4 shrink-0">
        <span class="kanji text-lg text-accent">{kanji}</span>
        <span class="text-sm font-semibold">{data.project?.name ?? '…'}</span>
        <span class="text-xs text-ink-faint">· project window</span>
    </div>

    <div class="flex flex-1 overflow-hidden">
        <ProjectSidebar projectId={data.projectId} {ftr14d} />

        <main class="flex-1 overflow-y-auto">
            {@render children()}
        </main>
    </div>
</div>
