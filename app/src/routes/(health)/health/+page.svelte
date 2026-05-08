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
    import { GATES } from "$lib/bootstrap-gates.js";
    import type { GateStatus } from "$lib/bootstrap-gates.js";

    // In e2e builds (VITE_SENSEI_MODE=dev baked in by globalSetup), skip auto-advance
    // so tests can observe gate states without the page redirecting under them.
    const isDevBuild = import.meta.env.VITE_SENSEI_MODE === "dev";

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

    // Auto-advance when all ready — suppressed in dev/e2e builds so tests can observe gate states
    $effect(() => {
        if (bs.allReady && !isDevBuild) {
            setTimeout(() => {
                appState.setHealthReady();
                if (appState.setupComplete)
                    goto("/observatory", { replaceState: true });
                else goto("/setup/welcome", { replaceState: true });
            }, 900);
        }
    });

    // Wire Tauri events → state and kick off the check-and-fix pipeline
    let unlistenFn: (() => void) | undefined;
    onMount(() => {
        if (!hasTauri()) return;

        (async () => {
            // Load platform info
            try {
                const info = await getPlatform();
                bs.setPlatform(info);
            } catch {
                /* browser fallback */
            }

            // Subscribe to gate/phase events before starting the engine
            unlistenFn = await listenBootstrapEvents((event) =>
                bs.handleEvent(event),
            );

            // Single call: checks all prerequisites, fixes what's broken,
            // streams progress via "bootstrap" events.
            await checkAndFixBootstrap();
        })();

        return () => unlistenFn?.();
    });

    // Retry: re-run the full check-and-fix pipeline
    async function retry(_gateId: string) {
        if (!hasTauri()) {
            // Browser-mode simulation
            bs.setGateStatus(_gateId, "checking");
            setTimeout(() => {
                bs.setGateStatus(_gateId, "ready");
                const idx = GATES.findIndex((g) => g.id === _gateId);
                if (
                    idx + 1 < GATES.length &&
                    bs.statuses[GATES[idx + 1].id] === "waiting"
                ) {
                    bs.setGateStatus(GATES[idx + 1].id, "checking");
                    setTimeout(() => {
                        GATES.slice(idx + 1).forEach((g) =>
                            bs.setGateStatus(g.id, "ready"),
                        );
                    }, 900);
                }
            }, 1100);
            return;
        }
        await checkAndFixBootstrap();
    }

    async function runInstallPrereqs() {
        if (!hasTauri()) return;
        bs.installing = true;
        await checkAndFixBootstrap();
    }

    function retryAll() {
        checkAndFixBootstrap();
    }

    function statusColor(s: GateStatus): string {
        if (s === "ready") return "oklch(var(--color-success-z6) / 1)";
        if (s === "missing" || s === "blocked")
            return "oklch(var(--color-primary-z6) / 1)";
        if (s === "checking" || s === "installing" || s === "starting")
            return "oklch(var(--color-surface-z8) / 1)";
        return "oklch(var(--color-surface-z6) / 1)";
    }

    function pillBg(s: GateStatus): string {
        if (s === "ready") return "oklch(var(--color-success-z6) / 0.10)";
        if (s === "missing" || s === "blocked")
            return "oklch(var(--color-primary-z6) / 0.08)";
        if (s === "checking" || s === "installing" || s === "starting")
            return "oklch(var(--color-surface-z2) / 1)";
        return "transparent";
    }
</script>

<div
    class="flex-1 min-h-0 overflow-hidden bg-surface-z1 text-surface-z9 flex flex-col"
