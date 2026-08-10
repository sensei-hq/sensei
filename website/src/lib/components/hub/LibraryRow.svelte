<script lang="ts">
  import { base } from '$app/paths';
  import type { Product } from '$lib/hub-data.js';

  let { p }: { p: Product } = $props();

  const isExternal = $derived(p.href.startsWith('http'));
  const href = $derived(isExternal ? p.href : `${base}${p.href}`);

  function statusClass(status: string): string {
    if (status === 'Available') return 'text-accent bg-accent-soft';
    if (status === 'Beta') return 'text-warning bg-warning-soft';
    return 'text-success bg-success-soft';
  }
</script>

<a {href}
   target={isExternal ? '_blank' : undefined}
   rel={isExternal ? 'noopener' : undefined}
   class="library-row gap-5 px-5 py-5 no-underline text-{p.id}">
  <span class="kanji text-{p.id} text-center text-2xl" style="line-height:1;width:34px">{p.kanji}</span>
  <span class="lib-name">
    <span class="display text-ink block text-lg" style="letter-spacing:-0.01em">{p.name}</span>
    <span class="mono text-ink-faint text-xs">{p.lang}</span>
  </span>
  <span class="lib-desc">
    <span class="text-ink block text-sm">{p.tagline}</span>
    <span class="text-ink-mute text-xs">{p.category}</span>
  </span>
  <span class="mono rounded-sm px-2 py-0 {statusClass(p.status)} text-xs" style="white-space:nowrap">{p.status}</span>
  <span class="text-{p.id} text-sm">{isExternal ? '↗' : '→'}</span>
</a>

<style>
  .library-row {
    display: grid;
    grid-template-columns: auto 150px 1fr auto auto;
    align-items: center;
    border-top: 1px solid var(--paper-edge);
    transition: background 0.15s;
  }
  .library-row:first-child {
    border-top: none;
  }
  .library-row:hover {
    background: var(--paper-mute);
  }
  @media (max-width: 768px) {
    .library-row {
      grid-template-columns: auto 1fr auto;
    }
    .lib-desc {
      display: none;
    }
  }
</style>
