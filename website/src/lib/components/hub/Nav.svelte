<script lang="ts">
  import { base } from '$app/paths';
  import { Button } from '@rokkit/ui';
  import { NAV_LINKS } from '$lib/hub-data.js';
</script>

<div class="hub-nav">
  <nav class="mx-auto px-7 py-4 hub-nav-inner">
    <a href="{base}/" class="flex items-center gap-3 no-underline">
      <!-- Sensei HQ brushed-ensō mark (i-brand:sensei-hq), tinted to the brand vermillion -->
      <span class="i-brand:sensei-hq text-primary flex-shrink-0 block" style="width:28px;height:28px" aria-hidden="true"></span>
      <span class="flex items-baseline gap-2">
        <span class="display text-ink" style="font-size:18px;letter-spacing:-0.01em">Sensei</span>
        <span class="mono text-ink-mute" style="font-size:11px;letter-spacing:0.08em">HQ</span>
      </span>
    </a>

    <div class="flex items-center gap-6">
      {#each NAV_LINKS as [href, label] (href)}
        <a {href} class="text-ink-soft text-sm nav-link no-underline">{label}</a>
      {/each}
      <Button href="https://github.com/sensei-hq" target="_blank" rel="noopener"
              variant="default" size="sm" label="GitHub" class="ml-1" />
    </div>
  </nav>
</div>

<style>
  .hub-nav {
    position: sticky;
    top: 0;
    z-index: 50;
    background: color-mix(in oklch, var(--paper) 80%, transparent);
    backdrop-filter: blur(14px) saturate(150%);
    -webkit-backdrop-filter: blur(14px) saturate(150%);
    -webkit-mask-image: linear-gradient(to bottom, #000 72%, transparent);
    mask-image: linear-gradient(to bottom, #000 72%, transparent);
    padding-bottom: 6px;
  }
  .hub-nav-inner {
    max-width: 1120px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .nav-link {
    transition: color 0.15s;
  }
  .nav-link:hover {
    color: var(--ink);
  }
</style>
