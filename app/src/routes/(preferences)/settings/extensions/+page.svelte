<script lang="ts">
    import { railIcon } from "../settings-nav";
    import { untrack } from "svelte";
    import { invalidateAll } from "$app/navigation";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { PageHeader, Screen, ScreenState } from "$lib/components";
    import type { ExtensionRow } from "../settings-reads";
    import { ExtensionsState } from "./extensions-state.svelte.js";

    let { data } = $props();

    // Presentation only: ExtensionsState holds the list and the toggles; the
    // read is +page.ts.
    const ex = new ExtensionsState(untrack(() => data));
    $effect(() => {
        const read = data;
        untrack(() => ex.setRead(read));
    });
    const api = $derived(senseiApi(appState.port));

    // The box shows the state's answer once the daemon has replied — a refused
    // toggle puts it back.
    async function toggleExtension(ext: ExtensionRow, box: HTMLInputElement) {
        await ex.toggle(api, ext, box.checked);
        box.checked = ex.extensions.find((e) => e.kind === ext.kind && e.name === ext.name)?.enabled ?? box.checked;
    }
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="拡" icon={railIcon("/settings/extensions")} eyebrow="Settings" title="Extensions" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16" data-testid="settings-extensions">
        {#if ex.loadError}
            <ScreenState status="error" error={ex.loadError} onretry={() => void invalidateAll()} />
        {:else}
            <div
                class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
            >
                <h3 class="text-base m-0 mb-1">Extensions</h3>
                <p class="text-sm text-ink-soft m-0 mb-6">
                    Skills, commands, agents, and hooks installed in sensei.
                </p>
                {#if ex.extensions.length === 0}
                    <p class="text-sm text-ink-soft leading-normal">
                        No extensions installed yet.
                    </p>
                {:else}
                    <div class="flex flex-col gap-0.5">
                        {#each ex.extensions as ext (ext.kind + "::" + ext.name)}
                            {@const busy = ex.isBusy(ext)}
                            {@const toggleError = ex.toggleError(ext)}
                            <div
                                class="extension-row flex items-center gap-3 py-2.5 border-b border-paper-edge"
                                data-testid={`ext-row-${ext.kind}-${ext.name}`}
                            >
                                <span
                                    class="text-xs uppercase tracking-wide text-ink-soft w-[70px]"
                                    >{ext.kind}</span
                                >
                                <span class="text-sm text-ink flex-1">{ext.name}</span>
                                {#if busy}
                                    <span class="text-xs text-ink-soft w-14 text-right"
                                        >saving…</span
                                    >
                                {:else if toggleError}
                                    <span class="text-xs text-danger" data-testid={`ext-error-${ext.kind}-${ext.name}`}>{toggleError}</span>
                                {/if}
                                <label
                                    class="inline-flex items-center gap-2 cursor-pointer"
                                >
                                    <input
                                        type="checkbox"
                                        class="cursor-pointer"
                                        data-testid={`ext-toggle-${ext.kind}-${ext.name}`}
                                        checked={ext.enabled}
                                        disabled={busy}
                                        onchange={(e) => toggleExtension(ext, e.currentTarget)}
                                    />
                                    <span
                                        class="extension-enabled text-xs text-ink-soft w-8"
                                        class:on={ext.enabled}
                                        >{ext.enabled ? "on" : "off"}</span
                                    >
                                </label>
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</Screen>

<style>
    .extension-row:last-child {
        border-bottom: none;
    }
    .extension-enabled.on {
        color: var(--success);
    }
</style>
