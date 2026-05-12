<script lang="ts">
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import {
        hasTauri,
        checkAndFixBootstrap,
        getPlatform,
        listenBootstrapEvents,
    } from "$lib/bootstrap.js";
    import { bootstrapState as bs } from "$lib/bootstrap-state.svelte.js";

    // Item ledger — matches BS_ITEMS order in docs/mockups/lib/bootstrap-simple.jsx
    const LEDGER_ITEMS = [
        { id: "postgres", label: "PostgreSQL @17",    note: null },
        { id: "ollama",   label: "Ollama",             note: null },
        { id: "sensei",   label: "Sensei components",  note: "cli · mcp · daemon" },
        { id: "database", label: "Database & schema",  note: "pgvector · sensei tables" },
        { id: "senseid",  label: "Background daemon",  note: null },
    ] as const;

    // Browser mode: apply mock preset
    if (!hasTauri()) {
        bs.applyPreset({
            homebrew: "ready",
            postgres: "missing",
            ollama: "missing",
            sensei: "missing",
            database: "waiting",
            senseid: "waiting",
        });
    }


    // Auto-advance when all gates are ready.
    // Disabled for now — the "Enter" / "Continue →" buttons serve this role.
    // Re-enable once we decide on the right trigger (e.g. only on first-run,
    // only in release builds, only if no dev-navigate menu was used, etc.).
    //
    // $effect(() => {
    //     if (bs.allReady) {
    //         setTimeout(() => enterApp(), 900);
    //     }
    // });

    // Wire Tauri events → state and kick off the check-and-fix pipeline
    let unlistenFn: (() => void) | undefined;
    onMount(() => {
        if (!hasTauri()) return;

        (async () => {
            try {
                const info = await getPlatform();
                bs.setPlatform(info);
            } catch { /* browser fallback */ }

            unlistenFn = await listenBootstrapEvents((event) =>
                bs.handleEvent(event),
            );
            await checkAndFixBootstrap();
        })();

        return () => unlistenFn?.();
    });

    async function retryAll() {
        if (!hasTauri()) return;
        await checkAndFixBootstrap();
    }

    function enterApp() {
        appState.setHealthReady();
        // Always navigate to /observatory — the reroute guard in hooks.client.ts
        // will redirect to /setup/welcome if setup isn't complete yet.
        goto("/observatory", { replaceState: true });
    }

    // ── Derived visual state ──────────────────────────────────────────────────

    const isGreen  = $derived(bs.simpleState === "all-green");
    const isManual = $derived(bs.simpleState === "manual");
    const isFixing = $derived(
        bs.simpleState === "auto-fixing" || bs.simpleState === "detecting"
    );

    // Hero card: homebrew sub-states
    const hwReady   = $derived(bs.homebrewStatus === "ready");
    const hwBusy    = $derived(bs.homebrewStatus === "checking" || bs.homebrewStatus === "installing");
    const hwMissing = $derived(bs.homebrewStatus === "missing" || bs.homebrewStatus === "blocked");

    // Progress bar width (0–100)
    const progress = $derived((bs.readyCount / bs.totalCount) * 100);

    // Active ledger item label while fixing
    const activeItemLabel = $derived(
        LEDGER_ITEMS[Math.min(bs.activeSimpleItemIdx, LEDGER_ITEMS.length - 1)].label
    );
    const activeItemCount = $derived(
        Math.min(bs.activeSimpleItemIdx + 1, LEDGER_ITEMS.length)
    );

    // ── Script card state ────────────────────────────────────────────────────

    let copied = $state(false);
    let scriptOpen = $state(true);

    function copyScript() {
        const cmd = bs.platformInfo.prereq_remedy.command;
        navigator.clipboard?.writeText(cmd).catch(() => {});
        copied = true;
        setTimeout(() => (copied = false), 1800);
    }

    // Per-item visual state
    type ItemState = "ready" | "running" | "blocked" | "pending";
    function itemState(id: string): ItemState {
        const s = bs.statuses[id];
        if (s === "ready") return "ready";
        if (s === "checking" || s === "installing" || s === "starting") return "running";
        if (s === "missing" || s === "blocked") return "blocked";
        return "pending";
    }
</script>

