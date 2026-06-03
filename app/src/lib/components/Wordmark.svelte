<script lang="ts">
  interface Props { size?: 'sm' | 'md' | 'lg'; }
  let { size = 'md' }: Props = $props();

  const kanjiSizeClass = $derived(
    size === 'sm' ? 'text-lg'
    : size === 'lg' ? 'text-3xl'
    :                 'text-2xl',
  );
  const wordSizeClass = $derived(
    size === 'sm' ? 'text-sm'
    : size === 'lg' ? 'text-xl'
    :                 'text-base',
  );
</script>

<div class="flex items-baseline gap-2">
  <span
    data-component="wordmark-kanji"
    class="font-kanji text-accent leading-none {kanjiSizeClass}"
  >先生</span>
  <span
    data-component="wordmark-word"
    class="font-display font-normal tracking-tight text-ink {wordSizeClass}"
  >Sensei</span>
</div>
