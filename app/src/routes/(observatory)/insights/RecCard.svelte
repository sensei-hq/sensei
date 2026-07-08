<script lang="ts">
  import { RECOMMENDED_VERB, type RecCardVm } from './insights-board.svelte.js';

  let {
    rec,
    focal = false,
    onApply,
    onReview,
    onDismiss,
  }: {
    rec: RecCardVm;
    focal?: boolean;
    onApply: () => void;
    onReview: () => void;
    onDismiss: () => void;
  } = $props();
</script>

<!--
  RecCardSlim — one recommendation collapsed to a single decision. Apply is
  the highlighted, recommended verb (bg-ink); Review and Dismiss are secondary.
  Identical action row on every rec card — the one-decision-one-default theme.
-->
<article
  data-component="rec-card"
  data-rec-id={rec.id}
  class="rounded bg-paper-soft border border-paper-edge py-2 px-3"
  class:border-l-2={focal}
  class:border-l-accent={focal}
>
  <!-- project chip + (focal) do-first marker -->
  <div class="flex items-center gap-2 mb-1">
    <span class="inline-flex items-center gap-1 text-xs text-ink-soft" data-rec-project>
      <span class="kanji text-xs text-accent">{rec.projectKanji}</span>
      <span class="truncate max-w-[140px]">{rec.projectName}</span>
    </span>
    <span class="flex-1"></span>
    {#if focal}
      <span class="text-xs uppercase tracking-wider text-accent font-medium" data-do-first>do first</span>
    {/if}
  </div>

  <p class="text-sm text-ink m-0 leading-snug">{rec.title}</p>

  {#if rec.why}
    <p class="text-xs text-ink-soft mt-1 m-0 leading-normal line-clamp-2">{rec.why}</p>
  {/if}

  {#if rec.impact}
    <p class="text-xs text-ink-faint mt-2 m-0 leading-snug" data-rec-impact>{rec.impact}</p>
  {/if}

  <!-- one consistent action row — Apply highlighted as recommended -->
  <div class="flex items-center gap-2 mt-2">
    <button
      type="button"
      data-verb="apply"
      data-recommended
      class="text-xs bg-ink text-paper rounded-sm py-1 px-3 border-none cursor-pointer"
      onclick={onApply}
    >
      {RECOMMENDED_VERB}
    </button>
    <button
      type="button"
      data-verb="review"
      class="text-xs text-ink-soft bg-transparent border border-paper-edge rounded-sm py-1 px-3 cursor-pointer"
      onclick={onReview}
    >
      Review
    </button>
    <span class="flex-1"></span>
    <button
      type="button"
      data-verb="dismiss"
      class="text-xs text-ink-faint bg-transparent border-none py-1 px-1 cursor-pointer"
      onclick={onDismiss}
    >
      Dismiss
    </button>
  </div>
</article>
