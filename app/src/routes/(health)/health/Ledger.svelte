<script lang="ts">
  import type { Component } from '$lib/health-types.js';
  import { COMPONENT_ORDER } from '$lib/health-types.js';

  interface Props { components: Component[]; }
  let { components }: Props = $props();

  /** Validates length on every prop change. Throws synchronously during render, which propagates through Svelte's mount() and is what the test expects. */
  const rows = $derived.by(() => {
    if (components.length !== COMPONENT_ORDER.length) {
      throw new Error(`Ledger: expected 5 components, got ${components.length}`);
    }
    return components;
  });

  const dotClass = (s: Component['status']): string => {
    switch (s) {
      case 'ready':      return 'bg-success-z5';
      case 'installing':
      case 'checking':   return 'bg-primary-z5';
      case 'failed':     return 'bg-ink-z4';
      case 'pending':    return 'bg-surface-z3';
    }
  };

  const badgeClass = (s: Component['status']): string => {
    switch (s) {
      case 'ready':      return 'text-success-z5';
      case 'installing':
      case 'checking':   return 'text-primary-z5';
      case 'failed':     return 'text-ink-z4';
      case 'pending':    return 'text-ink-z5';
    }
  };
</script>

<section class="mt-5.5">
  <div class="text-2xs tracking-tag uppercase text-ink-z5 mb-2.5">what this resolves</div>
  <ul class="flex flex-col">
    {#each rows as c (c.id)}
      <li data-row={c.id}
          class="grid grid-cols-[10px_1fr_auto] gap-3 items-center py-2 border-b border-surface-z2"
          style="opacity: {c.status === 'pending' ? 0.55 : 1}">
        <span class="w-2 h-2 rounded-full shrink-0 {dotClass(c.status)}"></span>
        <div>
          <span class="text-sm text-ink-z1">{c.label}</span>
          {#if c.note}<span class="text-xs text-ink-z5 ml-2">· {c.note}</span>{/if}
          {#if c.status === 'failed' && c.detail}
            <div data-detail class="text-2xs text-ink-z4 mt-0.5">{c.detail}</div>
          {/if}
        </div>
        <span data-badge class="mono text-2xs tracking-wider uppercase {badgeClass(c.status)}">
          {c.status}
        </span>
      </li>
    {/each}
  </ul>
</section>
