<script lang="ts">
    import { railIcon } from "../settings-nav";
    import { invalidateAll } from "$app/navigation";
    import { PageHeader, Screen, ScreenState, StatusDot } from "$lib/components";
    import TranscriptConsentCard from "$lib/components/settings/TranscriptConsentCard.svelte";

    // The detected assistants, or why they could not be detected (#288) —
    // read by +page.ts.
    let { data } = $props();
    const assistants = $derived(data.assistants);
    const loadError = $derived(data.error);
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="連" icon={railIcon("/settings/assistants")} eyebrow="Settings" title="Assistants" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16" data-testid="settings-assistants">
        {#if loadError}
            <ScreenState status="error" error={loadError} onretry={() => void invalidateAll()} />
        {:else}
            <div
                class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
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
                                class="assistant-row flex items-center gap-3 py-3 border-b border-paper-edge"
                            >
                                <div class="flex-1 flex flex-col gap-0.5">
                                    <span class="text-sm text-ink">{asst.name}</span>
                                    <span class="text-xs text-ink-soft"
                                        >{asst.family}</span
                                    >
                                </div>
                                {#if asst.version}
                                    <span class="text-xs text-ink-soft font-mono"
                                        >{asst.version}</span
                                    >
                                {/if}
                                <StatusDot status={asst.configured ? "ok" : "idle"} />
                                <span class="text-xs text-ink-soft w-20"
                                    >{asst.configured ? "configured" : "detected"}</span
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