<div class="flex-1 min-h-0 overflow-hidden bg-surface-z1 text-surface-z9 flex flex-col">

    <!-- ── Fixed top: header ───────────────────────────────────────────── -->
    <div class="shrink-0 pt-14 px-10">
        <div class="max-w-[640px] w-full mx-auto pb-9">

            <div class="flex items-center gap-2.5 mb-3.5">
                <span class="kanji text-xl text-primary-z5">支</span>
                <span class="text-2xs tracking-tag uppercase text-surface-z6">
                    bootstrap · {bs.platformInfo.platform}
                </span>
            </div>

            <h1 class="display text-4xl font-light leading-tight mb-3.5 tracking-tight">
                {#if isGreen}
                    The foundation <span class="text-success-z5">holds.</span>
                {:else if bs.simpleState === "auto-fixing"}
                    Setting up your <span class="text-primary-z5">foundation.</span>
                {:else if bs.simpleState === "detecting"}
                    Checking the <span class="text-surface-z7">foundation…</span>
                {:else}
                    One last <span class="text-primary-z5">step.</span>
                {/if}
            </h1>

            <p class="text-sm text-surface-z6 leading-reading max-w-[540px]">
                {#if isGreen}
                    Everything sensei needs is here. Opening the observatory.
                {:else if bs.simpleState === "auto-fixing"}
                    Running <span class="mono text-surface-z7">{bs.platformInfo.package_manager.toLowerCase()}</span>
                    with the manifest from <span class="mono text-surface-z7">sensei-hq/homebrew-tap</span>. No input needed.
                {:else if bs.simpleState === "detecting"}
                    Verifying {bs.platformInfo.package_manager}, Postgres, Ollama, and the sensei
                    components. This takes a few seconds on a cold start.
                {:else if bs.needsPrereqInstall}
                    {bs.platformInfo.package_manager} isn't installed. Run the script below —
                    it installs {bs.platformInfo.package_manager} first, then everything else.
                {:else}
                    {bs.platformInfo.package_manager} is here, but hit a problem finishing the setup.
                    The script below runs the same steps manually.
                {/if}
            </p>
        </div>
    </div>

    <!-- ── Scrollable bottom ─────────────────────────────────────────────── -->
    <div class="flex-1 min-h-0 overflow-y-auto px-10 pb-12">
        <div class="max-w-[640px] w-full mx-auto">

            <!-- ── Hero card ─────────────────────────────────────────────── -->
            <div class="relative overflow-hidden border border-surface-z2 rounded-xl bg-surface-z2 p-6.5">

                <!-- Progress bar across the top while fixing -->
                {#if isFixing}
                    <div class="absolute top-0 left-0 right-0 h-0.5 bg-surface-z3">
                        <div
                            class="h-full bg-primary-z5 transition-all duration-700 ease-out"
                            style="width: {progress}%"
                        ></div>
                    </div>
                {/if}

                <div class="flex items-center gap-4.5">

                    <!-- Status indicator circle -->
                    <div
                        class="w-14 h-14 rounded-full border-[1.5px] flex items-center justify-center shrink-0"
                        class:border-success-z5={isGreen}
                        class:border-primary-z5={isManual || hwMissing}
                        class:border-surface-z5={isFixing}
                    >
                        {#if isGreen}
                            <span class="text-2xl text-success-z5 leading-none">✓</span>
                        {:else if isManual || hwMissing}
                            <span class="kanji text-xl text-primary-z5">?</span>
                        {:else}
                            <span class="spinner-ring"></span>
                        {/if}
                    </div>

                    <!-- Package manager info -->
                    <div class="flex-1 min-w-0">
                        <div class="flex items-baseline gap-2.5 mb-1">
                            <span class="display text-prose font-medium">
                                {bs.platformInfo.package_manager}
                            </span>
                            <span class="mono text-2xs text-surface-z5">
                                which {bs.platformInfo.package_manager.toLowerCase()}
                            </span>
                        </div>
                        <div class="text-sm text-surface-z7 leading-snug">
                            {#if isGreen}
                                Detected. All dependencies installed.
                            {:else if isManual || hwMissing}
                                Couldn't finish automatically. Run the script below.
                            {:else if hwReady || hwBusy}
                                Detected. Installing
                                <span class="text-surface-z9">{activeItemLabel}</span>
                                <span class="mono text-2xs text-surface-z5 ml-2">
                                    ({activeItemCount}/{LEDGER_ITEMS.length})
                                </span>
                            {:else}
                                Checking system…
                            {/if}
                        </div>
                    </div>

                    <!-- Enter button (all-green) -->
                    {#if isGreen}
                        <button class="btn-solid shrink-0" onclick={enterApp}>
                            Enter
                        </button>
                    {/if}

                </div>
            </div>

            <!-- ── Script card (manual fallback) ─────────────────────────── -->
            {#if isManual}
                <div class="mt-4.5 border border-primary-z5/30 rounded-xl bg-surface-z1 overflow-hidden">
                    <div class="flex items-center gap-2.5 px-4.5 py-3.5 border-b border-surface-z2">
                        <span class="kanji text-base text-primary-z5">手</span>
                        <div class="flex-1">
                            <div class="text-sm text-surface-z9">Run this in your terminal</div>
                            <div class="text-2xs text-surface-z5 mt-0.5">
                                Same steps sensei would run · {bs.platformInfo.platform}
                            </div>
                        </div>
                        <button
                            class="text-2xs text-surface-z6 px-2 py-1 border border-surface-z3 rounded hover:bg-surface-z2 transition-colors"
                            onclick={() => (scriptOpen = !scriptOpen)}
                        >{scriptOpen ? "Hide" : "Show"}</button>
                    </div>

                    {#if scriptOpen}
                        <pre class="m-0 px-4.5 py-4 mono text-xs text-surface-z9 bg-surface-z1 leading-relaxed whitespace-pre-wrap break-words max-h-56 overflow-auto">{bs.platformInfo.prereq_remedy.command}</pre>
                        <div class="flex items-center justify-between gap-2.5 px-4.5 py-3 border-t border-surface-z2">
                            <button class="btn-solid btn-sm" onclick={copyScript}>
                                {copied ? "Copied ✓" : "Copy script"}
                            </button>
                            <button
                                class="btn-outline btn-sm"
                                style="color: oklch(var(--color-primary-z5) / 1); border-color: oklch(var(--color-primary-z5) / 0.4);"
                                onclick={retryAll}
                            >
                                I've run it · re-check
                            </button>
                        </div>
                    {/if}
                </div>
            {/if}

            <!-- ── Item ledger ─────────────────────────────────────────────── -->
            <div class="mt-5.5">
                <div class="text-2xs tracking-tag uppercase text-surface-z5 mb-2.5">
                    what this resolves
                </div>
                <div class="flex flex-col">
                    {#each LEDGER_ITEMS as item}
                        {@const s = itemState(item.id)}
                        <div
                            class="grid grid-cols-[10px_1fr_auto] gap-3 items-center py-2 border-b border-surface-z2 transition-opacity duration-200"
                            style="opacity: {s === 'pending' ? 0.45 : 1}"
                        >
                            <span
                                class="w-2 h-2 rounded-full shrink-0"
                                style="background: {
                                    s === 'ready'   ? 'oklch(var(--color-success-z5) / 1)' :
                                    s === 'running' ? 'oklch(var(--color-primary-z5) / 1)' :
                                    s === 'blocked' ? 'oklch(var(--color-primary-z5) / 0.5)' :
                                    'oklch(var(--color-surface-z4) / 1)'
                                }"
                            ></span>
                            <div>
                                <span class="text-sm text-surface-z9">{item.label}</span>
                                {#if item.note}
                                    <span class="text-xs text-surface-z5 ml-2">· {item.note}</span>
                                {/if}
                            </div>
                            <span
                                class="mono text-2xs tracking-wider uppercase"
                                style="color: {
                                    s === 'ready'   ? 'oklch(var(--color-success-z5) / 1)' :
                                    s === 'running' ? 'oklch(var(--color-primary-z5) / 1)' :
                                    'oklch(var(--color-surface-z5) / 1)'
                                }"
                            >
                                {s === "running" ? (bs.statuses[item.id] ?? s) : s}
                            </span>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- ── Footer ─────────────────────────────────────────────────── -->
            <div class="flex justify-between items-center gap-4 mt-8 pt-5.5 border-t border-surface-z2">
                <div class="text-2xs text-surface-z5 leading-relaxed">
                    Bootstrap runs once. The next launch will be quick.
                </div>
                {#if isGreen}
                    <button class="btn-solid" onclick={enterApp}>Continue →</button>
                {/if}
            </div>

        </div>
    </div>
</div>

<style>
    .spinner-ring {
        display: block;
        width: 20px;
        height: 20px;
        border-radius: 50%;
        border: 2px solid oklch(var(--color-surface-z5) / 1);
        border-top-color: transparent;
        animation: spin 0.9s linear infinite;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }

    .btn-sm {
        padding: 8px 14px !important;
        font-size: 12px !important;
    }
</style>
