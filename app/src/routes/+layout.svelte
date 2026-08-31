<script lang="ts">
    import "uno.css";
    import "../app.css";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { hasTauri } from "$lib/bootstrap.js";
    import { appState } from "$lib/appstate.svelte.js";
    import { healthState } from "$lib/health-state.svelte.js";
    import { daemonHealth } from "$lib/daemon-health.svelte.js";
    import { forgeAuth } from "$lib/forge-auth.svelte.js";
    import { personaList } from "$lib/personas.svelte.js";
    import { DaemonStatusBanner, SignInOverlay } from "$lib/components";

    let { children } = $props();

    // The overlay is opened by need, and closable. An expired credential stops
    // syncing but does not stop sensei, so blocking the app behind it would be
    // out of proportion.
    let dismissed = $state(false);
    // Asked for explicitly, from View → Identities…. Kept separate from the
    // automatic path so opening it by hand is not undone by `dismissed`, and so
    // it works when nothing is wrong — connecting a new identity is an
    // intentional act, not a repair.
    let requested = $state(false);
    // The automatic half opens only when a CONNECTED identity has broken. It
    // deliberately ignores never-connected personas (an install has several) and
    // renewal (which is silent), or the overlay would appear on every launch.
    let overlayOpen = $derived(requested || (!dismissed && personaList.needsAttention));

    function closeOverlay() {
        requested = false;
        dismissed = true;
    }

    async function signIn(p: Parameters<typeof personaList.signIn>[0]) {
        await personaList.signIn(p);
        // Re-read rather than assume: the standing changes only once the daemon
        // has observed the new credential, and reporting success from the click
        // is how a failed sign-in looks like a successful one.
        await personaList.load();
    }

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

        // Watch the forge credential and re-authorize before it expires. The
        // daemon cannot do this: renewal re-runs the authorize flow Supabase
        // performs (that is where the App's client secret lives, in one place),
        // and that needs a browser. For an already-authorized App it is a
        // redirect with no prompt — measured at ~6s, no clicks.
        //
        // Tauri-only. A browser dev session has no opener plugin, and polling
        // there would report a standing nobody can act on.
        if (!hasTauri()) return () => daemonHealth.stop();
        // Five minutes against an 8h token with a 1h renewal window: ~12 checks
        // inside the window, and the ONE automatic attempt per window is
        // enforced in ForgeAuth, not by the interval.
        forgeAuth.start();
        // The identity list backs the overlay. Loaded once here and re-read
        // after each attempt; the 5-minute forgeAuth poll is what notices a
        // credential dying while the app sits open.
        void personaList.load();
        const unlistens: Array<() => void> = [];
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        import("@tauri-apps/api/event").then(({ listen }) => {
            listen<void>("open-logs", () => {
                goto("/logs");
            }).then((fn) => unlistens.push(fn));
            listen<void>("open-identities", () => {
                // Re-read on open: the list is polled every five minutes, and a
                // stale standing is what makes a "ready" row fail on click.
                void personaList.load();
                requested = true;
            }).then((fn) => unlistens.push(fn));
            listen<string>("dev-navigate", (e) => {
                goto(e.payload, { replaceState: true });
            }).then((fn) => unlistens.push(fn));
        });
        return () => {
            for (const fn of unlistens) fn();
            daemonHealth.stop();
            forgeAuth.stop();
        };
    });
</script>

<DaemonStatusBanner mode={daemonHealth.dbMode} />
<SignInOverlay
    open={overlayOpen}
    personas={personaList.personas}
    error={personaList.error ?? forgeAuth.lastError}
    loaded={personaList.loaded}
    isBusy={(p) => personaList.isBusy(p)}
    actionLabel={(p) => personaList.actionLabel(p)}
    describe={(p) => personaList.describe(p, Math.floor(Date.now() / 1000))}
    onSignIn={signIn}
    onClose={closeOverlay}
/>
{@render children()}
