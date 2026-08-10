<script lang="ts">
    import "uno.css";
    import "../app.css";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { hasTauri } from "$lib/bootstrap.js";
    import { appState } from "$lib/appstate.svelte.js";
    import { healthState } from "$lib/health-state.svelte.js";
    import { daemonHealth } from "$lib/daemon-health.svelte.js";
    import { DaemonStatusBanner } from "$lib/components";

    let { children } = $props();

    // Expose appState + healthState on window for E2E test helpers (dev builds
    // only). Tests inject daemon config via `__sensei_state__.appState.config`
    // and can drive the boot gate via `__sensei_state__.healthState.status` to
    // exercise reroute (e.g. the fresh-window project gate).
    if (import.meta.env.DEV && typeof window !== "undefined") {
        (window as { __sensei_state__?: unknown }).__sensei_state__ = { appState, healthState };
    }

    // One-shot migration: sweep the legacy `sensei:port` key. It used to
    // hold a runtime port override but is no longer used — the port is a
    // build-time constant. A stale value from a previous session would
    // otherwise be ignored harmlessly, but better to clear it than to
    // leave dead state lying around.
    if (typeof localStorage !== 'undefined') {
        try { localStorage.removeItem('sensei:port'); } catch { /* shim */ }
    }

    // Wire up the Tauri native menu → SvelteKit navigation bridge. The
    // Rust side emits `open-logs` (one specific shortcut) and
    // `dev-navigate` (any view-menu item) events; we just translate them
    // into goto() so the routing guard in hooks.reroute applies the same
    // way as in-app navigation.
    onMount(() => {
        // Watch the daemon's own DB-connection mode so a degraded → recovering
        // window (cold-boot race, self-healing) shows a banner. Cheap when the
        // daemon is healthy: it stops after the first `full` reading.
        daemonHealth.start();

        if (!hasTauri()) return () => daemonHealth.stop();
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
            daemonHealth.stop();
        };
    });
</script>

<DaemonStatusBanner mode={daemonHealth.dbMode} />
{@render children()}
