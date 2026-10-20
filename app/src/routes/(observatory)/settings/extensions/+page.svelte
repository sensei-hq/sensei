<script lang="ts">
    import { railIcon } from "../settings-nav";
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { PageHeader, Screen, ScreenState } from "$lib/components";
    import { readExtensions } from "../settings-reads";

    type Extension = { name: string; kind: string; enabled: boolean };

    let extensions = $state<Extension[]>([]);
    // Per-row busy so a slow toggle on one extension does not disable its
    // neighbours. Keyed by `${kind}::${name}`.
    let extBusy = $state<Record<string, boolean>>({});
    let loading = $state(true);

    // Why the installed extensions could not be read (#288) — not "none installed".
    let loadError = $state<string | null>(null);

    async function load() {
        loading = true;
        const read = await readExtensions(senseiApi(appState.port));
        extensions = read.extensions;
        loadError = read.error;
        loading = false;
    }
    onMount(load);

    async function toggleExtension(ext: Extension, next: boolean): Promise<void> {
        const key = `${ext.kind}::${ext.name}`;
        extBusy[key] = true;
        try {
            const api = senseiApi(appState.port);
            const result = await api.setInstalledItemEnabled(
                ext.name,
                ext.kind,
                next,
            );
            if (!result.ok) {
                console.warn(
                    "[settings] toggleExtension failed",
                    ext.kind,
                    ext.name,
                    result.error,
                );
                // Re-read so the checkbox reflects daemon truth after a failure —
                // or, if that read fails too, the page says so.
                await load();
                return;
            }
            // Optimistic update — daemon confirmed the move.
            ext.enabled = next;
            extensions = extensions.map((e) =>
                e.kind === ext.kind && e.name === ext.name
                    ? { ...e, enabled: next }
                    : e,
            );
        } finally {
            extBusy[key] = false;
        }
    }
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="拡" icon={railIcon("/settings/extensions")} eyebrow="Settings" title="Extensions" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16" data-testid="settings-extensions">
        {#if loading}
            <p class="text-sm text-ink-soft leading-normal">Loading…</p>
        {:else if loadError}
            <ScreenState status="error" error={loadError} onretry={() => void load()} />
        {:else}
            <div
                class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
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
                        {#each extensions as ext (ext.kind + "::" + ext.name)}
                            {@const busy = extBusy[ext.kind + "::" + ext.name]}
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
                                        onchange={(e) =>
                                            toggleExtension(
                                                ext,
                                                e.currentTarget.checked,
                                            )}
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