>
    <!-- Fixed top: header + progress rail -->
    <div class="shrink-0 bg-surface-z1 pt-14 px-10">
        <div class="max-w-[760px] w-full mx-auto flex flex-col gap-10 pb-10">
            <!-- Header -->
            <div>
                <div class="flex items-center gap-2.5 mb-3.5">
                    <span class="kanji text-2xl text-primary-z5">支</span>
                    <span
                        class="text-2xs tracking-tag uppercase text-surface-z6"
                        >bootstrap · checking the foundation</span
                    >
                </div>
                <h1
                    class="display text-4xl font-light leading-tight mb-3.5 tracking-tight"
                >
                    {#if bs.allReady}
                        The foundation <span class="text-success-z5"
                            >holds.</span
                        >
                    {:else if bs.firstBlockedIdx >= 0}
                        A few pieces are <span class="text-primary-z5"
                            >missing.</span
                        >
                    {:else}
                        Checking the foundation…
                    {/if}
                </h1>
                <p
                    class="text-sm text-surface-z6 leading-reading max-w-[540px]"
                >
                    {#if bs.allReady}
                        Homebrew, Postgres, Ollama, sensei components, database,
                        and the daemon are all present. Opening the observatory.
                    {:else if bs.firstBlockedIdx >= 0}
                        Sensei needs these to run locally. Install the missing
                        pieces below — the rest will check themselves once the
                        foundation is in place.
                    {:else}
                        Verifying Homebrew, Postgres, Ollama, and the sensei
                        components. This takes a few seconds on a cold start.
                    {/if}
                </p>
            </div>

            <!-- Progress rail -->
            <div class="flex items-center gap-3">
                <div
                    class="text-3xs tracking-tag uppercase text-surface-z5 tabular-nums whitespace-nowrap"
                >
                    {String(bs.readyCount).padStart(2, "0")} / {String(
                        bs.totalCount,
                    ).padStart(2, "0")} ready
                </div>
                <div class="flex-1 flex gap-0.75">
                    {#each bs.gates as gate}
                        <span
                            class="flex-1 h-0.5 rounded-sm transition-colors duration-300"
                            style="background: {statusColor(
                                gate.status,
                            )}; opacity: {gate.status === 'waiting' ? 0.5 : 1};"
                        ></span>
                    {/each}
                </div>
            </div>
        </div>
    </div>

    <!-- Scrollable bottom: gate list + footer -->
    <div class="flex-1 min-h-0 overflow-y-auto px-10 pb-12">
        <div class="max-w-[760px] w-full mx-auto">
            <!-- Gate list -->
            <div class="flex flex-col border-t border-surface-z2">
                {#each bs.visibleGates as gate, i (gate.id)}
                    {@const isBlocked =
                        gate.status === "missing" || gate.status === "blocked"}
                    {@const isBusy =
                        gate.status === "checking" ||
                        gate.status === "installing" ||
                        gate.status === "starting"}
                    {@const isReady = gate.status === "ready"}
                    {@const isPending = gate.status === "waiting"}
                    {@const showRemedy = i === bs.firstBlockedIdx && isBlocked}

                    <div
                        class="gate-row border-b border-surface-z2 py-4 transition-opacity duration-300"
                        class:pending={isPending}
                    >
                        <!-- Main row -->
                        <div
                            class="grid grid-cols-[32px_1fr_auto] gap-4 items-center"
                        >
                            <div
                                class="kanji text-2xl text-center"
                                style="color: {statusColor(gate.status)};"
                            >
                                {gate.n}
                            </div>
                            <div>
                                <div class="flex items-baseline gap-2.5">
                                    <span class="display text-prose font-normal"
                                        >{gate.name}</span
                                    >
                                    <span class="text-xs text-surface-z5"
                                        >· {gate.detail}</span
                                    >
                                </div>
                                <div class="mono text-2xs text-surface-z5 mt-1">
                                    {gate.check}
                                </div>
                            </div>
                            <div
                                class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded tabular-nums text-2xs tracking-widest uppercase"
                                style="color: {statusColor(
                                    gate.status,
                                )}; background: {pillBg(gate.status)};"
                            >
                                {#if isBusy}<span class="spinner"></span>{/if}
                                {#if isReady}<span class="text-3xs">✓</span
                                    >{/if}
                                {#if isBlocked}<span class="text-xs">·</span
                                    >{/if}
                                {gate.status}
                            </div>
                        </div>

                        <!-- Sub-checks (sensei components) -->
                        {#if gate.sub && (isBusy || isBlocked || isReady)}
                            <div
                                class="sub-checks mt-3 ml-12 flex flex-col gap-1 pl-3.5"
                            >
                                {#each gate.sub as sub}
                                    {@const subStatus = isReady
                                        ? "ready"
                                        : isBusy
                                          ? "checking"
                                          : "missing"}
                                    <div class="flex items-center gap-2.5">
                                        <span
                                            class="w-1.5 h-1.5 rounded-full inline-block"
                                            style="background: {statusColor(
                                                subStatus,
                                            )};"
                                        ></span>
                                        <span class="text-xs text-surface-z7"
                                            >{sub.name}</span
                                        >
                                        <span
                                            class="mono text-2xs text-surface-z5"
                                            >{sub.check}</span
                                        >
                                    </div>
                                {/each}
                            </div>
                        {/if}

                        <!-- Per-gate remedy (non-prereq only) -->
                        {#if showRemedy && gate.remedy !== "prereq"}
                            <div
                                class="mt-4 ml-12 px-5 py-4.5 bg-surface-z2 border border-surface-z2 rounded-md"
                            >
                                {#if gate.remedy === "install"}
                                    <div class="display text-body mb-1">
                                        {bs.platformInfo.pkgmgr_remedy.title}
                                    </div>
                                    <p
                                        class="text-ui text-surface-z6 leading-relaxed mb-3.5"
                                    >
                                        {bs.platformInfo.package_manager} is the base
                                        that installs everything else.
                                    </p>
                                    <div
                                        class="bg-surface-z1 border border-surface-z2 rounded-md px-3 py-2.5 mb-3"
                                    >
                                        <code
                                            class="font-mono text-xs text-surface-z9 break-all"
                                            >{bs.platformInfo.pkgmgr_remedy
                                                .command}</code
                                        >
                                    </div>
                                    <div class="flex gap-2.5 items-center">
                                        {#if bs.platformInfo.pkgmgr_remedy.url}
                                            <a
                                                href={bs.platformInfo
                                                    .pkgmgr_remedy.url}
                                                target="_blank"
                                                rel="noreferrer"
                                                class="btn-outline btn-sm"
                                                >Learn more <span
                                                    class="text-surface-z6"
                                                    >↗</span
                                                ></a
                                            >
                                        {/if}
                                        <button
                                            class="btn-solid btn-sm"
                                            onclick={() => retry(gate.id)}
                                            >I've installed it — retry</button
                                        >
                                    </div>
                                {:else if gate.remedy === "db"}
                                    <div class="display text-body mb-1">
                                        Could not create the sensei database
                                    </div>
                                    <p
                                        class="text-ui text-surface-z6 leading-relaxed mb-3.5"
                                    >
                                        Postgres is running but sensei couldn't
                                        create its database automatically.
                                    </p>
                                    <div
                                        class="bg-surface-z1 border border-surface-z2 rounded-md px-3 py-2.5 mb-3"
                                    >
                                        <code
                                            class="font-mono text-xs text-surface-z9 break-all"
                                            >createdb sensei && psql sensei -c
                                            'CREATE EXTENSION IF NOT EXISTS
                                            vector;'</code
                                        >
                                    </div>
                                    <div class="flex gap-2.5 items-center">
                                        <button
                                            class="btn-solid btn-sm"
                                            onclick={() => retry(gate.id)}
                                            >Retry</button
                                        >
                                    </div>
                                {:else if gate.remedy === "daemon"}
                                    <div class="display text-body mb-1">
                                        Daemon failed to start
                                    </div>
                                    <p
                                        class="text-ui text-surface-z6 leading-relaxed mb-3.5"
                                    >
                                        The database is reachable but the daemon
                                        did not come up.
                                    </p>
                                    <div class="flex gap-2.5 items-center">
                                        <button
                                            class="btn-solid btn-sm"
                                            onclick={() => retry(gate.id)}
                                            >Retry</button
                                        >
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                {/each}
            </div>

            <!-- Consolidated prereq remedy -->
            {#if bs.needsPrereqInstall}
                <div
                    class="mt-6 px-6 py-5 bg-surface-z2 border border-surface-z2 rounded-md"
                >
                    <div class="display text-body mb-1">
                        {bs.platformInfo.prereq_remedy.title}
                    </div>
                    <div class="flex gap-1.5 flex-wrap mt-2 mb-3">
                        {#each bs.missingPrereqGates as gate}
                            <span
                                class="text-2xs tracking-wider uppercase text-primary-z5 px-2 py-0.75 rounded-sm"
                                style="background: oklch(var(--color-primary-z5) / 0.08);"
                                >{gate.name}</span
                            >
                        {/each}
                    </div>
                    <p class="text-ui text-surface-z6 leading-relaxed mb-3.5">
                        One command installs everything. Already-installed items
                        are skipped.
                    </p>
                    {#if hasTauri()}
                        <div class="flex gap-2.5 items-center">
                            <button
                                class="btn-solid btn-sm"
                                onclick={runInstallPrereqs}
                                disabled={bs.installing}
                            >
                                {bs.installing ? "Installing…" : "Install all"}
                            </button>
                            <button
                                class="btn-outline btn-sm"
                                onclick={retryAll}>Retry checks</button
                            >
                        </div>
                    {:else}
                        <div
                            class="bg-surface-z1 border border-surface-z2 rounded-md px-3 py-2.5 mb-3"
                        >
                            <code
                                class="font-mono text-xs text-surface-z9 break-all"
                                >{bs.platformInfo.prereq_remedy.command}</code
                            >
                        </div>
                        <div class="flex gap-2.5 items-center">
                            <button class="btn-solid btn-sm" onclick={retryAll}
                                >Retry checks</button
                            >
                        </div>
                    {/if}
                </div>
            {/if}

            <!-- Footer -->
            <div
                class="flex justify-between items-center gap-4 pt-5.5 border-t border-surface-z2"
            >
                <div class="text-2xs text-surface-z5 leading-relaxed">
                    Bootstrap runs on every launch. Once a gate is green it
                    stays that way — the next startup is quick.
                </div>
                {#if bs.allReady}
                    <button
                        class="btn-solid"
                        onclick={() => {
                            appState.setHealthReady();
                            goto(
                                appState.setupComplete
                                    ? "/observatory"
                                    : "/setup/welcome",
                                { replaceState: true },
                            );
                        }}>Continue →</button
                    >
                {/if}
            </div>
        </div>
    </div>
</div>

<style>
    /* Spinner pseudo-element + animation */
    .spinner {
        display: inline-block;
        width: 10px;
        height: 10px;
        position: relative;
    }
    .spinner::after {
        content: "";
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        border: 1.5px solid currentColor;
        border-top-color: transparent;
        border-radius: 50%;
        animation: spin 0.9s linear infinite;
    }
    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    /* Dashed sub-checks guide line — needs CSS var, can't be expressed as utility */
    .sub-checks {
        border-left: 1px dashed oklch(var(--color-surface-z3) / 1);
    }

    /* Pending gate row */
    .gate-row.pending {
        opacity: 0.42;
    }

    /* Button size override for this page */
    .btn-sm {
        padding: 8px 14px !important;
        font-size: 12px !important;
    }
</style>
