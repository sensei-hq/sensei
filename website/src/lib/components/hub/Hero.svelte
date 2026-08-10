<script lang="ts">
  import { Button } from '@rokkit/ui';
  import { base } from '$app/paths';
  import { PRODUCTS } from '$lib/hub-data.js';

  const meta = ['Independent studio', 'Est. 2024', 'Three products · four libraries'];
  const workshopCount = String(PRODUCTS.length).padStart(2, '0');

  // Internal routes (/sensei, /torii-seiki) get the base prefix and open in place;
  // the library sites (absolute https URLs) open in a new tab.
  const isExternal = (p: typeof PRODUCTS[number]): boolean => p.href.startsWith('http');
  const productHref = (p: typeof PRODUCTS[number]): string =>
    isExternal(p) ? p.href : `${base}${p.href}`;
</script>

<header id="top" class="hero-wrap mx-auto px-7 pt-7 pb-9">
  <div class="hero-grid gap-8">
    <!-- Statement -->
    <div>
      <div class="flex items-baseline gap-3 mb-5">
        <span class="kanji text-accent text-3xl" style="line-height:1">道</span>
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Dō · the way — an independent studio</span>
      </div>
      <h1 class="display text-ink m-0 hero-h1">
        We build quiet, sharp tools for the people who build software.
      </h1>
      <p class="text-ink-soft mt-5 text-lg" style="line-height:1.6;max-width:540px">
        Sensei HQ is a small workshop of developer tools — three products you
        open, four libraries you build on. Restraint over noise, craft over
        scale, and a deep respect for the person on the other side of the screen.
      </p>
      <div class="flex items-center flex-wrap gap-3 mt-6">
        <Button href="#products" variant="primary" size="lg">
          <span class="kanji text-base" style="line-height:1">見</span>
          See the tools
        </Button>
        <a href="#approach" class="text-ink-soft text-sm no-underline">How we work ↓</a>
      </div>
      <div class="flex flex-wrap gap-4 mt-7">
        {#each meta as m (m)}
          <span class="mono text-ink-mute text-xs">{m}</span>
        {/each}
      </div>
    </div>

    <!-- In the workshop -->
    <aside class="border border-paper-edge rounded-lg bg-paper-soft" style="overflow:hidden">
      <div class="px-5 py-4 border-b border-paper-edge flex items-center justify-between">
        <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">In the workshop</span>
        <span class="mono text-ink-faint text-xs">{workshopCount}</span>
      </div>
      <div class="divide-y divide-paper-edge">
        {#each PRODUCTS as p (p.id)}
          <a href={productHref(p)}
             target={isExternal(p) ? '_blank' : undefined}
             rel={isExternal(p) ? 'noopener' : undefined}
             class="workshop-row gap-4 px-5 py-4 no-underline text-{p.id}">
            <span class="kanji text-2xl" style="line-height:1;width:30px;text-align:center">{p.kanji}</span>
            <span>
              <span class="display text-ink block text-base">{p.name}</span>
              <span class="text-ink-mute text-xs">{p.category}</span>
            </span>
            <span class="product-dot" style="background:var(--{p.id})"></span>
          </a>
        {/each}
      </div>
    </aside>
  </div>
</header>

<style>
  .hero-wrap {
    max-width: 1120px;
  }
  .hero-grid {
    display: grid;
    grid-template-columns: 1.55fr 1fr;
    align-items: start;
  }
  .hero-h1 {
    font-size: 60px;
    font-weight: 300;
    line-height: 1.08;
    letter-spacing: -0.025em;
    max-width: 640px;
  }
  .workshop-row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    transition: background 0.15s;
  }
  .workshop-row:hover {
    background: var(--paper-mute);
  }
  .product-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  @media (max-width: 768px) {
    .hero-grid {
      grid-template-columns: 1fr;
    }
    .hero-h1 {
      font-size: var(--text-3xl);
    }
  }
</style>
