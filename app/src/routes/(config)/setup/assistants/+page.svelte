<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import AssistantCard from '$lib/components/AssistantCard.svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { EventManager } from '$lib/events.js';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import type {
    AssistantPartEvent, AssistantPartStatus,
  } from '$lib/types.js';
  import type { DaemonAssistantFamily } from '$lib/setup/contracts.js';

  /** StateEvent shape on the wire. Matches the daemon's serde output:
   *  { action, entity, data }. We only care about entity="assistant"
   *  events here; everything else (scan/folder/activity) is filtered
   *  out at dispatch time. */
  interface WireEvent {
    action: string;
    entity: string;
    data: AssistantPartEvent;
  }

  const families = $derived(wizardState.assistants.assistants);
  const partStatus = $derived(wizardState.assistants.partStatus);
  const partErrors = $derived(wizardState.assistants.partErrors);

  /** True when any installed variant of a family is currently set up
   *  (mirrors the daemon's per-variant `configured` field). */
  function anyInstalled(family: DaemonAssistantFamily): boolean {
    return family.variants.some(v => v.installed);
  }

  /** Derive the AssistantCard's `parts` prop from the family + the live
   *  status map. Missing entries default to 'idle' — happens for parts
   *  that have never received an event since the slice hydrated. */
  function partsFor(family: DaemonAssistantFamily) {
    const status = partStatus[family.id] ?? {};
    return family.parts.map(p => ({
      id: p.id,
      label: p.label,
      status: (status[p.id] ?? 'idle') as AssistantPartStatus,
    }));
  }

  /** Consolidated error message — concat unique messages from any parts
   *  in error state. Mostly a single string since markFamilyError sets
   *  the same message across every part, but SSE-driven flows could
   *  legitimately differ per part if the daemon ever reports that way. */
  function errorFor(family: DaemonAssistantFamily): string | null {
    const errs = partErrors[family.id];
    if (!errs) return null;
    const messages = Object.values(errs).filter(Boolean);
    if (messages.length === 0) return null;
    return Array.from(new Set(messages)).join('; ');
  }

  // ── SSE subscription ────────────────────────────────────────────────
  // The daemon broadcasts AssistantPartEvent over the same /api/scan/events
  // channel used for scan progress, with entity="assistant". We open our
  // own EventManager here (separate from scan-state's subscription) so the
  // wizard's Assistants page can receive live chip transitions without
  // depending on whether the Scan stage has been visited yet.
  let events: EventManager<WireEvent> | null = $state(null);
  let unsubscribe: (() => void) | null = null;

  onMount(() => {
    events = new EventManager<WireEvent>(
      `http://127.0.0.1:${appState.port}/api/scan/events`,
      (data) => JSON.parse(data) as WireEvent,
    );
    unsubscribe = events.subscribe((evt) => {
      if (evt.entity !== 'assistant') return;
      wizardState.applyAssistantEvent(evt.data);
    });
  });

  onDestroy(() => {
    unsubscribe?.();
    events?.destroy();
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
  <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
    One switch per assistant — sensei registers every capability it can,
    or none. Flip a switch and watch the chips settle.
  </p>

  <div class="flex flex-col gap-3">
    {#each families as fam (fam.id)}
      <AssistantCard
        id={fam.id}
        name={fam.name}
        found={anyInstalled(fam)}
        enabled={fam.selected}
        parts={partsFor(fam)}
        error={errorFor(fam)}
        onToggle={() => handleToggle(fam.id)}
        onRetry={() => handleRetry(fam.id)}
      />
    {/each}
  </div>

  {#if families.length === 0}
    <p class="text-sm text-surface-z6 italic">
      No AI coding assistants detected. Make sure the daemon is running.
    </p>
  {/if}
</div>
