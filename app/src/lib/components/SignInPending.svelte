<script lang="ts">
  import { Button } from '@rokkit/ui';

  // #300: every sign-in opens the system browser. If that browser is signed in
  // to GitHub as someone else, the daemon refuses the callback rather than store
  // the wrong account — so say which account to use, and offer the link for a
  // private window. Shared by the sign-in overlay and Settings → Dōjō.
  let {
    pending,
    copy = (text: string) => navigator.clipboard.writeText(text),
  }: {
    pending: { label: string; expectedLogin: string | null; url: string } | null;
    copy?: (text: string) => Promise<void>;
  } = $props();

  let copied = $state(false);
  let copyError = $state<string | null>(null);

  async function copyLink() {
    if (!pending) return;
    copyError = null;
    try {
      await copy(pending.url);
      copied = true;
    } catch (e) {
      copyError = e instanceof Error ? e.message : 'the link could not be copied';
    }
  }
</script>

{#if pending}
  <div data-component="sign-in-pending" class="flex flex-col gap-2 rounded-md border border-paper-edge bg-paper-soft px-3 py-2 text-xs text-ink-soft">
    <p class="m-0">
      {#if pending.expectedLogin}
        Your browser is open for <span class="text-ink">{pending.label}</span>: sign in to GitHub as
        <span class="font-mono text-ink">{pending.expectedLogin}</span>.
      {:else}
        Your browser is open: sign in to GitHub with the account that belongs to
        <span class="text-ink">{pending.label}</span>.
      {/if}
      If your browser is already signed in to GitHub as someone else, copy the link into a private window.
    </p>
    <div class="flex items-center gap-2">
      <Button size="sm" style="outline" onclick={copyLink} data-testid="sign-in-copy-link">
        {copied ? 'Copied' : 'Copy link'}
      </Button>
      {#if copyError}<span class="text-danger">{copyError}</span>{/if}
    </div>
  </div>
{/if}
