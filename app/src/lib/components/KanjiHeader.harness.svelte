<script lang="ts">
  import KanjiHeader from './KanjiHeader.svelte';
  let { kanji, eyebrow, title: titleText, withRight = false }: {
    kanji: string; eyebrow: string; title: string; withRight?: boolean;
  } = $props();
</script>

<KanjiHeader {kanji} {eyebrow}>
  {#snippet title()}{titleText}{/snippet}
  {#snippet right()}
    {#if withRight}<span data-test="harness-right">R</span>{/if}
  {/snippet}
</KanjiHeader>
