<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { PageHeader, Switch } from "$lib/components";
    import { Select } from "@rokkit/ui";
    import type { PreferencesData } from "$lib/setup/contracts.js";
    import {
        DEFAULT_PREFERENCES,
        fromPreferencesForm,
        toPreferencesForm,
    } from "../preferences-form.js";

    // Static option lists for the preference pickers (label → wire value).
    const DIGEST_OPTIONS = [
        { label: "Off", value: "off" },
        { label: "Daily", value: "daily" },
        { label: "Weekly", value: "weekly" },
    ];
    const TONE_OPTIONS = [
        { label: "Gentle", value: "gentle" },
        { label: "Balanced", value: "balanced" },
        { label: "Direct", value: "direct" },
    ];

    let prefs = $state<PreferencesData>({ ...DEFAULT_PREFERENCES });
    let loading = $state(true);
    let saveStatus = $state<"idle" | "saving" | "saved" | "error">("idle");
    let saveError = $state<string | null>(null);
    let saveTimer: ReturnType<typeof setTimeout> | null = null;

    onMount(async () => {
        const cfg = await senseiApi(appState.port).getConfig();
        prefs = toPreferencesForm(cfg);
        loading = false;
    });

    // Boolean toggles auto-persist; text/select persist from their handlers.
    // Gated on `loading` so the initial hydrate does not spuriously re-save.
    $effect(() => {
        void prefs.nudgeOnRegression;
        void prefs.anonymizedTelemetry;
        void prefs.showWelcome;
        if (!loading) void persist();
    });

    async function persist() {
        if (saveTimer) {
            clearTimeout(saveTimer);
            saveTimer = null;
        }
        saveStatus = "saving";
        saveError = null;
        try {
            const result = await senseiApi(appState.port).trySetConfig(
                fromPreferencesForm(prefs),
            );
            if (result.ok) {
                saveStatus = "saved";
                saveTimer = setTimeout(() => {
                    if (saveStatus === "saved") saveStatus = "idle";
                }, 1500);
            } else {
                saveStatus = "error";
                saveError = result.error.message;
            }
        } catch (e) {
            saveStatus = "error";
            saveError = (e as Error).message;
        }
    }
</script>

<PageHeader kanji="名" eyebrow="Settings" title="General" />
<div class="max-w-[720px] mx-auto px-12 pt-8 pb-16">
    {#if loading}
        <p class="text-sm text-ink-soft leading-normal">Loading…</p>
    {:else}
        <div
            class="px-7 py-7 bg-paper-mute border border-paper-mute rounded-lg"
            data-testid="settings-general"
        >
            <div class="flex items-baseline justify-between mb-4">
                <h3 class="text-base m-0">Preferences</h3>
                <span
                    class="text-xs"
                    class:text-ink-mute={saveStatus === "idle"}
                    class:text-ink-soft={saveStatus === "saving"}
                    class:text-success={saveStatus === "saved"}
                    class:text-warning={saveStatus === "error"}
                    data-testid="settings-save-status"
                >
                    {#if saveStatus === "saving"}saving…
                    {:else if saveStatus === "saved"}saved
                    {:else if saveStatus === "error"}{saveError ?? "save failed"}
                    {:else}auto-saves as you edit
                    {/if}
                </span>
            </div>

            <div class="flex flex-col divide-y divide-paper-edge">
                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Display name</div>
                        <div class="text-xs text-ink-mute mt-0.5">
                            Used in the morning digest and when raising a teaching.
                        </div>
                    </div>
                    <input
                        type="text"
                        class="w-[220px] px-3 py-2 text-sm border border-paper-mute rounded-md bg-paper-soft text-ink outline-none text-right"
                        data-testid="pref-display-name"
                        value={prefs.displayName}
                        oninput={(e) => {
                            prefs.displayName = e.currentTarget.value;
                            void persist();
                        }}
                        placeholder="your name"
                    />
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Morning digest</div>
                        <div class="text-xs text-ink-mute mt-0.5">
                            The Today view cadence. Off keeps the dashboard quiet.
                        </div>
                    </div>
                    <!-- testid on wrapper: rokkit Select doesn't forward data-* to the DOM. -->
                    <div data-testid="pref-digest-cadence">
                        <Select
                            size="sm"
                            items={DIGEST_OPTIONS}
                            value={prefs.digestCadence}
                            onchange={(v) => {
                                prefs.digestCadence = v as string;
                                void persist();
                            }}
                        />
                    </div>
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Correction tone</div>
                        <div class="text-xs text-ink-mute mt-0.5">
                            How direct sensei is when something repeats.
                        </div>
                    </div>
                    <div data-testid="pref-correction-aggressiveness">
                        <Select
                            size="sm"
                            items={TONE_OPTIONS}
                            value={prefs.correctionAggressiveness}
                            onchange={(v) => {
                                prefs.correctionAggressiveness = v as string;
                                void persist();
                            }}
                        />
                    </div>
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Nudge on regression</div>
                        <div class="text-xs text-ink-mute mt-0.5">
                            If FTR drops sharply on a project, sensei surfaces it on Today.
                        </div>
                    </div>
                    <Switch
                        bind:value={prefs.nudgeOnRegression}
                        label="Toggle nudge on regression"
                    />
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Anonymized telemetry</div>
                        <div class="text-xs text-ink-mute mt-0.5">
                            Crashes, performance, which views you visit. Never code or session content.
                        </div>
                    </div>
                    <Switch
                        bind:value={prefs.anonymizedTelemetry}
                        label="Toggle anonymized telemetry"
                    />
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Show welcome greeting</div>
                        <div class="text-xs text-ink-mute mt-0.5">
                            The daily greeting toast on the observatory.
                        </div>
                    </div>
                    <Switch
                        bind:value={prefs.showWelcome}
                        label="Toggle welcome greeting"
                    />
                </div>
            </div>
        </div>
    {/if}
</div>
