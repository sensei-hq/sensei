<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import TabBar from "$lib/components/TabBar.svelte";
    import { PageHeader, StatusDot, Switch } from "$lib/components";
    import type { PreferencesData } from "$lib/setup/contracts.js";
    import { DEFAULT_PREFERENCES, fromPreferencesForm, toPreferencesForm } from "./preferences-form.js";
    import InferenceAssignmentsPanel from "./InferenceAssignmentsPanel.svelte";

    type Assistant = {
        family: string;
        name: string;
        version?: string;
        configured: boolean;
    };

    let assistants = $state<Assistant[]>([]);
    // The editable preferences form — hydrated from `config` on mount,
    // saved back through `setConfig` on every change so the daemon is
    // canonical and the wizard sees the same values.
    let prefs = $state<PreferencesData>({ ...DEFAULT_PREFERENCES });
    let extensions = $state<
        Array<{ name: string; kind: string; enabled: boolean }>
    >([]);
    let loading = $state(true);
    let section = $state("general");
    // Short banner state: 'idle' / 'saving' / 'saved' / 'error'. Auto-clears
    // 1.5s after a success so the reader gets one glance of confirmation
    // without a permanent noise line.
    let saveStatus = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
    let saveError = $state<string | null>(null);
    let saveTimer: ReturnType<typeof setTimeout> | null = null;

    const sectionTabs: [string, string][] = [
        ["general", "General"],
        ["assistants", "Assistants"],
        ["inference", "Inference"],
        ["extensions", "Extensions"],
    ];

    onMount(async () => {
        const api = senseiApi(appState.port);
        const [cfg, assts, items] = await Promise.all([
            api.getConfig(),
            api.detectAssistants(),
            api.getInstalledItems(),
        ]);
        prefs = toPreferencesForm(cfg);
        assistants = (assts as any[]).map((a) => ({
            family: a.family ?? a.name,
            name: a.name ?? a.family,
            version: a.version,
            configured: a.configured ?? a.found ?? false,
        }));
        extensions = (items as any[]).map((i) => ({
            name: i.name,
            kind: i.kind ?? "unknown",
            enabled: i.enabled ?? true,
        }));
        loading = false;
    });

    // Boolean toggles auto-persist. Text/select fields still call `persist()`
    // explicitly from their handlers (an `oninput`-driven effect would fire
    // on every keystroke). Gated on `loading` so the initial hydrate doesn't
    // spuriously re-save the same values back.
    $effect(() => {
        void prefs.nudgeOnRegression;
        void prefs.anonymizedTelemetry;
        void prefs.showWelcome;
        if (!loading) void persist();
    });

    async function persist() {
        if (saveTimer) { clearTimeout(saveTimer); saveTimer = null; }
        saveStatus = 'saving';
        saveError = null;
        try {
            const result = await senseiApi(appState.port).trySetConfig(fromPreferencesForm(prefs));
            if (result.ok) {
                saveStatus = 'saved';
                saveTimer = setTimeout(() => {
                    if (saveStatus === 'saved') saveStatus = 'idle';
                }, 1500);
            } else {
                saveStatus = 'error';
                saveError = result.error.message;
            }
        } catch (e) {
            saveStatus = 'error';
            saveError = (e as Error).message;
        }
    }
</script>

