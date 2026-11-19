<script lang="ts">
    import { Notice, StateView } from '@sensei/ui';
    import { Button } from '@rokkit/ui';
    import { appState } from "$lib/appstate.svelte.js";
    import { senseiApi } from "$lib/api.js";
    import { detectFailedNotice } from "$lib/setup/loaders.js";
    import { detectAgain, detectedLabel } from "../settings-reads";
    import { railIcon } from "../settings-nav";
    import { invalidateAll } from "$app/navigation";
    import { PageHeader, Screen, StatusDot } from "$lib/components";
    import TranscriptConsentCard from "$lib/components/settings/TranscriptConsentCard.svelte";

    // This machine's installs as detection stored them, or why they could not
    // be read (#288, #455) — read by +page.ts.
    let { data } = $props();
    const assistants = $derived(data.assistants);
    const loadError = $derived(data.error);

    // "Detect again" (#455): probe and store, then re-read; a refusal says why.
    let detecting = $state(false);
    let detectError = $state<string | null>(null);
    const detectNotice = $derived(detectFailedNotice(detectError));
    async function redetect() {
        detecting = true;
        detectError = await detectAgain(senseiApi(appState.port));
        detecting = false;
        if (!detectError) await invalidateAll();
    }
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="連" icon={railIcon("/settings/assistants")} eyebrow="Settings" title="Assistants" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16" data-testid="settings-assistants">
        {#if loadError}
            <StateView status="error" error={loadError} onretry={() => void invalidateAll()} />
        {:else}
            <div
                class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
            >
                <div class="flex items-start gap-3 mb-6">
                    <div class="flex-1">
                        <h3 class="text-base m-0 mb-1">Assistants</h3>
                        <p class="text-sm text-ink-mute m-0">
                            AI coding tools detected on this machine.
                        </p>
                    </div>
                    <Button style="outline" size="sm" label="Detect again" loading={detecting} onclick={redetect} data-action="detect-again" />
                </div>
{#if detectNotice}
    <div class="mb-4"><Notice size="inline" expanded {...detectNotice} data-testid="detect-error" /></div>
  {/if}
                {#if assistants.length === 0}
                    <p class="text-sm text-ink-mute leading-normal">
                        No assistants detected. Run the setup wizard to configure
                        assistants.
                    </p>
                {:else}
                    <div class="flex flex-col gap-1">
                        {#each assistants as asst}
                            <div
                                class="assistant-row flex items-center gap-3 py-3 border-b border-paper-edge"
                                data-assistant={asst.family}
                            >
                                <div class="flex-1 flex flex-col gap-0.5">
                                    <span class="text-sm text-ink">{asst.name}</span>
                                    <span class="text-xs text-ink-mute" data-detected-at={asst.detectedAt}
                                        >{asst.kind} · {detectedLabel(asst.detectedAt)}</span
                                    >
                                </div>
                                {#if asst.version}
                                    <span class="text-xs text-ink-mute font-mono"
                                        >{asst.version}</span
                                    >
                                {/if}
                                <StatusDot status={asst.configured ? "ok" : "idle"} />
                                <span class="text-xs text-ink-mute w-20"
                                    >{asst.configured ? "configured" : asst.installed ? "installed" : "not installed"}</span
                                >
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>
        {/if}
        <div class="mt-6">
            <TranscriptConsentCard />
        </div>
    </div>
</Screen>
<style>
    .assistant-row:last-child {
        border-bottom: none;
    }
</style>
