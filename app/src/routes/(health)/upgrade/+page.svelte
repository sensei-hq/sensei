<script lang="ts">
    /**
     * Upgrade page — runs the post-restart upgrade steps (health resolvers + db
     * deploy). Reached when `localStorage["sensei:app-version"]` is set, i.e.
     * the app was just updated and restarted. The run — each step, what failed,
     * every line this page shows — is UpgradeRun's (audit 2026-10-28, item 13).
     */
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { hasTauri } from "$lib/bootstrap.js";
    import { Eyebrow, Kanji, StatusDot } from "$lib/components";
    import { STORAGE_KEYS } from "$lib/storage-keys.js";
    import { UpgradeRun } from "./upgrade-state.svelte.js";

    const upgrade = new UpgradeRun();

    onMount(() => {
        if (!hasTauri()) return void upgrade.start(null);
        void upgrade.start({
            listen: async (handler) => {
                const { listen } = await import("@tauri-apps/api/event");
                return listen<{ step: string; status: string; error?: string }>("upgrade", (e) => handler(e.payload));
            },
            invoke: async () => {
                const { invoke } = await import("@tauri-apps/api/core");
                return invoke("run_upgrade_steps");
            },
            finish: () => {
                localStorage.removeItem(STORAGE_KEYS.appVersion);
                void goto("/health", { replaceState: true });
            },
        });
    });
</script>

<div class="flex-1 min-h-0 overflow-hidden bg-paper-soft text-ink flex flex-col">

    <!-- ── Fixed top: header ───────────────────────────────────────────── -->
    <div class="shrink-0 pt-14 px-10">
        <div class="max-w-[640px] w-full mx-auto pb-9">

            <div class="flex items-center gap-2.5 mb-3.5">
                <Kanji char="更" size="xl" />
                <Eyebrow>upgrade · post-restart</Eyebrow>
            </div>

            <h1 class="display text-4xl font-light leading-tight mb-3.5 tracking-tight">
                {upgrade.title.lead} <span class={upgrade.title.tailClass}>{upgrade.title.tail}</span>
            </h1>

            <p class="text-sm text-ink-mute leading-loose max-w-[540px]">
                {upgrade.blurb}
            </p>
        </div>
    </div>

    <!-- ── Scrollable bottom ─────────────────────────────────────────────── -->
    <div class="flex-1 min-h-0 overflow-y-auto px-10 pb-12">
        <div class="max-w-[640px] w-full mx-auto">

            <!-- ── Hero card ─────────────────────────────────────────────── -->
            <div class="hero-card relative overflow-hidden border border-paper-edge rounded-xl bg-paper-mute p-6">
                <!-- Progress bar -->
                {#if upgrade.phase === "running"}
                    <div class="absolute top-0 left-0 right-0 h-0.5 bg-paper-mute">
                        <div
                            class="h-full bg-accent transition-all duration-700 ease-out"
                            style="width: {upgrade.progress}"
                        ></div>
                    </div>
                {/if}

                <div class="flex items-center gap-4">
                    <!-- Status indicator -->
                    <div
                        class="indicator w-14 h-14 rounded-full border-[1.5px] flex items-center justify-center shrink-0"
                        class:border-success={upgrade.indicator === "check"}
                        class:border-accent={upgrade.indicator === "warn"}
                        class:border-ink-mute={upgrade.indicator === "spinner" || upgrade.indicator === "none"}
                    >
                        {#if upgrade.indicator === "check"}
                            <span class="text-2xl text-success leading-none">✓</span>
                        {:else if upgrade.indicator === "warn"}
                            <Kanji char="△" size="xl" />
                        {:else if upgrade.indicator === "spinner"}
                            <span class="spinner-ring"></span>
                        {/if}
                    </div>

                    <!-- Status info -->
                    <div class="flex-1 min-w-0">
                        <div class="flex items-baseline gap-2.5 mb-1">
                            <span class="display text-lg font-medium">Sensei update</span>
                            <span class="font-mono text-xs text-ink-mute">health resolvers · dbd</span>
                        </div>
                        <div class="text-sm text-ink-soft leading-snug">
                            {upgrade.cardText}
                        </div>
                    </div>
                </div>
            </div>

            <!-- ── Step ledger ─────────────────────────────────────────────── -->
            <div class="mt-6">
                <div class="mb-2.5"><Eyebrow>upgrade steps</Eyebrow></div>
                <div class="flex flex-col">
                    {#each upgrade.rows as step (step.id)}
                        <div
                            class="grid grid-cols-[10px_1fr_auto] gap-3 items-center py-2 border-b border-paper-edge transition-opacity duration"
                            class:opacity-50={step.dim}
                        >
                            <StatusDot status={step.dot} />

                            <div>
                                <span class="text-sm text-ink">{step.label}</span>
                                <span class="font-mono text-xs text-ink-mute ml-2">· {step.note}</span>
                                {#if step.error}
                                    <div class="font-mono text-xs text-accent mt-0.5">{step.error}</div>
                                {/if}
                            </div>

                            <span class="font-mono text-xs tracking-wide uppercase {step.statusClass}">{step.status}</span>
                        </div>
                    {/each}
                </div>
            </div>

            <!-- ── Footer ─────────────────────────────────────────────────── -->
            <div class="flex justify-between items-center gap-4 mt-8 pt-6 border-t border-paper-edge">
                <div class="text-xs text-ink-mute leading-normal">
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
        border: 2px solid var(--ink-mute);
        border-top-color: transparent;
        animation: spin 0.9s linear infinite;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }
</style>
