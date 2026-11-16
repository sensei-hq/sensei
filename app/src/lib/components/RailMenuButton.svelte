<script lang="ts">
  // Opens the Rail as a drawer below md (#420); from md up the Rail is always shown and
  // this button is not drawn. `no-drag` so a window's drag strip can hold it.
  import { railDrawer } from './rail-drawer.svelte.js';
</script>

<button
  type="button"
  data-rail-menu
  aria-label="Open the menu"
  aria-expanded={railDrawer.open}
  onclick={() => railDrawer.toggle()}
  class="no-drag md:hidden inline-flex items-center justify-center w-8 h-8 rounded-lg border-0 bg-transparent text-ink-soft hover:bg-paper-mute cursor-pointer"
><span aria-hidden="true" class="i-solar:hamburger-menu-linear text-lg"></span></button>
