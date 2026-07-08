<script lang="ts">
  import HeroKoan from './HeroKoan.svelte';
  import type { ObservatoryTodayHero } from '$lib/types.js';

  let {
    hero,
    mode,
    actionHref,
  }: {
    hero: ObservatoryTodayHero;
    mode: 'early' | 'mature';
    actionHref?: string;
  } = $props();
</script>

<HeroKoan {hero} {mode} {actionHref} />
