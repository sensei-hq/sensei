<script lang="ts">
  // Create a sprint over the selected backlog items (queue cutover, phase 4):
  // a title, the goal Task 0 decomposes, and the gate's knobs. Submit stays off
  // until a title, a goal and — with auto-merge on — the base checkout are there.
  // One write per press; a refusal is shown as the daemon said it.
  import { untrack } from 'svelte';
  import { Notice } from '@sensei/ui';
  import type { RelayBacklogItem } from '$lib/relay-api.js';
  import type { RelaySprintDraft } from '@sensei/state';

  let {
    selected,
    defaults,
    error = null,
    onsubmit,
    oncancel,
  }: {
    selected: RelayBacklogItem[];
    defaults: { base_branch: string; base_checkout: string; verify_cmd: string };
    /** The daemon's refusal of the last press, or null. */
    error?: string | null;
    onsubmit: (draft: RelaySprintDraft) => Promise<string | null>;
    oncancel: () => void;
  } = $props();

  let title = $state('');
  let goal = $state('');
  // Seeded once from the defaults: the form is the person's to edit after that.
  let baseBranch = $state(untrack(() => defaults.base_branch));
  let baseCheckout = $state(untrack(() => defaults.base_checkout));
  let verifyCmd = $state(untrack(() => defaults.verify_cmd));
  let autoMerge = $state(true);
  let busy = $state(false);

  const nothing = (s: string) => s.trim().length === 0;
  const ready = $derived(
    !nothing(title) && !nothing(goal) && (!autoMerge || !nothing(baseCheckout)),
  );
  const orNull = (s: string): string | null => (nothing(s) ? null : s.trim());

  async function submit(): Promise<void> {
    if (busy || !ready) return;
    busy = true;
    try {
      await onsubmit({
        title: title.trim(),
        goal: goal.trim(),
        repo_key: null,
        workspace_id: null,
        config: {
          base_checkout: orNull(baseCheckout),
          base_branch: orNull(baseBranch),
          verify_cmd: orNull(verifyCmd),
          auto_merge: autoMerge,
        },
      });
    } finally {
      busy = false;
    }
  }
</script>

<div class="flex flex-col gap-3 border-b border-paper-edge bg-paper-soft p-4" data-create-sprint>
  <div class="flex items-center gap-2">
    <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Create sprint · {selected.length}</h2>
    <button type="button" class="ml-auto text-xs underline" data-sprint-cancel onclick={oncancel}>Cancel</button>
  </div>

  <ul class="m-0 p-0 list-none flex flex-col gap-1" data-sprint-items>
    {#each selected as item (item.item_id)}
      <li class="text-sm text-ink-soft" data-sprint-item={item.item_id}>{item.title}</li>
    {/each}
  </ul>

  <label class="flex flex-col gap-1 text-xs text-ink-mute">
    <span>Title</span>
    <input
      class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
      data-sprint-field="title"
      placeholder="Sprint 14: Broker hardening"
      bind:value={title}
    />
  </label>
  <label class="flex flex-col gap-1 text-xs text-ink-mute">
    <span>Goal · what Task 0 decomposes</span>
    <textarea
      class="min-h-16 rounded-sm border border-paper-edge bg-paper py-2 px-3 text-sm text-ink"
      data-sprint-field="goal"
      placeholder="Resolve the high-severity findings and add the claim queue"
      bind:value={goal}
    ></textarea>
  </label>

  <div class="flex flex-wrap gap-3">
    <label class="flex flex-col gap-1 text-xs text-ink-mute">
      <span>Base branch</span>
      <input
        class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
        data-sprint-field="base_branch"
        bind:value={baseBranch}
      />
    </label>
    <label class="flex flex-col gap-1 text-xs text-ink-mute grow">
      <span>Base checkout{autoMerge ? ' · required for the merge gate' : ''}</span>
      <input
        class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
        data-sprint-field="base_checkout"
        placeholder="/path/to/the/checkout on the base branch"
        bind:value={baseCheckout}
      />
    </label>
    <label class="flex flex-col gap-1 text-xs text-ink-mute">
      <span>Verify command</span>
      <input
        class="rounded-sm border border-paper-edge bg-paper py-1 px-2 text-sm text-ink"
        data-sprint-field="verify_cmd"
        bind:value={verifyCmd}
      />
    </label>
    <label class="flex items-center gap-2 text-xs text-ink-mute self-end">
      <input type="checkbox" data-sprint-field="auto_merge" bind:checked={autoMerge} />
      <span>Auto-merge approved tasks through the gate</span>
    </label>
  </div>

  <div class="flex items-center gap-3">
    {#if error}<Notice size="inline" tone="fault" title={error} data-sprint-error />{/if}
    <button
      type="button"
      class="ml-auto rounded-sm bg-accent px-3 py-1 text-xs text-on-accent disabled:opacity-60"
      data-sprint-submit
      disabled={!ready || busy}
      onclick={() => void submit()}
    >
      Create sprint
    </button>
  </div>
</div>