<PageHeader kanji="設" eyebrow="Settings" title="Settings" />
<div class="max-w-[720px] mx-auto px-12 pt-8 pb-16">

    <TabBar tabs={sectionTabs} bind:active={section} class="mb-8" />

    {#if loading}
        <p class="text-sm text-ink-soft leading-normal">
            Loading settings...
        </p>
    {:else if section === "general"}
        <div
            class="px-7 py-7 bg-paper-mute border border-paper-mute rounded-lg"
            data-testid="settings-general"
        >
            <div class="flex items-baseline justify-between mb-4">
                <h3 class="text-base m-0">Preferences</h3>
                <span
                    class="text-xs"
                    class:text-ink-mute={saveStatus === 'idle'}
                    class:text-ink-soft={saveStatus === 'saving'}
                    class:text-success={saveStatus === 'saved'}
                    class:text-warning={saveStatus === 'error'}
                    data-testid="settings-save-status"
                >
                    {#if saveStatus === 'saving'}saving…
                    {:else if saveStatus === 'saved'}saved
                    {:else if saveStatus === 'error'}{saveError ?? 'save failed'}
                    {:else}auto-saves as you edit
                    {/if}
                </span>
            </div>

            <div class="flex flex-col divide-y divide-paper-edge">
                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Display name</div>
                        <div class="text-xs text-ink-mute mt-0.5">Used in the morning digest and when raising a teaching.</div>
                    </div>
                    <input
                        type="text"
                        class="w-[220px] px-3 py-2 text-sm border border-paper-mute rounded-md bg-paper-soft text-ink outline-none text-right"
                        data-testid="pref-display-name"
                        value={prefs.displayName}
                        oninput={(e) => { prefs.displayName = e.currentTarget.value; void persist(); }}
                        placeholder="your name"
                    />
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Morning digest</div>
                        <div class="text-xs text-ink-mute mt-0.5">The Today view cadence. Off keeps the dashboard quiet.</div>
                    </div>
                    <select
                        class="text-xs px-2.5 py-1.5 border border-paper-mute rounded-md bg-paper-soft text-ink cursor-pointer"
                        data-testid="pref-digest-cadence"
                        value={prefs.digestCadence}
                        onchange={(e) => { prefs.digestCadence = e.currentTarget.value; void persist(); }}
                    >
                        <option value="off">Off</option>
                        <option value="daily">Daily</option>
                        <option value="weekly">Weekly</option>
                    </select>
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Correction tone</div>
                        <div class="text-xs text-ink-mute mt-0.5">How direct sensei is when something repeats.</div>
                    </div>
                    <select
                        class="text-xs px-2.5 py-1.5 border border-paper-mute rounded-md bg-paper-soft text-ink cursor-pointer"
                        data-testid="pref-correction-aggressiveness"
                        value={prefs.correctionAggressiveness}
                        onchange={(e) => { prefs.correctionAggressiveness = e.currentTarget.value; void persist(); }}
                    >
                        <option value="gentle">Gentle</option>
                        <option value="balanced">Balanced</option>
                        <option value="direct">Direct</option>
                    </select>
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Nudge on regression</div>
                        <div class="text-xs text-ink-mute mt-0.5">If FTR drops sharply on a project, sensei surfaces it on Today.</div>
                    </div>
                    <Switch bind:value={prefs.nudgeOnRegression} label="Toggle nudge on regression" />
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Anonymized telemetry</div>
                        <div class="text-xs text-ink-mute mt-0.5">Crashes, performance, which views you visit. Never code or session content.</div>
                    </div>
                    <Switch bind:value={prefs.anonymizedTelemetry} label="Toggle anonymized telemetry" />
                </div>

                <div class="row grid grid-cols-[1fr_auto] gap-6 items-center py-3">
                    <div>
                        <div class="text-sm text-ink">Show welcome greeting</div>
                        <div class="text-xs text-ink-mute mt-0.5">The daily greeting toast on the observatory.</div>
                    </div>
                    <Switch bind:value={prefs.showWelcome} label="Toggle welcome greeting" />
                </div>
            </div>
        </div>
    {:else if section === "assistants"}
        <div
            class="px-7 py-7 bg-paper-mute border border-paper-mute rounded-lg"
        >
            <h3 class="text-base m-0 mb-1">Assistants</h3>
            <p class="text-sm text-ink-soft m-0 mb-6">
                AI coding tools detected on this machine.
            </p>
            {#if assistants.length === 0}
                <p class="text-sm text-ink-soft leading-normal">
                    No assistants detected. Run the setup wizard to configure
                    assistants.
                </p>
            {:else}
                <div class="flex flex-col gap-1">
                    {#each assistants as asst}
                        <div
                            class="assistant-row flex items-center gap-3 py-3 border-b border-paper-mute"
                        >
                            <div class="flex-1 flex flex-col gap-0.5">
                                <span class="text-sm text-ink"
                                    >{asst.name}</span
                                >
                                <span class="text-xs text-ink-soft"
                                    >{asst.family}</span
                                >
                            </div>
                            {#if asst.version}
                                <span class="text-xs text-ink-soft font-mono"
                                    >{asst.version}</span
                                >
                            {/if}
                            <StatusDot status={asst.configured ? 'ok' : 'idle'} />
                            <span class="text-xs text-ink-soft w-20"
                                >{asst.configured
                                    ? "configured"
                                    : "detected"}</span
                            >
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    {:else if section === "inference"}
        <InferenceAssignmentsPanel />
    {:else if section === "extensions"}
        <div
            class="px-7 py-7 bg-paper-mute border border-paper-mute rounded-lg"
        >
            <h3 class="text-base m-0 mb-1">Extensions</h3>
            <p class="text-sm text-ink-soft m-0 mb-6">
                Skills, commands, agents, and hooks installed in sensei.
            </p>
            {#if extensions.length === 0}
                <p class="text-sm text-ink-soft leading-normal">
                    No extensions installed yet.
                </p>
            {:else}
                <div class="flex flex-col gap-0.5">
                    {#each extensions as ext}
                        <div
                            class="extension-row flex items-center gap-3 py-2.5 border-b border-paper-mute"
                        >
                            <span
                                class="text-xs uppercase tracking-wide text-ink-soft w-[70px]"
                                >{ext.kind}</span
                            >
                            <span class="text-sm text-ink flex-1"
                                >{ext.name}</span
                            >
                            <span
                                class="extension-enabled text-xs text-ink-soft"
                                class:on={ext.enabled}
                                >{ext.enabled ? "on" : "off"}</span
                            >
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .assistant-row:last-child {
        border-bottom: none;
    }
    .extension-row:last-child {
        border-bottom: none;
    }

    .extension-enabled.on {
        color: var(--success);
    }
</style>
