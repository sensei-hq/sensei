<script lang="ts">
  import type { Component } from '$lib/health-types.js';
  import { COMPONENT_ORDER } from '$lib/health-types.js';

  interface Props {
    /** Ordered list of gates to render. Length must be 6: the package
     *  manager (homebrew or winget) followed by the 5 components in
     *  COMPONENT_ORDER. The caller composes the array — Ledger does not
     *  pull from state. */
    components: Component[];
  }
  let { components }: Props = $props();

  // Length + order validation runs as a $derived so prop changes are
  // re-checked. Throwing here propagates through Svelte's mount() —
  // the spec expects a synchronous failure.
  const rows = $derived.by(() => {
    if (components.length !== 6) {
      throw new Error(`Ledger: expected 6 components, got ${components.length}`);
    }
    const pm = components[0];
    if (pm.id !== 'homebrew' && pm.id !== 'winget') {
      throw new Error(`Ledger: row 0 must be the package manager, got "${pm.id}"`);
    }
    for (let i = 0; i < COMPONENT_ORDER.length; i++) {
      if (components[i + 1].id !== COMPONENT_ORDER[i]) {
        throw new Error(`Ledger: row ${i + 1} must be "${COMPONENT_ORDER[i]}", got "${components[i + 1].id}"`);
      }
    }
    return components;
  });

  const isBusy = (s: Component['status']): boolean =>
    s === 'checking' || s === 'installing';

  const badgeClass = (s: Component['status']): string => {
    switch (s) {
      case 'ready':      return 'text-success-z5';
      case 'installing':
      case 'checking':   return 'text-primary-z5';
      case 'failed':     return 'text-surface-z6';
      case 'pending':    return 'text-surface-z6';
    }
  };

  const discBorderClass = (s: Component['status']): string => {
    switch (s) {
      case 'ready':      return 'border-success-z5';
      case 'installing':
      case 'checking':   return 'border-primary-z5';
      case 'failed':     return 'border-primary-z5';
      case 'pending':    return 'border-surface-z3';
    }
  };

  // Per-component verb for the `installing` status comes from the wire —
  // see `Component.installingVerb` populated by the Rust `DependencySpec`.
  const badgeText = (c: Component): string =>
    c.status === 'installing' ? c.installingVerb : c.status;
</script>

<ul class="flex flex-col">
  {#each rows as c (c.id)}
    <li data-row={c.id}
        class="grid grid-cols-[1fr_auto] gap-3 items-center py-2.5 border-b border-surface-z2"
        style="opacity: {c.status === 'pending' ? 0.55 : 1}">
      <div class="min-w-0">
        <div class="flex items-baseline gap-2 leading-tight">
          <span class="text-sm text-surface-z9">{c.label}</span>
          {#if c.version}<span data-version class="mono text-xs text-surface-z6">{c.version}</span>{/if}
          {#if c.note}<span class="text-xs text-surface-z5">· {c.note}</span>{/if}
        </div>
        <div data-description class="text-xs italic text-surface-z6 mt-0.5 leading-snug">
          {c.description}
        </div>
        {#if c.status === 'failed' && c.detail}
          <div data-detail class="text-xs text-surface-z6 mt-1 select-text">{c.detail}</div>
        {/if}
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <span data-badge class="mono text-xs tracking-wide uppercase {badgeClass(c.status)}">
          {badgeText(c)}
        </span>
        <span class="w-5 h-5 rounded-full border-[1.5px] bg-surface-z0 flex items-center justify-center {discBorderClass(c.status)}">
          {#if c.status === 'ready'}
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path d="M2 5.2 L4.2 7.2 L8 3" stroke="currentColor"
                    stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"
                    class="text-success-z5"/>
            </svg>
          {:else if isBusy(c.status)}
            <span data-spinner class="ledger-spinner"></span>
          {:else if c.status === 'failed'}
            <span class="text-primary-z5 text-[10px] leading-none">!</span>
          {/if}
        </span>
      </div>
    </li>
  {/each}
</ul>

<style>
  .ledger-spinner {
    display: block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1.5px solid oklch(var(--color-primary-z5) / 1);
    border-top-color: transparent;
    animation: ledger-spin 0.9s linear infinite;
  }
  @keyframes ledger-spin { to { transform: rotate(360deg); } }
</style>
