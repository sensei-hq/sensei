<script lang="ts">
  // The one modal shell: `--scrim` backdrop, Escape and click-outside to close,
  // focus moved in on open, kept in while open, and given back on close.
  // Extracted from SignInOverlay and the logs report modal when a third modal
  // (the tracker disconnect dialog) appeared, so the keyboard contract lives
  // and is tested in one place.
  //
  // Focus in is load-bearing: Escape is handled on the container, so without it
  // the key goes to whatever had focus behind the overlay and the dialog cannot
  // be dismissed by keyboard.
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    /** `data-component` on the dialog element, for tests and e2e selectors. */
    component: string;
    /** Id of the element that names the dialog. */
    labelledBy?: string;
    onClose?: () => void;
    /** Classes for the card inside the scrim (size, padding, layout). */
    cardClass?: string;
    children: Snippet;
  }
  let {
    open,
    component,
    labelledBy,
    onClose,
    cardClass = 'flex w-full max-w-md flex-col gap-4 rounded-xl border border-paper-edge bg-paper-soft p-6',
    children,
  }: Props = $props();

  const FOCUSABLE =
    'button:not([disabled]), a[href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

  let card = $state<HTMLElement | null>(null);

  // Focus in on open; the cleanup gives it back on close, whether the shell
  // is closed or unmounted with its caller's `{#if}`.
  $effect(() => {
    if (!open || !card) return;
    const restoreTo = document.activeElement as HTMLElement | null;
    // The first button if there is one, so a keyboard user can act at once;
    // otherwise the card itself, which still receives Escape.
    const first = card.querySelector<HTMLElement>('button:not([disabled])');
    (first ?? card).focus();
    return () => {
      // Back where it was, rather than dropped on <body>.
      if (restoreTo?.isConnected) restoreTo.focus();
    };
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose?.();
      return;
    }
    if (e.key !== 'Tab' || !card) return;
    const items = Array.from(card.querySelectorAll<HTMLElement>(FOCUSABLE));
    if (items.length === 0) {
      e.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    if (e.shiftKey && (active === first || !card.contains(active))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (active === last || !card.contains(active))) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

{#if open}
  <div
    data-component={component}
    class="modal-scrim fixed inset-0 z-30 flex items-center justify-center p-6"
    role="dialog"
    aria-modal="true"
    aria-labelledby={labelledBy}
    tabindex="-1"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose?.();
    }}
    {onkeydown}
  >
    <div bind:this={card} tabindex="-1" class="modal-card outline-none {cardClass}">
      {@render children()}
    </div>
  </div>
{/if}

<style>
  /* Scrim and shadow from tokens — no hardcoded rgba. */
  .modal-scrim {
    background: var(--scrim);
  }
  .modal-card {
    box-shadow: 0 24px 60px color-mix(in oklch, var(--shadow-tint) 40%, transparent);
  }
</style>
