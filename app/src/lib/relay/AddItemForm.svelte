<script lang="ts">
  // Add an open item to triage (CONTRACT §1d: "open items the person adds"). A title and
  // its kind; it calls back, clears once added, and says why a refusal happened, keeping
  // what was typed. Not drawn in the r1 mockup: the smallest form the contract needs.
  import { Notice } from '@sensei/ui';
  import type { RelayItemKind } from '@sensei/state';

  let { onsubmit }: { onsubmit: (draft: { title: string; kind: RelayItemKind }) => Promise<string | null> } = $props();

  const KINDS: Array<{ value: RelayItemKind; label: string }> = [
    { value: 'issue', label: 'Issue' },
    { value: 'bug', label: 'Bug' },
    { value: 'feature', label: 'Feature' },
    { value: 'gap', label: 'Gap' },
    { value: 'finding', label: 'Review finding' },
  ];

  let title = $state('');
  let kind = $state<RelayItemKind>('issue');
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function submit(event?: Event): Promise<void> {
    event?.preventDefault();
    const text = title.trim();
    if (busy || !text) return;
    busy = true;
    error = null;
    const refused = await onsubmit({ title: text, kind });
    if (refused) error = refused;
    else title = '';
    busy = false;
  }
</script>

<form data-add-item onsubmit={submit} class="flex flex-wrap items-center gap-2">
  <input
    data-add-item-title
    bind:value={title}
    placeholder="Add an open item"
    aria-label="An open item to add to triage"
    class="flex-[1_1_240px] min-w-0 min-h-[34px] px-3 rounded-[7px] border border-solid border-paper-edge bg-paper text-sm text-ink"
  />
  <select
    data-add-item-kind
    bind:value={kind}
    aria-label="Its kind"
    class="min-h-[34px] px-2 rounded-[7px] border border-solid border-paper-edge bg-paper text-sm text-ink"
  >
    {#each KINDS as k (k.value)}<option value={k.value}>{k.label}</option>{/each}
  </select>
  <button
    type="button"
    onclick={() => void submit()}
    data-add-item-submit
    disabled={busy || !title.trim()}
    class="min-h-[34px] px-3.5 rounded-lg border-0 bg-ink text-paper text-sm cursor-pointer disabled:opacity-60"
  >Add</button>
  {#if error}<div class="basis-full"><Notice size="inline" tone="fault" title={error} data-add-item-error /></div>{/if}
</form>
