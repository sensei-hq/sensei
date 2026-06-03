<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { healthState } from "$lib/health-state.svelte.js";
    import type { Component, ComponentStatus, HealthStatus } from "$lib/health-types.js";
    import HealthView from "./HealthView.svelte";

    // `auto` controls whether the page navigates away on its own when health
    // reaches ok. Default true (cold-start bootstrap flow). Set `?auto=false`
    // for an inspectable view — e.g., the View → Health menu entry, or for
    // visual QA. When auto is false, HealthView shows a Continue button.
    let auto = $state(true);

    onMount(() => {
        if (typeof window !== "undefined") {
            const params = new URLSearchParams(window.location.search);
            // Explicit override via query param.
            if (params.get("auto") === "false") auto = false;
            // Fixture mode and inspect seam also imply non-auto (the user is
            // intentionally sitting on the screen, not flowing through).
            if (params.has("state") || params.has("inspect")) auto = false;
        }

        const fixture = readFixtureFromQuery();
        if (fixture) {
            applyFixture(fixture);
        } else {
            healthState.init();
        }
    });

    // Auto-leave the health page once the gate is green. The reroute hook
    // then decides whether to land at /setup/welcome (setup not complete)
    // or / (observatory). Only fires when `auto` is true.
    $effect(() => {
        if (!auto) return;
        if (healthState.isOk) {
            goto("/", { replaceState: true });
        }
    });

    function onEnter() {
        goto("/", { replaceState: true });
    }
    function onVerify() {
        healthState.verify();
    }

    // ─── Visual-inspection fixtures ──────────────────────────────────────
    // `/health?state=checking|resolving|needs-action|ok` drives the screen
    // through the four HealthStatus shapes without needing the daemon to be
    // in that state. Dev-only seam — never relied on in production code.

    function readFixtureFromQuery(): HealthStatus | null {
        if (typeof window === "undefined") return null;
        const v = new URLSearchParams(window.location.search).get("state");
        if (v === "checking" || v === "resolving" || v === "needs-action" || v === "ok") return v;
        return null;
    }

    const ZEN: Record<string, string> = {
        homebrew: "The gardener who tends the tools.",
        postgres: "A still pond where memories settle.",
        ollama:   "A mind that thinks without leaving the room.",
        sensei:   "Three hands of the practice — speak, listen, attend.",
        database: "Shelves shaped to the form of each memory.",
        daemon:   "The quiet breath that keeps watch.",
    };

    function gate(
        id: Component["id"],
        label: string,
        detail: string,
        status: ComponentStatus,
        version: string | null = null,
        installingVerb = "installing",
    ): Component {
        return {
            id, label, detail, note: null, status, version,
            installingVerb,
            description: ZEN[id] ?? "",
        };
    }

    function applyFixture(state: HealthStatus) {
        healthState.platform = "macos";
        healthState.version  = "0.2.16";

        switch (state) {
            case "checking":
                healthState.status = "checking";
                healthState.packageManager = gate("homebrew", "Homebrew", "package manager", "checking");
                healthState.components = [
                    gate("postgres", "PostgreSQL",        "storage · @16",       "pending"),
                    gate("ollama",   "Ollama",            "local models",        "pending"),
                    gate("sensei",   "Sensei components", "cli · mcp · daemon",  "pending"),
                    gate("database", "Database",          "schema · pgvector",   "pending"),
                    gate("daemon",   "Daemon",            "background",          "pending"),
                ];
                healthState.remedy = null;
                break;

            case "resolving":
                healthState.status = "resolving";
                healthState.packageManager = gate("homebrew", "Homebrew", "package manager", "ready", "4.4.0");
                healthState.components = [
                    gate("postgres", "PostgreSQL",        "storage · @16",       "ready", "16.4"),
                    gate("ollama",   "Ollama",            "local models",        "installing", null, "installing"),
                    gate("sensei",   "Sensei components", "cli · mcp · daemon",  "pending"),
                    gate("database", "Database",          "schema · pgvector",   "pending"),
                    gate("daemon",   "Daemon",            "background",          "pending"),
                ];
                healthState.remedy = null;
                break;

            case "needs-action":
                healthState.status = "needs-action";
                healthState.packageManager = gate("homebrew", "Homebrew", "package manager", "failed");
                healthState.components = [
                    gate("postgres", "PostgreSQL",        "storage · @16",       "pending"),
                    gate("ollama",   "Ollama",            "local models",        "pending"),
                    gate("sensei",   "Sensei components", "cli · mcp · daemon",  "pending"),
                    gate("database", "Database",          "schema · pgvector",   "pending"),
                    gate("daemon",   "Daemon",            "background",          "pending"),
                ];
                healthState.remedy = {
                    message: "Homebrew isn't here yet. Run the script to install it, then re-check.",
                    script:
                        '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"\n' +
                        "brew bundle --file=https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile",
                    url: "https://brew.sh",
                };
                break;

            case "ok":
                healthState.status = "ok";
                healthState.packageManager = gate("homebrew", "Homebrew", "package manager", "ready", "4.4.0");
                healthState.components = [
                    gate("postgres", "PostgreSQL",        "storage · @16",       "ready", "16.4"),
                    gate("ollama",   "Ollama",            "local models",        "ready", "0.3.10"),
                    gate("sensei",   "Sensei components", "cli · mcp · daemon",  "ready", "0.2.16"),
                    gate("database", "Database",          "schema · pgvector",   "ready"),
                    gate("daemon",   "Daemon",            "background",          "ready", "0.2.16"),
                ];
                healthState.remedy = null;
                break;
        }
    }
    // Clipboard copy is owned by Remedy.svelte now — it tracks success/failure
    // state internally and shows feedback in the button label. The parent
    // no longer needs to drive it.
</script>

<HealthView state={healthState} {auto} {onEnter} {onVerify} />
