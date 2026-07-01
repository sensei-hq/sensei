<script lang="ts">
  interface Props { size?: 'sm' | 'md' | 'lg'; }
  let { size = 'md' }: Props = $props();

  // The mark is a square icon (sensei.svg). Size it to sit beside the word.
  const markSizeClass = $derived(
    size === 'sm' ? 'h-5 w-5'
    : size === 'lg' ? 'h-9 w-9'
    :                 'h-7 w-7',
  );
  const wordSizeClass = $derived(
    size === 'sm' ? 'text-sm'
    : size === 'lg' ? 'text-xl'
    :                 'text-base',
  );
</script>

<div class="flex items-center gap-2">
  <img
    data-component="wordmark-mark"
    src="/sensei.svg"
    alt=""
    class="{markSizeClass} select-none"
    draggable="false"
  />
  <span
    data-component="wordmark-word"
    class="font-display font-normal tracking-tight text-ink {wordSizeClass}"
  >sensei</span>
</div>
