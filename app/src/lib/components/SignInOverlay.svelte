<script lang="ts">
  // The sign-in surface: every identity sensei knows, and what each one needs.
  //
  // An OVERLAY rather than a strip along the title bar. The strip was the first
  // shape and it read as odd placement — it competes with the window chrome, and
  // it has room for one message when the real subject is a LIST: an install has
  // several personas, each a separate GitHub identity with its own credential
  // and its own 8-hour expiry.
  //
  // Dismissible on purpose. An expired credential stops syncing; it does not
  // stop sensei, so blocking the app behind it would be out of proportion.
  //
  // The modal behaviour (scrim, Escape, click-outside, focus in, kept in and
  // given back) is ModalShell's.
  //
  // PRESENTATIONAL. The list, the per-row action and the busy state come from
  // personas.svelte.ts; this renders them and reports clicks upward.
  import type { PersonaWire } from '$lib/personas.svelte.js';
  import ModalShell from './ModalShell.svelte';
  import SignInPending from './SignInPending.svelte';

  interface Props {
    open: boolean;
    personas: PersonaWire[];
    /** Non-null when the read or an attempt FAILED. Rendered instead of a
     *  list — "no identities" and "we could not ask" are different answers. */
    error?: string | null;
    /** False until a read has succeeded. Distinguishes "none" from "not yet
     *  asked" — without it the overlay claims emptiness on open. */
    loaded?: boolean;
    /** Per-row, so one slow sign-in does not disable the others. */
    isBusy?: (p: PersonaWire) => boolean;
    actionLabel: (p: PersonaWire) => string;
    describe: (p: PersonaWire) => string;
    onSignIn?: (p: PersonaWire) => void;
    onClose?: () => void;
    /** The sign-in in flight (#300): which account to use, and its link. */
    pending?: { label: string; expectedLogin: string | null; url: string } | null;
  }
  let {
    open,
    personas,
    error = null,
    loaded = true,
    isBusy = () => false,
    actionLabel,
    describe,
    onSignIn,
    onClose,
    pending = null
  }: Props = $props();

  /** A row needs the user when it is not merely healthy. */
  const needsUser = (p: PersonaWire) => p.action === 'signIn' || p.action === 'connect';

</script>

<ModalShell {open} component="sign-in-overlay" labelledBy="sign-in-overlay-title" {onClose}>
      <header class="flex flex-col gap-1">
        <h2 id="sign-in-overlay-title" class="font-heading text-base text-ink">Identities</h2>
        <p class="text-xs text-ink-mute">
          Each identity holds its own GitHub credential, valid for about eight hours.
          sensei renews them on its own; sign in when it cannot.
        </p>
      </header>

      {#if error}
        <p role="alert" class="text-xs text-danger">{error}</p>
      {:else if !loaded}
        <p data-component="sign-in-loading" class="text-xs text-ink-mute">
          Reading identities…
        </p>
      {:else if personas.length === 0}
        <p class="text-xs text-ink-mute">
          No identities yet. They appear once sensei has scanned a repository and
          seen who authored the commits.
        </p>
      {:else}
        <ul class="m-0 flex list-none flex-col gap-2 p-0">
          {#each personas as p (p.label)}
            <li
              data-persona={p.label}
              class="flex items-center justify-between gap-3 rounded-md border border-paper-edge px-3 py-2"
            >
              <div class="flex min-w-0 flex-col">
                <span class="truncate text-sm text-ink">{p.githubLogin ?? p.label}</span>
                <span
                  class="text-xs"
                  class:text-warning={needsUser(p)}
                  class:text-ink-mute={!needsUser(p)}
                >
                  {describe(p)}
                </span>
              </div>

              {#if p.action === 'none'}
                <!-- Nothing to do. A button here would invite a needless
                     sign-in, which rotates a working credential for no reason. -->
                <span class="whitespace-nowrap text-xs text-success">ready</span>
              {:else}
                <button
                  type="button"
                  class="whitespace-nowrap rounded-md border border-paper-edge px-2 py-1 text-xs text-ink hover:bg-paper-mute disabled:opacity-50"
                  disabled={isBusy(p)}
                  onclick={() => onSignIn?.(p)}
                >
                  {isBusy(p) ? 'opening…' : actionLabel(p)}
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <SignInPending {pending} />

      <footer class="flex items-center justify-between gap-3">
        <!-- Stated because a fresh window is a deliberate, visible choice: the
             shared browser session would answer as whoever is already signed in
             to GitHub, silently connecting the wrong account. -->
        <span class="text-xs text-ink-faint">Opens a separate window per identity.</span>
        <button
          type="button"
          class="rounded-md border border-paper-edge px-2 py-1 text-xs text-ink hover:bg-paper-mute"
          onclick={() => onClose?.()}
        >
          Close
        </button>
      </footer>
</ModalShell>
