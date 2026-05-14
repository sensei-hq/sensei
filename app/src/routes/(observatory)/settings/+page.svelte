<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import TabBar from "$lib/components/TabBar.svelte";

    type Assistant = {
        family: string;
        name: string;
        version?: string;
        configured: boolean;
    };
    type ConfigEntry = { key: string; value: string };

    let assistants = $state<Assistant[]>([]);
    let config = $state<Record<string, string>>({});
    let extensions = $state<
        Array<{ name: string; kind: string; enabled: boolean }>
    >([]);
    let loading = $state(true);
    let section = $state("general");

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
        config = cfg;
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
</script>

<div class="max-w-[720px] mx-auto px-12 py-12 pb-16">
    <div class="mb-6">
        <p class="text-2xs tracking-loose uppercase text-surface-z6 m-0 mb-2">
            Settings
        </p>
        <h1 class="display text-2xl font-normal m-0">設 Settings</h1>
    </div>

    <TabBar tabs={sectionTabs} bind:active={section} class="mb-8" />

    {#if loading}
        <p class="text-ui text-surface-z6 leading-relaxed">
            Loading settings...
        </p>
    {:else if section === "general"}
        <div
            class="px-7 py-7 bg-surface-z2 border border-surface-z3 rounded-lg"
        >
            <h3 class="text-base m-0 mb-1">Preferences</h3>
            <div class="flex flex-col gap-0.5">
                {#each Object.entries(config) as [key, value]}
                    <div
                        class="setting-row flex justify-between py-2.5 border-b border-surface-z2"
                    >
                        <span class="text-ui text-surface-z9 mono">{key}</span>
                        <span class="text-ui text-surface-z7">{value}</span>
                    </div>
                {/each}
                {#if Object.keys(config).length === 0}
                    <p class="text-ui text-surface-z6 leading-relaxed">
                        No configuration values set. Preferences from the setup
                        wizard will appear here.
                    </p>
                {/if}
            </div>
        </div>
    {:else if section === "assistants"}
        <div
            class="px-7 py-7 bg-surface-z2 border border-surface-z3 rounded-lg"
        >
            <h3 class="text-base m-0 mb-1">Assistants</h3>
            <p class="text-ui text-surface-z6 m-0 mb-6">
                AI coding tools detected on this machine.
            </p>
            {#if assistants.length === 0}
                <p class="text-ui text-surface-z6 leading-relaxed">
                    No assistants detected. Run the setup wizard to configure
                    assistants.
                </p>
            {:else}
                <div class="flex flex-col gap-1">
                    {#each assistants as asst}
                        <div
                            class="assistant-row flex items-center gap-3 py-3 border-b border-surface-z2"
                        >
                            <div class="flex-1 flex flex-col gap-0.5">
                                <span class="text-ui text-surface-z9"
                                    >{asst.name}</span
                                >
                                <span class="text-2xs text-surface-z6"
                                    >{asst.family}</span
                                >
                            </div>
                            {#if asst.version}
                                <span class="text-xs text-surface-z6 mono"
                                    >{asst.version}</span
                                >
                            {/if}
                            <span
                                class="status-dot w-1.75 h-1.75 rounded-full"
                                class:configured={asst.configured}
                                class:unconfigured={!asst.configured}
                            ></span>
                            <span class="text-2xs text-surface-z6 w-20"
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
        <div
            class="px-7 py-7 bg-surface-z2 border border-surface-z3 rounded-lg"
        >
            <h3 class="text-base m-0 mb-1">Inference</h3>
            <p class="text-ui text-surface-z6 m-0 mb-6">
                Local and external model configuration.
            </p>
            <div class="flex flex-col items-center gap-3 py-10">
                <span class="kanji text-3xl text-primary-z5 opacity-40">想</span
                >
                <p class="text-ui text-surface-z6 leading-relaxed">
                    Inference configuration will be available once model
                    assignments are supported.
                </p>
            </div>
        </div>
    {:else if section === "extensions"}
        <div
            class="px-7 py-7 bg-surface-z2 border border-surface-z3 rounded-lg"
        >
            <h3 class="text-base m-0 mb-1">Extensions</h3>
            <p class="text-ui text-surface-z6 m-0 mb-6">
                Skills, commands, agents, and hooks installed in sensei.
            </p>
            {#if extensions.length === 0}
                <p class="text-ui text-surface-z6 leading-relaxed">
                    No extensions installed yet.
                </p>
            {:else}
                <div class="flex flex-col gap-0.5">
                    {#each extensions as ext}
                        <div
                            class="extension-row flex items-center gap-3 py-2.5 border-b border-surface-z2"
                        >
                            <span
                                class="text-3xs uppercase tracking-widest text-surface-z5 w-[70px]"
                                >{ext.kind}</span
                            >
                            <span class="text-ui text-surface-z9 flex-1"
                                >{ext.name}</span
                            >
                            <span
                                class="extension-enabled text-2xs text-surface-z6"
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
    .setting-row:last-child {
        border-bottom: none;
    }
    .assistant-row:last-child {
        border-bottom: none;
    }
    .extension-row:last-child {
        border-bottom: none;
    }

    .status-dot.configured {
        background: oklch(var(--color-success-z5) / 1);
    }
    .status-dot.unconfigured {
        background: oklch(var(--color-warning-z5) / 1);
    }

    .extension-enabled.on {
        color: oklch(var(--color-success-z5) / 1);
    }
</style>
