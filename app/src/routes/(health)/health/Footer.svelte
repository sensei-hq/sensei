<script lang="ts">
  import type { Platform } from '$lib/health-types.js';

  interface Props {
    version: string;
    platform: Platform;
  }
  let { version, platform }: Props = $props();

  const PLATFORM_LABEL: Record<Platform, string> = {
    macos:   'macOS',
    linux:   'Linux',
    windows: 'Windows',
  };

  const displayVersion = $derived(version || 'dev');
</script>

<div class="flex items-center gap-2 text-xs text-ink-soft font-mono tracking-tight">
  <span>sensei {displayVersion}</span>
  <span class="w-[3px] h-[3px] rounded-full bg-ink-soft"></span>
  <span>{PLATFORM_LABEL[platform]}</span>
</div>
