<script lang="ts">
    import "uno.css";
    import "../app.css";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { hasTauri } from "$lib/bootstrap.js";

    let { children } = $props();

    //   function applyColorScheme() {
    //     const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    //     document.body.dataset.mode = dark ? 'dark' : 'light';
    //   }

    //   onMount(() => {
    //     // Port resolution now happens in +layout.ts load() — runs before any page loader.

    //     applyColorScheme();
    //     const mq = window.matchMedia('(prefers-color-scheme: dark)');
    //     mq.addEventListener('change', applyColorScheme);

    //     const unlistens: Array<() => void> = [];
    //     if (hasTauri()) {
    //       import('@tauri-apps/api/event').then(({ listen }) => {
    //         listen<void>('open-logs', () => {
    //           goto('/logs');
    //         }).then(fn => unlistens.push(fn));

    //         // View-menu navigation — just navigates. The routing guard still
    //         // applies; if health is not ready, reroute will intercept and send
    //         // the user to /health (HealthState owns the cache, not this listener).
    //         listen<string>('dev-navigate', (e) => {
    //           goto(e.payload, { replaceState: true });
    //         }).then(fn => unlistens.push(fn));
    //       });
    //     }

    //     return () => {
    //       mq.removeEventListener('change', applyColorScheme);
    //       unlistens.forEach(fn => fn());
    //     };
    //   });
    //
</script>

{@render children()}
