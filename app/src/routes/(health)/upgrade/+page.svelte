<script lang="ts">
    /**
     * Upgrade page — runs post-restart upgrade steps (health resolvers + db deploy).
     *
     * Reached when `localStorage["sensei:app-version"]` is set, which means the
     * app was just updated by tauri-plugin-updater and restarted. This page:
     *   1. Calls run_upgrade_steps (health resolvers + db deploy)
     *   2. Streams progress via the "upgrade" Tauri event channel
     *   3. On completion, clears the flag and redirects to /health
     */
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { hasTauri } from "$lib/bootstrap.js";

    type StepStatus = "pending" | "running" | "done" | "failed";

    interface UpgradeStep {
        id: "prereqs" | "db_deploy";
        label: string;
        note: string;
    }

    const STEPS: UpgradeStep[] = [
        { id: "prereqs",   label: "Prerequisite upgrades",     note: "health resolvers" },
        { id: "db_deploy", label: "Database schema deploy",    note: "dbd deploy" },
    ];

    let stepStatuses = $state<Record<string, StepStatus>>({
        prereqs:   "pending",
        db_deploy: "pending",
    });
    let stepErrors = $state<Record<string, string>>({});
    let done = $state(false);
    let anyFailed = $state(false);

    $effect(() => {
        if (done) {
            // Give the user a moment to see the result, then continue
            setTimeout(() => {
                // Clear the upgrade-pending flag
                if (typeof localStorage !== "undefined") {
                    localStorage.removeItem("sensei:app-version");
                }
                goto("/health", { replaceState: true });
            }, 1200);
        }
    });

    // Derived state for the hero card
    const isRunning = $derived(Object.values(stepStatuses).some(s => s === "running"));
    const isComplete = $derived(done);
    const activeStepLabel = $derived(
        STEPS.find(s => stepStatuses[s.id] === "running")?.label ?? "Preparing…"
    );
    const doneCount = $derived(
        STEPS.filter(s => stepStatuses[s.id] === "done" || stepStatuses[s.id] === "failed").length
    );
    const progress = $derived((doneCount / STEPS.length) * 100);

    onMount(() => {
        if (!hasTauri()) {
            // Browser simulation: walk steps with delays
            simulateBrowserUpgrade();
            return;
        }

        runUpgradeSteps();
    });

    async function runUpgradeSteps() {
        const { listen } = await import("@tauri-apps/api/event");
        const { invoke } = await import("@tauri-apps/api/core");

        const unlisten = await listen<{
            step: string;
            status: string;
            error?: string;
        }>("upgrade", (event) => {
            const { step, status, error } = event.payload;
            if (step === "complete") {
                done = true;
                return;
            }
            stepStatuses = { ...stepStatuses, [step]: status as StepStatus };
            if (status === "failed" && error) {
                stepErrors = { ...stepErrors, [step]: error };
                anyFailed = true;
            }
        });

        try {
            await invoke("run_upgrade_steps");
        } catch (e) {
            done = true;
        }

        return unlisten;
    }

    function simulateBrowserUpgrade() {
        setTimeout(() => {
            stepStatuses = { ...stepStatuses, prereqs: "running" };
            setTimeout(() => {
                stepStatuses = { ...stepStatuses, prereqs: "done", db_deploy: "running" };
                setTimeout(() => {
                    stepStatuses = { ...stepStatuses, db_deploy: "done" };
                    done = true;
                }, 1400);
            }, 1600);
        }, 400);
    }

    function stepState(id: string): StepStatus {
        return stepStatuses[id] as StepStatus;
    }
</script>

