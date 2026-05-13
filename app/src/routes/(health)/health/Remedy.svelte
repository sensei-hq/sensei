<script lang="ts">
  import type { Remedy } from '$lib/health-types.js';

  interface Props {
    remedy: Remedy;
    onCopyScript?: () => void;
    onRecheck?: () => void;
  }
  let { remedy, onCopyScript, onRecheck }: Props = $props();
</script>

<section class="mt-4.5 border border-primary-z5/30 rounded-xl bg-surface-z1 overflow-hidden">
  <header class="flex items-center gap-2.5 px-4.5 py-3.5 border-b border-surface-z2">
    <span class="kanji text-base text-primary-z5">手</span>
    <div class="flex-1">
      <div class="text-sm text-ink-z1">Run this in your terminal</div>
      <div class="text-2xs text-ink-z5 mt-0.5">{remedy.message}</div>
    </div>
    {#if remedy.url}
      <a data-role="remedy-url" href={remedy.url} target="_blank" rel="noopener noreferrer"
         class="text-2xs text-ink-z5 underline">Learn more</a>
    {/if}
  </header>

  <pre class="m-0 px-4.5 py-4 mono text-xs text-ink-z1 bg-surface-z2 leading-relaxed whitespace-pre-wrap break-words max-h-56 overflow-auto">{remedy.script}</pre>

  <footer class="flex items-center justify-between gap-2.5 px-4.5 py-3 border-t border-surface-z2">
    <button data-action="copy" class="btn-solid btn-sm" onclick={onCopyScript}>Copy script</button>
    <button data-action="recheck"
            class="btn-outline btn-sm"
            style="color: oklch(var(--color-primary-z5) / 1); border-color: oklch(var(--color-primary-z5) / 0.4);"
            onclick={onRecheck}>
      I've run it · re-check
    </button>
  </footer>
</section>
