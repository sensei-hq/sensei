<script lang="ts">
  import PageHeader from './PageHeader.svelte';

  // Test-friendly wrapper: plain props only, so a spec can drive every prop
  // without composing snippets. `rightText` stands in for the right slot.
  let {
    title,
    eyebrow,
    kanji,
    icon,
    description,
    clampDescription = false,
    size = 'md',
    bordered = true,
    padded = true,
    rightText,
  }: {
    title: string;
    eyebrow?: string;
    kanji?: string;
    icon?: string;
    description?: string;
    clampDescription?: boolean;
    size?: 'sm' | 'md' | 'lg';
    bordered?: boolean;
    padded?: boolean;
    rightText?: string;
  } = $props();
</script>

<PageHeader
  {title}
  {eyebrow}
  {kanji}
  {icon}
  {description}
  {clampDescription}
  {size}
  {bordered}
  {padded}
>
  {#snippet right()}
    {#if rightText}<span>{rightText}</span>{/if}
  {/snippet}
</PageHeader>