<div class="flex-1 min-h-0 overflow-hidden bg-surface-z1 text-surface-z9 flex flex-col">

    <!-- ── Fixed top: header ───────────────────────────────────────────── -->
    <div class="shrink-0 pt-14 px-10">
        <div class="max-w-[640px] w-full mx-auto pb-9">

            <div class="flex items-center gap-2.5 mb-3.5">
                <span class="kanji text-xl text-primary-z5">更</span>
                <span class="text-2xs tracking-tag uppercase text-surface-z6">
                    upgrade · post-restart
                </span>
            </div>

            <h1 class="display text-4xl font-light leading-tight mb-3.5 tracking-tight">
                {#if isComplete && !anyFailed}
                    Up to date. <span class="text-success-z5">Resuming.</span>
                {:else if isComplete && anyFailed}
                    Partial upgrade. <span class="text-primary-z5">Continuing.</span>
                {:else}
                    Upgrading <span class="text-primary-z5">sensei.</span>
                {/if}
            </h1>

            <p class="text-sm text-surface-z6 leading-reading max-w-[540px]">
                {#if isComplete && !anyFailed}
                    Homebrew and the database schema are up to date. Running health checks now.
                {:else if isComplete && anyFailed}
                    Some steps had issues. Health checks will run — if anything is missing you can fix it there.
                {:else}
                    Applying the update: upgrading packages via Homebrew and deploying the latest schema.
                    No input needed.
                {/if}
            </p>
        </div>
    </div>

    <!-- ── Scrollable bottom ─────────────────────────────────────────────── -->
    <div class="flex-1 min-h-0 overflow-y-auto px-10 pb-12">
        <div class="max-w-[640px] w-full mx-auto">

            <!-- ── Hero card ─────────────────────────────────────────────── -->
            <div class="hero-card relative overflow-hidden border border-surface-z2 rounded-xl bg-surface-z2 p-6.5">
                <!-- Progress bar -->
                {#if !isComplete}
                    <div class="absolute top-0 left-0 right-0 h-0.5 bg-surface-z3">
                        <div
                            class="h-full bg-primary-z5 transition-all duration-700 ease-out"
                            style="width: {progress}%"
                        ></div>
                    </div>
                {/if}

                <div class="flex items-center gap-4.5">
                    <!-- Status indicator -->
                    <div
                        class="indicator w-14 h-14 rounded-full border-[1.5px] flex items-center justify-center shrink-0"
                        class:border-success-z5={isComplete && !anyFailed}
                        class:border-primary-z5={isComplete && anyFailed}
                        class:border-surface-z5={!isComplete}
                    >
                        {#if isComplete && !anyFailed}
                            <span class="text-2xl text-success-z5 leading-none">✓</span>
                        {:else if isComplete && anyFailed}
                            <span class="kanji text-xl text-primary-z5">△</span>
                        {:else}
                            <span class="spinner-ring"></span>
                        {/if}
                    </div>

                    <!-- Status info -->
                    <div class="flex-1 min-w-0">
                        <div class="flex items-baseline gap-2.5 mb-1">
                            <span class="display text-prose font-medium">Sensei update</span>
                            <span class="mono text-2xs text-surface-z5">health resolvers · dbd</span>
                        </div>
                        <div class="text-sm text-surface-z7 leading-snug">
                            {#if isComplete && !anyFailed}
                                All upgrade steps completed.
                            {:else if isComplete && anyFailed}
                                Completed with warnings. Continuing to health check.
                            {:else}
                                {activeStepLabel}…
                                <span class="mono text-2xs text-surface-z5 ml-2">
                                    ({doneCount + 1}/{STEPS.length})
                                </span>
                            {/if}
                        </div>
                    </div>
                </div>
            </div>

            <!-- ── Step ledger ─────────────────────────────────────────────── -->
            <div class="mt-5.5">
                <div class="text-2xs tracking-tag uppercase text-surface-z5 mb-2.5">
                    upgrade steps
                </div>
                <div class="flex flex-col">
                    {#each STEPS as step}
                        {@const s = stepState(step.id)}
                        <div
                            class="grid grid-cols-[10px_1fr_auto] gap-3 items-center py-2 border-b border-surface-z2 transition-opacity duration-200"
                            class:opacity-50={s === "pending"}
                        >
                            <span
                                class="w-2 h-2 rounded-full shrink-0 transition-colors duration-300"
                                class:bg-success-z5={s === "done"}
                                class:bg-primary-z5={s === "running" || s === "failed"}
                                class:bg-surface-z4={s === "pending"}
                            ></span>

                            <div>
                                <span class="text-sm text-surface-z9">{step.label}</span>
                                <span class="mono text-2xs text-surface-z5 ml-2">· {step.note}</span>
                                {#if s === "failed" && stepErrors[step.id]}
                                    <div class="mono text-2xs text-primary-z5 mt-0.5">
                                        {stepErrors[step.id]}
                                    </div>
                                {/if}
                            </div>

                            <span
                                class="mono text-2xs tracking-wider uppercase"
                                class:text-success-z5={s === "done"}
                                class:text-primary-z5={s === "running" || s === "failed"}
                                class:text-surface-z5={s === "pending"}
                            >
                                {s}
                            </span>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- ── Footer ─────────────────────────────────────────────────── -->
            <div class="flex justify-between items-center gap-4 mt-8 pt-5.5 border-t border-surface-z2">
                <div class="text-2xs text-surface-z5 leading-relaxed">
                    Upgrade steps run once after each update. The next launch will be quick.
                </div>
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
</style>
