<script lang="ts">
  import { EmptyState, Notice, StateView } from '@sensei/ui';
    import { railIcon } from "../settings-nav";
    import { untrack } from "svelte";
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { PageHeader, Screen } from "$lib/components";
    import type { ExtensionRow } from "../settings-reads";
    import { ExtensionsState } from "./extensions-state.svelte.js";

    // The marketplace's own Installation section: `sensei init` installs the
    // recommended set, the catalog has the rest. Nothing in the app installs an
    // extension yet, so the empty state's next step is that page, not a button.
    const INSTALL_DOCS = "https://github.com/sensei-hq/marketplace#installation";

    let { data } = $props();

    // Presentation only: ExtensionsState holds the list, the toggles and Retry;
    // the read is +page.ts.
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
        box.checked = ex.extensions.find((e) => e.kind === ext.kind && e.name === ext.name)?.enabled === true;
    }
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="拡" icon={railIcon("/settings/extensions")} eyebrow="Settings" title="Extensions" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16" data-testid="settings-extensions">
        {#if ex.loadError}
            <StateView status="error" error={ex.loadError} onretry={() => void ex.load(api)} />
        {:else if ex.extensions.length === 0 && !ex.skipped?.length}
            <EmptyState
                size="panel"
                kanji="拡"
                title="No extensions installed yet."
                description="Skills, commands, agents, and hooks arrive with the sensei plugin: `sensei init` installs the recommended set, and the marketplace has the rest."
                action={{ label: "How to install extensions", href: INSTALL_DOCS, external: true, name: "install-docs" }}
            />
        {:else}
            <div
                class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
            >
                <h3 class="text-base m-0 mb-1">Extensions</h3>
                <p class="text-sm text-ink-mute m-0 mb-6">
                    Skills, commands, agents, and hooks installed in sensei.
                </p>
                {#if ex.extensions.length > 0}
                    <div class="flex flex-col gap-0.5">
                        {#each ex.extensions as ext (ext.kind + "::" + ext.name)}
                            {@const busy = ex.isBusy(ext)}
                            {@const toggleNotice = ex.toggleNotice(ext)}
                            <div
                                class="extension-row flex items-center gap-3 py-2.5 border-b border-paper-edge"
                                data-testid={`ext-row-${ext.kind}-${ext.name}`}
                            >
                                <span
                                    class="text-xs uppercase tracking-wide text-ink-mute w-[70px]"
                                    >{ext.kind}</span
                                >
                                <span class="text-sm text-ink flex-1">{ext.name}</span>
                                {#if busy}
                                    <span class="text-xs text-ink-mute w-14 text-right"
                                        >saving…</span
                                    >
                                {:else if toggleNotice}
                                    <Notice size="inline" bare expanded {...toggleNotice} data-testid={`ext-error-${ext.kind}-${ext.name}`} />
                                {/if}
                                <label
                                    class="inline-flex items-center gap-2 cursor-pointer"
                                >
                                    <input
                                        type="checkbox"
                                        class="cursor-pointer"
                                        data-testid={`ext-toggle-${ext.kind}-${ext.name}`}
                                        checked={ext.enabled === true}
                                        disabled={busy}
                                        onchange={(e) => toggleExtension(ext, e.currentTarget)}
                                    />
                                    <span
                                        class="extension-enabled text-xs w-14 {ext.enabled === true ? 'text-success' : 'text-ink-mute'}"
                                        >{ext.enabled === null ? "unknown" : ext.enabled ? "on" : "off"}</span
                                    >
                                </label>
                            </div>
                        {/each}
                    </div>
                {/if}
                {#if ex.skipped?.length}
                    <p
                        class="text-xs text-ink-mute m-0 {ex.extensions.length > 0 ? 'mt-3' : ''}"
                        data-testid="ext-skipped"
                    >
                        {ex.skipped.length} installed {ex.skipped.length === 1 ? "item" : "items"} not
                        listed — {ex.skipped
                            .map((s) => `${s.name || "an unnamed item"} (${s.reason})`)
                            .join("; ")}.
                    </p>
                {/if}
            </div>
        {/if}
    </div>
</Screen>

<style>
    .extension-row:last-child {
        border-bottom: none;
    }
</style>
