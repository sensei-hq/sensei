<script lang="ts">
    import { wizardState } from "$lib/wizard-state.svelte.js";
    import { Switch } from "$lib/components/index.js";

    const prefs = $derived(wizardState.preferences);
</script>

<div class="max-w-[760px] divide-y divide-paper-mute">
    <!-- ── Display name ─────────────────────────────────── -->
    <section class="pt-6">
        <header class="flex items-center gap-3.5 mb-3.5">
            <span
                class="kanji text-2xl text-accent leading-none w-8 text-center shrink-0"
                >名</span
            >
            <div class="flex-1 min-w-0">
                <h3 class="display text-lg font-normal m-0 text-ink">
                    What should sensei call you?
                </h3>
                <p class="text-xs text-ink-soft mt-1 leading-normal m-0">
                    Used in the morning digest and when raising a teaching.
                </p>
            </div>
            <input
                type="text"
                class="name-input w-[200px] px-3 py-2 text-sm border border-paper-mute rounded-md bg-paper-soft text-ink outline-none shrink-0 text-right ml-auto"
                value={prefs.displayName}
                oninput={(e) => {
                    wizardState.preferences.displayName = e.currentTarget.value;
                }}
                placeholder="your name"
            />
        </header>
    </section>

    <!-- ── Shared learnings ───────────────────────────────── -->
    <section class="pt-6 pb-1 border-t border-paper-mute">
        <header class="flex items-center gap-3.5 mb-3.5">
            <span
                class="kanji text-2xl text-accent leading-none w-8 text-center shrink-0"
                >共</span
            >
            <div class="flex-1 min-w-0">
                <h3 class="display text-lg font-normal m-0 text-ink">
                    Shared learnings
                </h3>
                <p class="text-xs text-ink-soft mt-1 leading-normal m-0">
                    Contribute anonymized patterns to a collective pool — and
                    pull what others have learned.
                </p>
            </div>
        </header>
        <div class="pl-11 divide-y divide-paper-mute">
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Contribute my learnings</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        Anonymized patterns only. No code, no commit messages,
                        no project names.
                    </div>
                </div>
                <Switch bind:value={wizardState.preferences.contributeLearnings} label="Toggle contribute learnings" />
            </div>
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Review before sharing</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        Each learning shows up in a queue for your approval
                        before it leaves your machine.
                    </div>
                </div>
                <Switch bind:value={wizardState.preferences.reviewBeforeShare} label="Toggle review before sharing" />
            </div>
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Sharing schedule</div>
                </div>
                <select
                    class="text-xs px-2.5 py-1.5 border border-paper-mute rounded-md bg-paper-soft text-ink cursor-pointer"
                    value={prefs.shareSchedule}
                    onchange={(e) => {
                        wizardState.preferences.shareSchedule = e.currentTarget.value;
                    }}
                >
                    <option value="off">Off · manual only</option>
                    <option value="daily">Daily</option>
                    <option value="weekly-saturday">Every Saturday</option>
                    <option value="monthly">Monthly</option>
                </select>
            </div>
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Download collective learnings</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        Reviewed before they enter your library.
                    </div>
                </div>
                <select
                    class="text-xs px-2.5 py-1.5 border border-paper-mute rounded-md bg-paper-soft text-ink cursor-pointer"
                    value={prefs.downloadCollective}
                    onchange={(e) => {
                        wizardState.preferences.downloadCollective = e.currentTarget.value;
                    }}
                >
                    <option value="never">Never</option>
                    <option value="weekly">Weekly</option>
                    <option value="daily">Daily</option>
                    <option value="on-demand">On demand</option>
                </select>
            </div>
        </div>
    </section>

    <!-- ── Sensei behavior ────────────────────────────────── -->
    <section class="pt-6 pb-1 border-t border-paper-mute">
        <header class="flex items-center gap-3.5 mb-3.5">
            <span
                class="kanji text-2xl text-accent leading-none w-8 text-center shrink-0"
                >師</span
            >
            <div class="flex-1 min-w-0">
                <h3 class="display text-lg font-normal m-0 text-ink">
                    Sensei behavior
                </h3>
                <p class="text-xs text-ink-soft mt-1 leading-normal m-0">
                    How forward sensei is — when it nudges, how it phrases
                    corrections.
                </p>
            </div>
        </header>
        <div class="pl-11 divide-y divide-paper-mute">
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Correction tone</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        How direct sensei is when something repeats.
                    </div>
                </div>
                <div class="inline-flex border border-paper-mute rounded-md overflow-hidden">
                    {#each [{ value: "gentle", label: "Gentle" }, { value: "balanced", label: "Balanced" }, { value: "direct", label: "Direct" }] as opt}
                        <button
                            class="segment-btn px-3 py-1.5 text-xs border-none border-l border-paper-mute bg-paper-soft text-ink-soft cursor-pointer"
                            class:active={prefs.correctionAggressiveness === opt.value}
                            onclick={() => {
                                wizardState.preferences.correctionAggressiveness = opt.value;
                            }}>{opt.label}</button
                        >
                    {/each}
                </div>
            </div>
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Morning digest</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        The Today view. Off keeps the dashboard quiet.
                    </div>
                </div>
                <div class="inline-flex border border-paper-mute rounded-md overflow-hidden">
                    {#each [{ value: "off", label: "Off" }, { value: "daily", label: "Daily" }, { value: "weekly", label: "Weekly" }] as opt}
                        <button
                            class="segment-btn px-3 py-1.5 text-xs border-none border-l border-paper-mute bg-paper-soft text-ink-soft cursor-pointer"
                            class:active={prefs.digestCadence === opt.value}
                            onclick={() => {
                                wizardState.preferences.digestCadence = opt.value;
                            }}>{opt.label}</button
                        >
                    {/each}
                </div>
            </div>
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Nudge on regression</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        If FTR drops sharply on a project, sensei surfaces it on
                        Today.
                    </div>
                </div>
                <Switch bind:value={wizardState.preferences.nudgeOnRegression} label="Toggle nudge on regression" />
            </div>
        </div>
    </section>

    <!-- ── Telemetry ──────────────────────────────────────── -->
    <section class="pt-6 pb-1 border-t border-paper-mute">
        <header class="flex items-center gap-3.5 mb-3.5">
            <span
                class="kanji text-2xl text-accent leading-none w-8 text-center shrink-0"
                >守</span
            >
            <div class="flex-1 min-w-0">
                <h3 class="display text-lg font-normal m-0 text-ink">
                    Telemetry
                </h3>
                <p class="text-xs text-ink-soft mt-1 leading-normal m-0">
                    Help us improve sensei itself — separate from shared
                    learnings, this is about the app, not your work.
                </p>
            </div>
        </header>
        <div class="pl-11 divide-y divide-paper-mute">
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Anonymized usage telemetry</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        Crashes, performance, which views you visit. Never code,
                        prompts, or session content.
                    </div>
                </div>
                <Switch bind:value={wizardState.preferences.anonymizedTelemetry} label="Toggle anonymized telemetry" />
            </div>
            <div class="row grid grid-cols-[1fr_auto] gap-8 items-center py-[11px]">
                <div>
                    <div class="text-sm text-ink">Show welcome message on first entry</div>
                    <div class="text-xs text-ink-soft mt-1 leading-snug max-w-[460px]">
                        The greeting toast that appears when you first open the
                        observatory each day.
                    </div>
                </div>
                <Switch bind:value={wizardState.preferences.showWelcome} label="Toggle welcome message" />
            </div>
        </div>
    </section>
</div>

<style>
    /* Segment button active state */
    .segment-btn:first-child {
        border-left: none;
    }
    .segment-btn.active {
        background: var(--paper-mute);
        color: var(--ink);
    }

    /* Name input focus */
    .name-input:focus {
        outline: none;
        border-color: var(--ink-mute);
    }
</style>
