<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import { Wordmark, Eyebrow } from '$lib/components';

  interface Props { state: HealthState; }
  let { state }: Props = $props();

  const headlineKeyTone = $derived(
    state.display.headlineTone === 'success' ? 'text-success'
    : state.display.headlineTone === 'accent' ? 'text-accent'
    :                                            'text-ink-mute',
  );
</script>

<header class="flex flex-col gap-4 mb-6">
  <Wordmark size="md" />

  <div class="flex flex-col gap-2">
    <Eyebrow>{state.display.eyebrow}</Eyebrow>

    <h1 class="font-display text-3xl font-light leading-tight tracking-tight text-ink">
      {state.display.headlinePre}
      <span class={headlineKeyTone}>{state.display.headlineKey}</span>
    </h1>

    <p data-sub class="text-sm text-ink-soft leading-relaxed max-w-prose">
      {state.display.subCopy}
    </p>
  </div>
</header>
