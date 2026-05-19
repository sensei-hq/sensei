<script lang="ts">
    import { onMount } from "svelte";
    let { children } = $props();

    // Boot-timing — remove with the rest of the marks once boot loader ships.
    (window as any).__senseiMark?.("health-layout-script");
    onMount(() => {
        (window as any).__senseiMark?.("health-layout-mount");
    });
</script>

<div class="w-full h-screen flex flex-col bg-surface-z0 overflow-hidden">
    <div class="drag-region h-8 shrink-0"></div>
    <main class="flex-1 min-h-0 flex flex-col">
        {@render children()}
    </main>
</div>
