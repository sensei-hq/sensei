<script lang="ts">
    /**
     * Consent to read each assistant's conversation history (#218).
     *
     * Its own card, apart from configuring the assistant, because they are two
     * different permissions: configuring writes hooks into the assistant's
     * settings; this copies its past conversations into sensei's database.
     * Used by Settings → Assistants and the setup wizard's assistants step.
     */
    import { onMount } from 'svelte';
    import Switch from '$lib/components/Switch.svelte';
    import { senseiApi } from '$lib/api.js';
    import { appState } from '$lib/appstate.svelte.js';
    import { TranscriptConsent } from './transcript-consent.svelte.js';

    const consent = new TranscriptConsent(senseiApi(appState.port));
    onMount(() => consent.load());
</script>

<div
    class="px-7 py-7 bg-paper-mute border border-paper-edge rounded-lg"
    data-testid="transcript-consent"
>
    <h3 class="text-base m-0 mb-1">Conversation history</h3>
    <p class="text-sm text-ink-soft m-0 mb-6 leading-normal">{consent.explanation}</p>

    {#if consent.failure}
        <p class="text-sm text-warning m-0 mb-4" data-testid="transcript-consent-error">
            {consent.failure}
        </p>
    {/if}

    <div class="flex flex-col gap-1">
        {#each consent.sources as s (s.source)}
            <div
                class="flex items-center gap-3 py-2 border-b border-paper-edge"
                data-testid="transcript-consent-row"
                data-source={s.source}
            >
                <span class="text-sm text-ink flex-1">{s.label}</span>
                <span class="text-xs text-ink-soft">
                    {s.consented ? 'reading history' : 'not read'}
                </span>
                <Switch
                    value={s.consented}
                    label="Read {s.label} conversation history"
                    disabled={consent.saving !== null}
                    onchange={() => consent.toggle(s.source)}
                />
            </div>
        {/each}
    </div>
</div>
