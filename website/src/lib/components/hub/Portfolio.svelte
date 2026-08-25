<script lang="ts">
  import { productList, libraryList, featuredProduct } from '$lib/hub-data.js';
  import ProductCard from './ProductCard.svelte';
  import LibraryRow from './LibraryRow.svelte';

  const featured = featuredProduct()!;
  const rest = productList().filter((p) => !p.featured);
  const libraries = libraryList();
</script>

<section id="products" class="bg-paper-soft">
  <div class="portfolio-inner mx-auto px-7 py-24">
    <div class="flex items-end justify-between flex-wrap gap-4 mb-12">
      <div>
        <span class="mono text-ink-mute mb-3 block text-xs" style="letter-spacing:0.12em;text-transform:uppercase">The portfolio</span>
        <h2 class="display text-ink m-0 text-3xl" style="font-weight:300;letter-spacing:-0.02em">
          Three products. Four libraries.
        </h2>
      </div>
      <p class="text-ink-soft text-sm" style="line-height:1.6;max-width:360px">
        The products are things you open. The libraries are things you build on.
        Both carry the same temperament and the same refusal to get in your way.
      </p>
    </div>

    <div class="mb-6">
      <ProductCard p={featured} />
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      {#each rest as p (p.id)}
        <ProductCard {p} />
      {/each}
    </div>

    <!-- Libraries — the foundation stones our products stand on -->
    <div id="libraries" class="mt-24">
      <div class="flex items-end justify-between flex-wrap gap-4 mb-8">
        <div>
          <div class="flex items-baseline gap-3 mb-3">
            <span class="kanji text-accent text-2xl" style="line-height:1">礎</span>
            <span class="mono text-ink-mute text-xs" style="letter-spacing:0.12em;text-transform:uppercase">Soseki · the foundation stones</span>
          </div>
          <h2 class="display text-ink m-0 text-2xl" style="font-weight:300;letter-spacing:-0.02em">
            What our products stand on.
          </h2>
        </div>
        <p class="text-ink-soft text-sm" style="line-height:1.6;max-width:340px">
          Written for our own tools first, then published — so the thing you
          install is the thing we depend on.
        </p>
      </div>
      <div class="border border-paper-edge rounded-lg bg-paper" style="overflow:hidden">
        {#each libraries as p (p.id)}
          <LibraryRow {p} />
        {/each}
      </div>
    </div>
  </div>
</section>

<style>
  .portfolio-inner {
    max-width: 1120px;
  }
</style>
