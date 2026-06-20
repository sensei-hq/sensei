<script lang="ts">
    import { onMount } from 'svelte';
    import { appState } from '$lib/appstate.svelte.js';
    import ObservatorySidebar from './ObservatorySidebar.svelte';

    let { children } = $props();

    onMount(async () => {
        await appState.load();
    });
</script>

<div class="w-full h-screen flex flex-col bg-paper-soft text-ink overflow-hidden">
    <div class="drag-region h-8 shrink-0"></div>

    <div class="flex-1 flex min-h-0">
        <ObservatorySidebar port={appState.port} />

        <main class="flex-1 overflow-auto">
            {@render children()}
        </main>
    </div>
</div>
