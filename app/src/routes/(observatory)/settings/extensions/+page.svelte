<script lang="ts">
    import { onMount } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { PageHeader } from "$lib/components";

    type Extension = { name: string; kind: string; enabled: boolean };

    let extensions = $state<Extension[]>([]);
    // Per-row busy so a slow toggle on one extension does not disable its
    // neighbours. Keyed by `${kind}::${name}`.
    let extBusy = $state<Record<string, boolean>>({});
    let loading = $state(true);

    onMount(async () => {
        const raw = await senseiApi(appState.port).getInstalledItems();
        extensions = (raw as any[]).map((i) => ({
            name: i.name,
            kind: i.kind ?? "unknown",
            enabled: i.enabled ?? true,
        }));
        loading = false;
    });

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
                // Refetch so the checkbox reflects daemon truth after a failure.
                const fresh = await api.getInstalledItems();
                extensions = (fresh as any[]).map((i) => ({
                    name: i.name,
                    kind: i.kind ?? "unknown",
                    enabled: i.enabled ?? true,
                }));
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

<PageHeader kanji="拡" eyebrow="Settings" title="Extensions" />
<div class="max-w-[720px] mx-auto px-12 pt-8 pb-16" data-testid="settings-extensions">
    {#if loading}
        <p class="text-sm text-ink-soft leading-normal">Loading…</p>
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

<style>
    .extension-row:last-child {
        border-bottom: none;
    }
    .extension-enabled.on {
        color: var(--success);
    }
</style>
