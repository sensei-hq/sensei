<script lang="ts">
  // A Now section's heading (COMPONENTS §2): 12px, .12em, uppercase, ink-mute, 500;
  // an optional mark in its tone; "Label · N" when it counts something; and, when a
  // stat's filter chose this section, "Showing: X" with Clear (CONTRACT §1c-0).
  let {
    label,
    count,
    mark,
    showing,
  }: {
    label: string;
    count?: number;
    mark?: { glyph: string; tone: string };
    showing?: { label: string; onclear: () => void } | null;
  } = $props();
</script>

<div class="flex flex-wrap items-center gap-2">
  <h3
    data-section-heading
    class="m-0 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.12em] text-ink-mute"
  >
    {#if mark}<span aria-hidden="true" class="font-kanji {mark.tone}">{mark.glyph}</span>{/if}
    {label}{#if count !== undefined}&nbsp;·&nbsp;{count}{/if}
  </h3>
  {#if showing}
    <span
      data-showing
      class="inline-flex items-center gap-1.5 min-h-7 py-0 pl-2.5 pr-1 rounded-full border border-solid border-ink text-xs"
    >
      Showing: <strong>{showing.label}</strong>
      <button
        type="button"
        onclick={showing.onclear}
        class="min-h-[22px] px-2 rounded-full border-0 bg-paper-mute text-ink text-xs cursor-pointer"
      >Clear</button>
    </span>
  {/if}
</div>
