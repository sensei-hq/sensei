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
    import { glyphMode, setGlyphMode } from "$lib/glyphs";
    import { personaList } from "$lib/personas.svelte.js";
    import { DaemonStatusBanner, SignInOverlay } from "$lib/components";
    import { windowDuties } from "$lib/window-duties.js";

    let { children } = $props();

    // The overlay is opened by need, and closable. An expired credential stops
    // syncing but does not stop sensei, so blocking the app behind it would be
    // out of proportion.
    // The glyph preference, for every mark below (#133).
    setGlyphMode(() => glyphMode(appState.config));

    let dismissed = $state(false);
    // Asked for explicitly, from View → Identities…. Kept separate from the
    // automatic path so opening it by hand is not undone by `dismissed`, and so
    // it works when nothing is wrong — connecting a new identity is an
    // intentional act, not a repair.
    let requested = $state(false);
    // The automatic half opens only when a CONNECTED identity has broken. It
    // deliberately ignores never-connected personas (an install has several) and
    // renewal (which is silent), or the overlay would appear on every launch.
    // Which window this is decides what it does beyond its own pages: the
    // global work is the main window's alone (a second copy in Settings opened
    // a second browser tab for one renewal). No Tauri (a browser, a test): main.
    const label = (() => {
        if (!hasTauri()) return "main";
        try {
            return (window as { __TAURI_INTERNALS__?: { metadata?: { currentWebview?: { label?: string } } } })
                .__TAURI_INTERNALS__?.metadata?.currentWebview?.label ?? "main";
        } catch {
            return "main";
        }
    })();
    const duties = windowDuties(label);
    let overlayOpen = $derived(
        duties.showsIdentityOverlay && (requested || (!dismissed && personaList.needsAttention)),
    );

    function closeOverlay() {
        requested = false;
        dismissed = true;
    }

    // Expose appState + healthState on window for E2E test helpers (dev builds
    // only). Tests inject daemon config via `__sensei_state__.appState.config`
    // and can drive the boot gate via `__sensei_state__.healthState.status` to
    // exercise the boot gate (e.g. the fresh-window workspace gate).
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
    // into goto() so the routing guard in the boot gate (routes/+layout.ts) applies the same
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
        if (duties.renewsCredentials) {
            forgeAuth.start();
            // The identity state backs the overlay. It re-reads every minute and
            // moves its clock, so a credential that dies while the app sits open
            // says so without a click.
            void personaList.load();
            personaList.start();
        }
        const unlistens: Array<() => void> = [];
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        import("@tauri-apps/api/event").then(({ listen, emit }) => {
            if (duties.handlesMenu) {
                listen<void>("open-logs", () => {
                    goto("/logs");
                }).then((fn) => unlistens.push(fn));
                listen<void>("open-identities", () => {
                    // Re-read on open: the list is polled every five minutes, and a
                    // stale standing is what makes a "ready" row fail on click.
                    void personaList.load();
                    requested = true;
                }).then((fn) => unlistens.push(fn));
            }
            // A navigation for another window is that window's to take.
            listen<string>("dev-navigate", (e) => {
                if (duties.goesTo(e.payload.split("?")[0])) goto(e.payload, { replaceState: true });
            }).then((fn) => unlistens.push(fn));
            // Each window holds its own copy of the config: a change saved in
            // one (Settings, say) is re-read by the others.
            appState.onConfigSaved = () => void emit("config-saved", { from: label });
            listen<{ from: string }>("config-saved", (e) => {
                if (e.payload.from !== label) void appState.load();
            }).then((fn) => unlistens.push(fn));
            // The same for identities: the overlay (main) and Settings · Dōjō
            // (settings window) each hold the identity state; a sign-in, an
            // approval or a new default in one is re-read by the other.
            personaList.onChanged = () => void emit("identities-changed", { from: label });
            listen<{ from: string }>("identities-changed", (e) => {
                if (e.payload.from !== label) void personaList.load();
            }).then((fn) => unlistens.push(fn));
        });
        return () => {
            for (const fn of unlistens) fn();
            daemonHealth.stop();
            if (duties.renewsCredentials) {
                forgeAuth.stop();
                personaList.stop();
            }
            appState.onConfigSaved = null;
            personaList.onChanged = null;
        };
    });
</script>

<DaemonStatusBanner mode={daemonHealth.dbMode} />
<SignInOverlay
    open={overlayOpen}
    identities={personaList.identities}
    error={personaList.error ?? forgeAuth.lastError}
    loaded={personaList.loaded}
    onAct={(i) => personaList.act(i)}
    onClose={closeOverlay}
    pending={personaList.pending}
/>
{@render children()}
