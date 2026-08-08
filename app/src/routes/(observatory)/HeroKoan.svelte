<script lang="ts">
  import { Kanji } from '$lib/components';
  import { Button } from '@rokkit/ui';
  import { heroProvenance } from './today-view.svelte.js';
  import type { ObservatoryTodayHero } from '$lib/types.js';

  interface Props {
    hero: ObservatoryTodayHero;
    mode: 'early' | 'mature';
    /** Where the koan action routes. The hero is the top recommendation, so
     *  it opens the insights review surface by default. */
    actionHref?: string;
  }
  let { hero, mode, actionHref = '/insights' }: Props = $props();

  const provenance = $derived(heroProvenance(hero));
</script>

<div
  data-component="hero-koan"
  data-mode={mode}
  class="relative grid grid-cols-[auto_1fr] gap-5 px-6 py-6 bg-paper-soft border border-paper-edge rounded-lg"
>
  <!-- Giant kanji anchor + vertical caption -->
  <div class="relative">
    <Kanji char={hero.kanji} size="4xl" tone={mode === 'early' ? 'watermark' : 'accent'} />
    <span
      class="absolute -left-1.5 -top-1 text-xs tracking-wide uppercase text-ink-soft"
      style="writing-mode: vertical-rl; transform: rotate(180deg); height: 96px;"
    >{mode === 'early' ? 'sensei is listening' : 'sensei speaks'}</span>
  </div>

  <div class="flex flex-col">
    <p class="display text-2xl font-normal m-0 mb-3 tracking-tight leading-tight text-ink">
      {hero.koan}
    </p>
    <p class="text-base text-ink-mute leading-relaxed m-0 mb-4 max-w-[620px]">{hero.body}</p>

    <div class="flex items-center gap-4 flex-wrap mt-auto">
      {#if hero.action}
        <Button href={actionHref} variant="primary" size="sm" data-hero-action>{hero.action} →</Button>
      {/if}
      {#if hero.impact}
        <div data-hero-impact class="flex items-center gap-1 text-sm text-accent">
          <span class="w-[5px] h-[5px] rounded-full bg-accent"></span>
          <span>{hero.impact}</span>
        </div>
      {/if}
      <span class="flex-1"></span>
      {#if provenance}
        <span data-hero-source class="font-mono text-xs text-ink-soft">{provenance}</span>
      {/if}
    </div>
  </div>
</div>
