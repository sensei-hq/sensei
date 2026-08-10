<script lang="ts">
  import { base } from '$app/paths';
  import type { Product } from '$lib/hub-data.js';

  let { p }: { p: Product } = $props();

  // Internal routes are authored as `/sensei`, `/torii-seiki`; everything else
  // (the library sites) is an absolute https URL opened in a new tab.
  const isExternal = $derived(p.href.startsWith('http'));
  const href = $derived(isExternal ? p.href : `${base}${p.href}`);

  function statusClass(status: string): string {
    if (status === 'Available') return 'text-accent bg-accent-soft';
    if (status === 'Beta') return 'text-warning bg-warning-soft';
    return 'text-success bg-success-soft';
  }
</script>

<!-- Featured card (full-width, two-column) -->
{#if p.featured}
  <a {href}
     target={isExternal ? '_blank' : undefined}
     rel={isExternal ? 'noopener' : undefined}
     class="featured-card border border-paper-edge rounded-lg no-underline text-{p.id}"
     style="overflow:hidden">
    <div class="p-7">
      <div class="flex items-center justify-between mb-5">
        <span class="mono text-ink-faint text-xs">{p.index}</span>
        <span class="mono rounded-sm px-2 py-0.5 {statusClass(p.status)} text-xs" style="white-space:nowrap">{p.status}</span>
      </div>
      <div class="flex items-baseline gap-3 mb-2">
        <span class="display text-ink text-2xl" style="font-weight:400;letter-spacing:-0.02em">{p.name}</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.1em;text-transform:uppercase">{p.category}</span>
      </div>
      <p class="display text-ink m-0 text-xl" style="font-weight:300;letter-spacing:-0.015em;line-height:1.3">
        {p.tagline}
      </p>
      <p class="text-ink-soft mt-3 text-sm" style="line-height:1.65;max-width:460px">{p.blurb}</p>
      <div class="flex flex-col gap-2 mt-5 mb-6">
        {#each (p.highlights ?? []) as h (h)}
          <div class="flex items-center gap-3">
            <span class="product-dot" style="background:var(--{p.id})"></span>
            <span class="text-ink-soft text-sm">{h}</span>
          </div>
        {/each}
      </div>
      <div class="flex items-center justify-between flex-wrap gap-4">
        <div class="flex flex-wrap gap-2">
          {#each p.meta as m (m)}
            <span class="mono text-ink-mute border border-paper-edge rounded-sm text-xs px-2 py-1" style="white-space:nowrap">{m}</span>
          {/each}
        </div>
        <span class="text-{p.id} text-sm font-medium">Explore {p.name} →</span>
      </div>
    </div>
    <!-- Kanji panel -->
    <div class="kanji-panel border-l border-paper-edge">
      <span class="kanji text-{p.id}" style="font-size:220px;line-height:1;opacity:0.92">{p.kanji}</span>
      <span class="mono text-xs" style="position:absolute;bottom:18px;right:20px;letter-spacing:0.18em;text-transform:uppercase;color:var(--ink-faint)">Kan · to observe</span>
    </div>
  </a>
{:else}
  <!-- Regular product card -->
  <a {href}
     target={isExternal ? '_blank' : undefined}
     rel={isExternal ? 'noopener' : undefined}
     class="product-card border border-paper-edge rounded-lg bg-paper-soft no-underline text-{p.id}">
    <div class="p-6 flex flex-col" style="flex:1">
      <div class="flex items-center justify-between">
        <span class="mono text-ink-faint text-xs">{p.index}</span>
        <span class="mono rounded-sm px-2 py-0.5 {statusClass(p.status)} text-xs" style="white-space:nowrap">{p.status}</span>
      </div>
      <span class="kanji text-{p.id} text-4xl mt-5 mb-4" style="line-height:1">{p.kanji}</span>
      <div class="flex items-baseline gap-2 mb-1">
        <span class="display text-ink text-xl" style="font-weight:400;letter-spacing:-0.02em">{p.name}</span>
      </div>
      <span class="mono text-ink-mute mb-3 text-xs" style="letter-spacing:0.1em;text-transform:uppercase">{p.category}</span>
      <p class="display text-ink m-0 text-lg" style="font-weight:400;line-height:1.3">{p.tagline}</p>
      <p class="text-ink-soft mt-2 text-sm" style="line-height:1.6">{p.blurb}</p>
      <div style="flex:1"></div>
      <div class="mt-5 mb-4 flex flex-wrap gap-2">
        {#each p.meta as m (m)}
          <span class="mono text-ink-mute border border-paper-edge rounded-sm text-xs px-2 py-1" style="white-space:nowrap">{m}</span>
        {/each}
      </div>
      <div class="border-t border-paper-edge pt-4 flex items-center justify-between">
        <span class="text-{p.id} text-sm font-medium">Explore {p.name} →</span>
      </div>
    </div>
  </a>
{/if}

<style>
  .featured-card {
    display: grid;
    grid-template-columns: 1.3fr 1fr;
    transition: border-color 0.18s;
  }
  .featured-card:hover {
    border-color: var(--accent);
  }
  .kanji-panel {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--paper-mute);
    position: relative;
    min-height: 320px;
    overflow: hidden;
  }
  .product-card {
    display: flex;
    flex-direction: column;
    transition: border-color 0.18s, transform 0.18s;
  }
  .product-card:hover {
    transform: translateY(-2px);
  }
  .product-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>
