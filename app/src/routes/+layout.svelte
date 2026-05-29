<script lang="ts">
    import "uno.css";
    import "../app.css";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { hasTauri } from "$lib/bootstrap.js";
    import { appState } from "$lib/appstate.svelte.js";

    let { children } = $props();

    // Expose appState on window for E2E test helpers (dev builds only).
    // Tests call `window.__sensei_state__.appState.config = {...}` to inject
    // daemon config without a full page reload.
    if (import.meta.env.DEV && typeof window !== "undefined") {
        (window as { __sensei_state__?: unknown }).__sensei_state__ = { appState };
    }

    // Wire up the Tauri native menu → SvelteKit navigation bridge. The
    // Rust side emits `open-logs` (one specific shortcut) and
    // `dev-navigate` (any view-menu item) events; we just translate them
    // into goto() so the routing guard in hooks.reroute applies the same
    // way as in-app navigation.
    onMount(() => {
        if (!hasTauri()) return;
        const unlistens: Array<() => void> = [];
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        import("@tauri-apps/api/event").then(({ listen }) => {
            listen<void>("open-logs", () => {
                goto("/logs");
            }).then((fn) => unlistens.push(fn));
            listen<string>("dev-navigate", (e) => {
                goto(e.payload, { replaceState: true });
            }).then((fn) => unlistens.push(fn));
        });
        return () => {
            for (const fn of unlistens) fn();
        };
    });
</script>

{@render children()}
