<script lang="ts">
  import type { Remedy } from '$lib/health-types.js';
  import { Kanji } from '$lib/components';

  interface Props {
    remedy: Remedy;
    /** Optional override for clipboard write. Tests inject a mock; production
     *  defaults to `navigator.clipboard.writeText`. Returns void on success,
     *  throws on failure — we use try/catch to drive the button feedback. */
    writeText?: (text: string) => Promise<void>;
    onVerify?: () => void;
  }
  let { remedy, writeText, onVerify }: Props = $props();

  type CopyState = 'idle' | 'copied' | 'failed';
  let copyState = $state<CopyState>('idle');

  async function handleCopy() {
    // Pre-resolved override (tests) → call it directly. Otherwise reach for
    // the browser/webview clipboard API. The `?.` is intentional — in a
    // restricted webview the property may not exist; treat that as failure
    // rather than letting the click silently no-op.
    const write = writeText ?? (
      typeof navigator !== 'undefined' && navigator.clipboard
        ? navigator.clipboard.writeText.bind(navigator.clipboard)
        : null
    );

    if (!write) {
      copyState = 'failed';
      setTimeout(() => { copyState = 'idle'; }, 2400);
      return;
    }

    try {
      await write(remedy.script);
      copyState = 'copied';
    } catch {
      copyState = 'failed';
    }
    setTimeout(() => { copyState = 'idle'; }, 2400);
  }

  const copyLabel = $derived(
    copyState === 'copied' ? 'Copied ✓'
    : copyState === 'failed' ? 'Copy failed — select below'
    : 'Copy script',
  );
</script>

<section class="mt-4 border border-accent/30 rounded-xl bg-paper-soft">
  <header class="flex items-center gap-2.5 px-4 py-3.5 border-b border-paper-edge">
    <Kanji char="手" />
    <div class="flex-1">
      <div class="text-sm text-ink">Run this in your terminal</div>
      <div data-remedy-message class="text-xs text-ink-mute mt-0.5 select-text">{remedy.message}</div>
    </div>
    {#if remedy.url}
      <a data-role="remedy-url" href={remedy.url} target="_blank" rel="noopener noreferrer"
         class="text-xs text-ink-mute underline">Learn more</a>
    {/if}
  </header>

  <pre class="m-0 px-4 py-4 mono text-xs text-ink bg-paper-mute leading-normal whitespace-pre-wrap break-words max-h-56 overflow-auto select-text cursor-text">{remedy.script}</pre>

  <footer class="flex items-center justify-between gap-2.5 px-4 py-3 border-t border-paper-edge">
    <button data-action="copy"
            data-state={copyState}
            class="btn-solid btn-sm"
            onclick={handleCopy}>
      {copyLabel}
    </button>
    <button data-action="verify"
            class="btn-outline btn-sm text-accent border-accent/40"
            onclick={onVerify}>
      I've run it · re-check
    </button>
  </footer>
</section>
