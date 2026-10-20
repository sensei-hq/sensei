<script lang="ts">
    import { railIcon } from "../settings-nav";
    import { onMount, untrack } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { PageHeader, Screen, ScreenState, Switch } from "$lib/components";
    import { Select } from "@rokkit/ui";
    import type { PreferencesData } from "$lib/setup/contracts.js";
    import {
        DEFAULT_PREFERENCES,
        fromPreferencesForm,
    } from "../preferences-form.js";
    import { readPreferences } from "../settings-reads";
    import { GLYPH_OPTIONS, type GlyphMode } from "$lib/glyphs";

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

    // Why the saved preferences could not be read (#288). While set there is no
    // form — and nothing auto-saves: a form of defaults would be written back
    // over the user's real settings.
    let loadError = $state<string | null>(null);

    async function load() {
        loading = true;
        const read = await readPreferences(senseiApi(appState.port));
        loadError = read.error;
        if (read.prefs) prefs = read.prefs;
        loading = false;
    }
    onMount(load);

    // Boolean toggles auto-persist; text/select persist from their handlers.
    // Gated on `loading` so the initial hydrate does not spuriously re-save.
    $effect(() => {
        void prefs.nudgeOnRegression;
        void prefs.anonymizedTelemetry;
        void prefs.showWelcome;
        // Untracked: saving reads and writes appState.config, and tracking that
        // made every save re-run this effect — an endless write loop.
        if (!loading && !loadError) untrack(() => void persist());
    });

    async function persist() {
        // Never save a form that was not read from the daemon (#288).
        if (loadError) return;
        if (saveTimer) {
            clearTimeout(saveTimer);
            saveTimer = null;
        }
        saveStatus = "saving";
        saveError = null;
        try {
            // Through appState, so the cached config — which the rails read
            // their glyph mode from (#133) — changes with the daemon's.
            await appState.setConfigs(fromPreferencesForm(prefs));
            saveStatus = "saved";
            saveTimer = setTimeout(() => {
                if (saveStatus === "saved") saveStatus = "idle";
            }, 1500);
        } catch (e) {
            saveStatus = "error";
            saveError = (e as Error).message;
        }
    }
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="名" icon={railIcon("/settings/general")} eyebrow="Settings" title="General" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16">
        {#if loading}
            <p class="text-sm text-ink-soft leading-normal">Loading…</p>
        {:else if loadError}
            <ScreenState status="error" error={loadError} onretry={() => void load()} />
        {:else}
            <div
                class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
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
                            class="w-[220px] px-3 py-2 text-sm border border-paper-edge rounded-md bg-paper-soft text-ink outline-none text-right"
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
                            <div class="text-sm text-ink">Glyphs</div>
                            <div class="text-xs text-ink-mute mt-0.5">
                                Kanji or icons beside each section, across the app.
                            </div>
                        </div>
                        <div data-testid="pref-glyphs">
                            <Select
                                size="sm"
                                items={GLYPH_OPTIONS}
                                value={prefs.glyphs}
                                onchange={(v) => {
                                    prefs.glyphs = v as GlyphMode;
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
</Screen>
