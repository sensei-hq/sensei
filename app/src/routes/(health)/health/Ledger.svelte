<script lang="ts">
  import type { Component } from '$lib/health-types.js';
  import { COMPONENT_ORDER } from '$lib/health-types.js';
  import { Eyebrow, StatusDot } from '$lib/components';

  interface Props { components: Component[]; }
  let { components }: Props = $props();

  /** Validates length on every prop change. Throws synchronously during render, which propagates through Svelte's mount() and is what the test expects. */
  const rows = $derived.by(() => {
    if (components.length !== COMPONENT_ORDER.length) {
      throw new Error(`Ledger: expected 5 components, got ${components.length}`);
    }
    return components;
  });

  const dotStatus = (s: Component['status']): 'ok' | 'busy' | 'fail' | 'idle' => {
    switch (s) {
      case 'ready':      return 'ok';
      case 'installing':
      case 'checking':   return 'busy';
      case 'failed':     return 'fail';
      case 'pending':    return 'idle';
    }
  };

  const badgeClass = (s: Component['status']): string => {
    switch (s) {
      case 'ready':      return 'text-success-z5';
      case 'installing':
      case 'checking':   return 'text-primary-z5';
      case 'failed':     return 'text-surface-z6';
      case 'pending':    return 'text-surface-z6';
    }
  };
</script>

<section class="mt-6">
  <div class="mb-2.5"><Eyebrow>what this resolves</Eyebrow></div>
  <ul class="flex flex-col">
    {#each rows as c (c.id)}
      <li data-row={c.id}
          class="grid grid-cols-[10px_1fr_auto] gap-3 items-center py-2 border-b border-surface-z2"
          style="opacity: {c.status === 'pending' ? 0.55 : 1}">
        <StatusDot status={dotStatus(c.status)} />
        <div>
          <span class="text-sm text-surface-z9">{c.label}</span>
          {#if c.version}<span data-version class="mono text-xs text-surface-z6 ml-2">{c.version}</span>{/if}
          {#if c.note}<span class="text-xs text-surface-z5 ml-2">· {c.note}</span>{/if}
          {#if c.status === 'failed' && c.detail}
            <div data-detail class="text-xs text-surface-z6 mt-0.5">{c.detail}</div>
          {/if}
        </div>
        <span data-badge class="mono text-xs tracking-wide uppercase {badgeClass(c.status)}">
          {c.status}
        </span>
      </li>
    {/each}
  </ul>
</section>
