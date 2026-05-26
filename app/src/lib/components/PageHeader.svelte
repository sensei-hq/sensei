<script lang="ts">
  import type { Snippet } from 'svelte';
  import Eyebrow from './Eyebrow.svelte';
  import Kanji from './Kanji.svelte';

  type Variant = 'h1' | 'h2' | 'h3';

  let {
    title,
    eyebrow,
    kanji,
    description,
    variant = 'h2',
    bordered = true,
    right,
  }: {
    title: string;
    eyebrow?: string;
    kanji?: string;
    description?: string;
    variant?: Variant;
    bordered?: boolean;
    right?: Snippet;
  } = $props();

  const titleSize = $derived(
    ({ h1: 'text-2xl', h2: 'text-xl', h3: 'text-lg' })[variant],
  );
  const kanjiSize = $derived(
    ({ h1: '3xl', h2: '2xl', h3: 'xl' })[variant] as
      | '3xl'
      | '2xl'
      | 'xl',
  );
</script>

<header
  data-component="page-header"
  class="flex items-center gap-5 pt-5 pb-4 px-6 {bordered ? 'border-b border-paper-z2' : ''}"
>
  {#if kanji}
    <Kanji char={kanji} size={kanjiSize} />
  {/if}
  <div class="flex-1 min-w-0">
    {#if eyebrow}
      <div class="mb-1"><Eyebrow>{eyebrow}</Eyebrow></div>
    {/if}
    <h1 class="display {titleSize} font-normal m-0 tracking-tight text-ink-z9">{title}</h1>
    {#if description}
      <p class="text-sm text-ink-z8 leading-normal m-0 mt-1 max-w-[720px]">{description}</p>
    {/if}
  </div>
  {#if right}
    <div class="ml-auto">{@render right()}</div>
  {/if}
</header>
