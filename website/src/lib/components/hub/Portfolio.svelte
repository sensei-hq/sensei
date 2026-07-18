<script lang="ts">
  import { PRODUCTS } from '$lib/hub-data.js';
  import ProductCard from './ProductCard.svelte';

  const featured = PRODUCTS.find((p) => p.featured)!;
  const rest = PRODUCTS.filter((p) => !p.featured);
</script>

<section id="products" class="bg-paper-soft">
  <div class="portfolio-inner mx-auto px-7 py-24">
    <div class="flex items-end justify-between flex-wrap gap-4 mb-7">
      <div>
        <span class="mono text-ink-mute mb-3 block text-xs" style="letter-spacing:0.12em;text-transform:uppercase">The portfolio</span>
        <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;letter-spacing:-0.02em">
          Four tools, one workshop.
        </h2>
      </div>
      <p class="text-ink-soft text-sm" style="line-height:1.6;max-width:360px">
        Each carries its own mark and its own accent — but they share a
        temperament, a type system, and a refusal to get in your way.
      </p>
    </div>

    <div class="mb-5">
      <ProductCard p={featured} />
    </div>

    <div class="products-grid gap-5">
      {#each rest as p (p.id)}
        <ProductCard {p} />
      {/each}
    </div>
  </div>
</section>

<style>
  .portfolio-inner {
    max-width: 1120px;
  }
  .products-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }
  @media (max-width: 900px) {
    .products-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
