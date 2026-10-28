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
  // PRESENTATIONAL. Every identity's sentence, tone, button and busy state is
  // decided by PersonaList (personas.svelte.ts) — the same state Settings · Dōjō
  // renders — so the two cannot disagree. This renders it and reports clicks.
  import { IDENTITY_TONE_CLASS, type IdentitiesPhase, type Identity } from '$lib/personas.svelte.js';
  import ModalShell from './ModalShell.svelte';
  import SignInPending from './SignInPending.svelte';

  interface Props {
    open: boolean;
    identities: Identity[];
    /** The state's phase: loading, error, empty or list. */
    phase: IdentitiesPhase;
    /** Why a read or an attempt failed, when one did. */
    error?: string | null;
    onAct?: (identity: Identity) => void;
    onClose?: () => void;
    /** The sign-in in flight (#300): which account to use, and its link. */
    pending?: { label: string; expectedLogin: string | null; url: string } | null;
  }
  let { open, identities, phase, error = null, onAct, onClose, pending = null }: Props = $props();

</script>

<ModalShell {open} component="sign-in-overlay" labelledBy="sign-in-overlay-title" {onClose}>
      <header class="flex flex-col gap-1">
        <h2 id="sign-in-overlay-title" class="font-heading text-base text-ink">Identities</h2>
        <p class="text-xs text-ink-mute">
          Each identity holds its own GitHub credential, valid for about eight hours.
          sensei renews them on its own; sign in when it cannot.
        </p>
      </header>

      {#if phase === 'error'}
        <p role="alert" class="text-xs text-danger">{error}</p>
      {:else if phase === 'loading'}
        <p data-component="sign-in-loading" class="text-xs text-ink-mute">
          Reading identities…
        </p>
      {:else if phase === 'empty'}
        <p class="text-xs text-ink-mute">
          No identities yet. They appear once sensei has scanned a repository and
          seen who authored the commits.
        </p>
      {:else}
        <ul class="m-0 flex list-none flex-col gap-2 p-0">
          {#each identities as p (p.label)}
            <li
              data-persona={p.label}
              data-tone={p.tone}
              class="flex items-center justify-between gap-3 rounded-md border border-paper-edge px-3 py-2"
            >
              <div class="flex min-w-0 flex-col">
                <span class="truncate text-sm text-ink">{p.name}</span>
                <span class="text-xs {IDENTITY_TONE_CLASS[p.tone]}">{p.standing}</span>
              </div>

              {#if p.badge}
                <!-- Nothing to do. A button here would invite a needless
                     sign-in, which rotates a working credential for no reason. -->
                <span class="whitespace-nowrap text-xs text-success">{p.badge}</span>
              {:else if p.action}
                <button
                  type="button"
                  class="whitespace-nowrap rounded-md border border-paper-edge px-2 py-1 text-xs text-ink hover:bg-paper-mute disabled:opacity-50"
                  disabled={p.busy}
                  onclick={() => onAct?.(p)}
                >
                  {p.buttonLabel}
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <SignInPending {pending} />

      <footer class="flex items-center justify-between gap-3">
        <!-- Sign-in happens in the system browser (#300); a browser signed in as
             another GitHub account is caught by the daemon's account guard. -->
        <span class="text-xs text-ink-faint">Sign-in opens your browser.</span>
        <button
          type="button"
          class="rounded-md border border-paper-edge px-2 py-1 text-xs text-ink hover:bg-paper-mute"
          onclick={() => onClose?.()}
        >
          Close
        </button>
      </footer>
</ModalShell>
