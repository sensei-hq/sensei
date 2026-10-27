<script lang="ts">
    import { railIcon } from "../settings-nav";
    import { onMount, untrack } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { PageHeader, Screen, ScreenState, Switch } from "$lib/components";
    import { Toggle } from "@rokkit/ui";
    import { invalidateAll } from "$app/navigation";
    import { PreferencesState, type SaveConfig } from "./general-state.svelte.js";
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

    let { data } = $props();

    // Presentation only: PreferencesState holds the form and its saves; the
    // read is +page.ts. A re-read (Retry) replaces the form.
    const st = new PreferencesState(untrack(() => data));
    $effect(() => {
        const read = data;
        untrack(() => st.setRead(read));
    });
    const save: SaveConfig = (config) => appState.setConfigs(config);
    const persist = () => st.persist(save);

    onMount(() => void st.savePrefilled(save));

    // The toggles save as they change; text and selects save from their handlers.
    // Untracked: saving reads and writes appState.config, and tracking that made
    // every save re-run this effect — an endless write loop.
    $effect(() => {
        void st.prefs.nudgeOnRegression;
        void st.prefs.anonymizedTelemetry;
        void st.prefs.showWelcome;
        untrack(() => void st.saveIfTogglesChanged(save));
    });
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="名" icon={railIcon("/settings/general")} eyebrow="Settings" title="General" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16">
        {#if st.loadError}
            <ScreenState status="error" error={st.loadError} onretry={() => void invalidateAll()} />
        {:else}
            <div
                class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
                data-testid="settings-general"
            >
                <div class="flex items-baseline justify-between mb-4">
                    <h3 class="text-base m-0">Preferences</h3>
                    <span
                        class="text-xs"
                        class:text-ink-mute={st.saveStatus === "idle"}
                        class:text-ink-soft={st.saveStatus === "saving"}
                        class:text-success={st.saveStatus === "saved"}
                        class:text-warning={st.saveStatus === "error"}
                        data-testid="settings-save-status"
                    >
                        {#if st.saveStatus === "saving"}saving…
                        {:else if st.saveStatus === "saved"}saved
                        {:else if st.saveStatus === "error"}{st.saveError ?? "save failed"}
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
                            value={st.prefs.displayName}
                            oninput={(e) => {
                                st.prefs.displayName = e.currentTarget.value;
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
                        <!-- testid on wrapper: rokkit Toggle doesn't forward data-* to the DOM. -->
                        <div data-testid="pref-digest-cadence">
                            <Toggle
                                size="sm"
                                options={DIGEST_OPTIONS}
                                label="Morning digest"
                                value={st.prefs.digestCadence}
                                onchange={(v) => {
                                    st.prefs.digestCadence = v as string;
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
                            <Toggle
                                size="sm"
                                options={TONE_OPTIONS}
                                label="Correction tone"
                                value={st.prefs.correctionAggressiveness}
                                onchange={(v) => {
                                    st.prefs.correctionAggressiveness = v as string;
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
                            <Toggle
                                size="sm"
                                options={GLYPH_OPTIONS}
                                label="Glyphs"
                                value={st.prefs.glyphs}
                                onchange={(v) => {
                                    st.prefs.glyphs = v as GlyphMode;
                                    void persist();
                                }}
                            />
                        </div>
                    </div>

                    <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                        <div>
                            <div class="text-sm text-ink">Nudge on regression</div>
                            <div class="text-xs text-ink-mute mt-0.5">
                                If FTR drops sharply on a workspace, sensei surfaces it on Today.
                            </div>
                        </div>
                        <Switch
                            bind:value={st.prefs.nudgeOnRegression}
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
                            bind:value={st.prefs.anonymizedTelemetry}
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
                            bind:value={st.prefs.showWelcome}
                            label="Toggle welcome greeting"
                        />
                    </div>
                </div>
            </div>
        {/if}
    </div>
</Screen>
