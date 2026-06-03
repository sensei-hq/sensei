<script lang="ts">
    import { onMount } from 'svelte';
    import { appState } from '$lib/appstate.svelte.js';
    import { senseiApi } from '$lib/api.js';
    import ObservatorySidebar from './ObservatorySidebar.svelte';

    type SidebarProject = { id: string; name: string; kanji: string };

    let { children } = $props();

    let projects = $state<SidebarProject[]>([]);

    onMount(async () => {
        await appState.load();
        const api = senseiApi(appState.port);
        const raw = await api.listProjects();
        projects = raw.map((p: any) => ({
            id: p.id,
            name: p.name,
            kanji: p.icon?.value ?? '場',
        }));
    });
</script>

<div class="w-full h-screen flex flex-col bg-paper-soft text-ink overflow-hidden">
    <div class="drag-region h-8 shrink-0"></div>

    <div class="flex-1 flex min-h-0">
        <ObservatorySidebar {projects} port={appState.port} />

        <main class="flex-1 overflow-auto">
            {@render children()}
        </main>
    </div>
</div>
