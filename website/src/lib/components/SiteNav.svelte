<script lang="ts">
  import { Button } from '@rokkit/ui';
  import type { Snippet } from 'svelte';

  // Shared, responsive top nav for the hub and the Torii · Seiki page. Below the
  // collapse breakpoint the inline links + CTA fold into an accessible hamburger
  // disclosure (a real <button> with aria-expanded / aria-controls, closing on
  // navigation) so small-mobile visitors can still reach every destination.
  type Cta = {
    href: string;
    label: string;
    target?: string;
    rel?: string;
    variant?: 'default' | 'primary' | 'secondary' | 'accent' | 'danger';
    style?: 'default' | 'outline' | 'ghost' | 'link';
  };

  let {
    brand,
    links,
    cta,
  }: {
    brand: Snippet;
    links: readonly (readonly [string, string])[];
    cta: Cta;
  } = $props();

  let open = $state(false);
  const close = () => (open = false);
</script>

<div class="site-nav sticky top-0 z-50">
  <div class="site-nav-bar">
    <nav class="site-nav-inner mx-auto px-7 py-4 flex items-center justify-between">
      {@render brand()}

      <!-- Inline links (desktop / wide) -->
      <div class="hidden md:flex items-center gap-6">
        {#each links as [href, label] (href)}
          <a {href} class="text-ink-soft text-sm nav-link no-underline">{label}</a>
        {/each}
        <Button
          href={cta.href}
          target={cta.target}
          rel={cta.rel}
          variant={cta.variant}
          style={cta.style}
          size="sm"
          label={cta.label}
          class="ml-1" />
      </div>

      <!-- Hamburger (small mobile) -->
      <button
        type="button"
        class="site-nav-toggle inline-flex md:hidden text-ink"
        aria-label={open ? 'Close menu' : 'Open menu'}
        aria-expanded={open}
        aria-controls="site-nav-menu"
        onclick={() => (open = !open)}>
        {#if open}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true">
            <path d="M5 5l14 14M19 5L5 19" />
          </svg>
        {:else}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true">
            <path d="M4 7h16M4 12h16M4 17h16" />
          </svg>
        {/if}
      </button>
    </nav>
  </div>

  <!-- Disclosure panel (small mobile) -->
  {#if open}
    <div id="site-nav-menu" class="site-nav-menu md:hidden border-t border-paper-edge px-7 py-5 flex flex-col gap-5">
      {#each links as [href, label] (href)}
        <a {href} class="text-ink-soft text-base nav-link no-underline" onclick={close}>{label}</a>
      {/each}
      <Button
        href={cta.href}
        target={cta.target}
        rel={cta.rel}
        variant={cta.variant}
        style={cta.style}
        size="md"
        label={cta.label}
        onclick={close} />
    </div>
  {/if}
</div>

<style>
  .site-nav-bar {
    background: color-mix(in oklch, var(--paper) 80%, transparent);
    backdrop-filter: blur(14px) saturate(150%);
    -webkit-backdrop-filter: blur(14px) saturate(150%);
    -webkit-mask-image: linear-gradient(to bottom, #000 72%, transparent);
    mask-image: linear-gradient(to bottom, #000 72%, transparent);
    padding-bottom: 8px;
  }
  .site-nav-inner {
    max-width: 1120px;
  }
  .nav-link {
    transition: color 0.15s;
  }
  .nav-link:hover {
    color: var(--ink);
  }
  .site-nav-toggle {
    align-items: center;
    justify-content: center;
    padding: 4px;
    margin: -4px;
    background: transparent;
    border: none;
    cursor: pointer;
  }
  .site-nav-menu {
    background: var(--paper);
  }
</style>
