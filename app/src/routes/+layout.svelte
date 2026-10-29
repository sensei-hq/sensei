<script lang="ts">
    import "uno.css";
    import "../app.css";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { hasTauri } from "$lib/bootstrap.js";
    import { appState } from "$lib/appstate.svelte.js";
    import { healthState } from "$lib/health-state.svelte.js";
    import { daemonHealth } from "$lib/daemon-health.svelte.js";
    import { glyphMode, setGlyphMode } from "$lib/glyphs";
    import { personaList } from "$lib/personas.svelte.js";
    import { DaemonStatusBanner } from "$lib/components";
    import { windowDuties } from "$lib/window-duties.js";
    import { openSettingsWindow } from "$lib/stores/windows.svelte.js";
    import { AlertList } from "@rokkit/ui";
    import { alerts } from "@rokkit/states";

    let { children } = $props();

    // The glyph preference, for every mark below (#133).
    setGlyphMode(() => glyphMode(appState.config));

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

    // An identity that needs you pauses sync but not sensei: no screen-covering
    // overlay (user, 2026-10-29). The sidebar footer says so while it lasts, and a
    // NEW need is announced once, in the main window, as a toast that fades —
    // with the way to Settings · Dōjō, where every identity has its action.
    const openIdentities = () => void openSettingsWindow("/settings/dojo");

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

        // The identity state: every identity's standing, and — in the main window
        // only — the one automatic re-authorization per renewal window (renewal
        // runs the authorize flow in a browser; see PersonaList).
        //
        // Tauri-only. A browser dev session has no opener plugin, and polling
        // there would report a standing nobody can act on.
        if (!hasTauri()) return () => daemonHealth.stop();
        if (duties.announcesIdentities) {
            personaList.onAttention = (text) =>
                void alerts.push({ type: "warning", text, timeout: 8000, actions: openSettingsAction });
        }
        if (duties.renewsCredentials) {
            personaList.renews = true;
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
                listen<void>("open-identities", openIdentities).then((fn) => unlistens.push(fn));
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
            // The same for identities: the sidebar footer (main) and Settings · Dōjō
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
                personaList.renews = false;
                personaList.stop();
            }
            appState.onConfigSaved = null;
            personaList.onChanged = null;
            personaList.onAttention = null;
        };
    });
</script>

{#snippet openSettingsAction()}
    <button type="button" data-open-identities onclick={openIdentities}>Open Settings</button>
{/snippet}

<DaemonStatusBanner mode={daemonHealth.dbMode} configError={appState.configError} />
{#if duties.announcesIdentities}
    <AlertList position="bottom-right" />
{/if}
{@render children()}
