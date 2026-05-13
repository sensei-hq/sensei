<script lang="ts">
  import type { Platform, HealthStatus } from '$lib/health-types.js';

  interface Props { platform: Platform; status: HealthStatus; }
  let { platform, status }: Props = $props();

  const PLATFORM_LABEL: Record<Platform, string> = {
    macos: 'macOS', linux: 'Linux', windows: 'Windows',
  };
</script>

<header class="flex flex-col gap-3.5 mb-9">
  <div class="flex items-center gap-2.5">
    <span class="kanji text-xl text-primary-z5">支</span>
    <span class="text-2xs tracking-tag uppercase text-ink-z5">
      bootstrap · {PLATFORM_LABEL[platform]}
    </span>
  </div>

  <h1 class="display text-4xl font-light leading-tight tracking-tight text-ink-z1">
    {#if status === 'ok'}
      The foundation <span class="text-success-z5">holds.</span>
    {:else if status === 'resolving'}
      Setting up your <span class="text-primary-z5">foundation.</span>
    {:else if status === 'checking'}
      Checking the <span class="text-ink-z3">foundation…</span>
    {:else}
      One last <span class="text-primary-z5">step.</span>
    {/if}
  </h1>
</header>
