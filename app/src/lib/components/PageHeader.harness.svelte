<script lang="ts">
  import PageHeader from './PageHeader.svelte';

  let {
    title,
    eyebrow,
    kanji,
    description,
    variant = 'h2',
    bordered = true,
    rightText,
  }: {
    title: string;
    eyebrow?: string;
    kanji?: string;
    description?: string;
    variant?: 'h1' | 'h2' | 'h3';
    bordered?: boolean;
    rightText?: string;
  } = $props();
</script>

<PageHeader {title} {eyebrow} {kanji} {description} {variant} {bordered}>
  {#snippet right()}
    {#if rightText}<span>{rightText}</span>{/if}
  {/snippet}
</PageHeader>
