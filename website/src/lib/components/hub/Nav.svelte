<script lang="ts">
  import { base } from '$app/paths';
  import SiteNav from '$lib/components/SiteNav.svelte';
  import { NAV_LINKS } from '$lib/hub-data.js';
</script>

<SiteNav
  links={NAV_LINKS}
  cta={{ href: 'https://github.com/sensei-hq', target: '_blank', rel: 'noopener', style: 'outline', label: 'GitHub' }}>
  {#snippet brand()}
    <a href="{base}/" class="flex items-center gap-3 no-underline">
      <!-- Sensei HQ brushed-ensō mark (i-brand:sensei-hq), tinted to the brand vermillion -->
      <span class="i-brand:sensei-hq text-accent flex-shrink-0 block" style="width:28px;height:28px" aria-hidden="true"></span>
      <span class="flex items-baseline gap-2">
        <span class="display text-ink text-lg" style="letter-spacing:-0.01em">Sensei</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.08em">HQ</span>
      </span>
    </a>
  {/snippet}
</SiteNav>
