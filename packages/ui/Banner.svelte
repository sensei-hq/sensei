<script lang="ts">
  // A strip across a screen or the app: degraded, a partial read, a quiet note.
  // The Notice tones (fault, refusal, action, normal) on their tints, text on a
  // tint from `text-on-*-soft` (BANNER_TONES); a mark that follows the glyph
  // mode; an optional next step and a `right` slot. `bleed` spans the app. Copy comes from the caller's
  // state module, as for Notice.
  import type { Snippet } from 'svelte';
  import Mark from './Mark.svelte';
  import ActionButton from './ActionButton.svelte';
  import { BANNER_TONES, noticeTone } from './notice.js';
  import type { NoticeAction, NoticeTone } from './types.js';

  let {
    tone = 'normal',
    title,
    detail,
    mark,
    action,
    bleed = false,
    right,
    children,
    ...rest
  }: {
    tone?: NoticeTone | string;
    title?: string;
    detail?: string;
    mark?: string;
    action?: NoticeAction;
    /** Full width across the app: no rounding, an edge only below. */
    bleed?: boolean;
    right?: Snippet;
    children?: Snippet;
    [attr: `data-${string}`]: unknown;
  } = $props();

  const key = $derived((tone in BANNER_TONES ? tone : 'fault') as NoticeTone);
  const t = $derived(BANNER_TONES[key]);
  const role = $derived(noticeTone(key).role);
</script>

<div
  {...rest}
  data-component="banner"
  data-tone={key}
  {role}
  class="flex items-start gap-3 px-4 py-3 {bleed ? 'border-b' : 'rounded-lg border'} {t.bg} {t.edge}"
>
  <Mark char={mark ?? noticeTone(key).kanji} tone={key} class="{t.mark} text-lg" />
  <div class="min-w-0 flex-1">
    {#if title}<p data-part="title" class="m-0 text-sm font-medium leading-snug {t.text}">{title}</p>{/if}
    {#if detail || children}
      <div data-part="detail" class="text-sm leading-normal {t.text}">
        {#if children}{@render children()}{:else}{detail}{/if}
      </div>
    {/if}
  </div>
  {#if action}<ActionButton {action} look="outline" />{/if}
  {#if right}{@render right()}{/if}
</div>
