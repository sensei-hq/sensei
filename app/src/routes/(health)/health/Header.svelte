<script lang="ts">
  import type { HealthStatus } from '$lib/health-types.js';
  import { Eyebrow } from '$lib/components';

  interface Props { status: HealthStatus; }
  let { status }: Props = $props();

  const eyebrowText = $derived.by(() => {
    switch (status) {
      case 'checking':     return 'starting';
      case 'resolving':    return 'setting up';
      case 'needs-action': return 'needs your hand';
      case 'ok':           return 'ready';
    }
  });

  const subCopy = $derived.by(() => {
    switch (status) {
      case 'checking':
        return 'A quick health check before opening the observatory.';
      case 'resolving':
        return 'Running brew bundle from sensei-hq/homebrew-tap. No input needed.';
      case 'needs-action':
        return "Run the script below — it'll install everything else.";
      case 'ok':
        return 'Foundation holds. Opening the observatory…';
    }
  });
</script>

<header class="flex flex-col gap-4 mb-6">
  <div class="flex items-baseline gap-2.5">
    <span class="kanji text-2xl text-primary-z6 leading-none">先生</span>
    <span class="display text-xl font-normal tracking-tight text-surface-z9">Sensei</span>
  </div>

  <div class="flex flex-col gap-2">
    <Eyebrow>{eyebrowText}</Eyebrow>

    <h1 class="display text-3xl font-light leading-tight tracking-tight text-surface-z9">
      {#if status === 'ok'}
        The foundation <span class="text-success-z5">holds.</span>
      {:else if status === 'resolving'}
        Setting up your <span class="text-primary-z5">foundation.</span>
      {:else if status === 'checking'}
        Checking the <span class="text-surface-z7">foundation…</span>
      {:else}
        One last <span class="text-primary-z5">step.</span>
      {/if}
    </h1>

    <p data-sub class="text-sm text-surface-z6 leading-relaxed max-w-prose">
      {subCopy}
    </p>
  </div>
</header>
