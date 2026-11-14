<script lang="ts">
  // An honest empty — the read worked and there is nothing to show. Never a
  // failure: a failed read is a Notice (StateView's error), so this is
  // `role=status`, not alert. A faint mark (glyph mode), one landing line, a
  // description (prop or children), and the next step when there is one.
  // `panel` is a bordered card (inside a screen); `screen` is centred, no card.
  import type { Snippet } from 'svelte';
  import Mark from './Mark.svelte';
  import ActionButton from './ActionButton.svelte';
  import type { NoticeAction } from './types.js';

  let {
    kanji = '空',
    title,
    description,
    action,
    size = 'panel',
    children,
  }: {
    kanji?: string;
    title: string;
    description?: string;
    action?: NoticeAction;
    size?: 'panel' | 'screen';
    children?: Snippet;
  } = $props();
</script>

<div
  data-component="empty-state"
  data-empty
  data-size={size}
  role="status"
  class="flex flex-col items-center justify-center text-center {size === 'screen' ? 'py-20 px-6' : 'py-16 px-8'}"
>
  <div
    class="flex max-w-sm flex-col items-center gap-3 {size === 'panel' ? 'border-paper-edge bg-paper-soft rounded-lg border p-8' : ''}"
  >
    <Mark char={kanji} class="text-ink-faint text-3xl" />
    <p data-part="title" class="display text-ink m-0 text-lg font-normal">{title}</p>
    {#if description || children}
      <p data-part="description" class="text-ink-mute m-0 text-sm leading-normal">
        {#if children}{@render children()}{:else}{description}{/if}
      </p>
    {/if}
    {#if action}<div class="mt-2"><ActionButton {action} /></div>{/if}
  </div>
</div>
