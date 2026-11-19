<script lang="ts">
  import { onMount } from 'svelte';
  import AssistantCard from '$lib/components/AssistantCard.svelte';
  import TranscriptConsentCard from '$lib/components/settings/TranscriptConsentCard.svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import { detectFailedNotice, storeAssistantInstalls } from '$lib/setup/loaders.js';
  import { Notice } from '@sensei/ui';

  // Every card is the wizard state's view of its family; the live part events
  // are the state's to follow too (audit 2026-10-28, item 12).
  const families = $derived(wizardState.assistants.assistants);

  onMount(() => wizardState.watchAssistantEvents(appState.port));

  // First run (#455): store this machine's installs on entry, so Settings and the
  // relay picker have rows to read. A refusal is shown, never dropped.
  let detectError = $state<string | null>(null);
  const detectNotice = $derived(detectFailedNotice(detectError));
  onMount(() => {
    void storeAssistantInstalls(appState.port).then((why) => (detectError = why));
  });

  async function handleToggle(familyId: string) {
    try { await wizardState.toggleAssistant(familyId); }
    catch { /* error already surfaced on the card via partErrors */ }
  }

  async function handleRetry(familyId: string) {
    await wizardState.retryAssistant(familyId);
  }
</script>

<div>
  <p class="text-sm text-ink-mute leading-normal m-0 mb-6">
    One switch per assistant — sensei registers every capability it can,
    or none. Flip a switch and watch the chips settle.
  </p>

{#if detectNotice}
    <div class="mb-4"><Notice size="inline" expanded {...detectNotice} data-testid="detect-error" /></div>
  {/if}

  <div class="grid grid-cols-2 gap-3">
    {#each families as fam (fam.id)}
      <AssistantCard
        view={wizardState.assistantView(fam)}
        onToggle={() => handleToggle(fam.id)}
        onRetry={() => handleRetry(fam.id)}
      />
    {/each}
  </div>

  {#if families.length === 0}
    <p class="text-sm text-ink-mute italic">
      No AI coding assistants detected. Make sure the daemon is running.
    </p>
  {/if}

  <!-- A separate yes (#218): the switches above configure hooks; reading past
       conversations is asked here, per assistant, and starts off. -->
  <div class="mt-6">
    <TranscriptConsentCard />
  </div>
</div>
