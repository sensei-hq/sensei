<script lang="ts">
  import type { Remedy } from '$lib/health-types.js';
  import { Kanji } from '$lib/components';

  interface Props {
    remedy: Remedy;
    onCopyScript?: () => void;
    onVerify?: () => void;
  }
  let { remedy, onCopyScript, onVerify }: Props = $props();
</script>

<section class="mt-4 border border-primary-z5/30 rounded-xl bg-surface-z1 overflow-hidden">
  <header class="flex items-center gap-2.5 px-4 py-3.5 border-b border-surface-z2">
    <Kanji char="手" />
    <div class="flex-1">
      <div class="text-sm text-surface-z9">Run this in your terminal</div>
      <div class="text-xs text-surface-z7 mt-0.5">{remedy.message}</div>
    </div>
    {#if remedy.url}
      <a data-role="remedy-url" href={remedy.url} target="_blank" rel="noopener noreferrer"
         class="text-xs text-surface-z7 underline">Learn more</a>
    {/if}
  </header>

  <pre class="m-0 px-4 py-4 mono text-xs text-surface-z9 bg-surface-z3 leading-normal whitespace-pre-wrap break-words max-h-56 overflow-auto">{remedy.script}</pre>

  <footer class="flex items-center justify-between gap-2.5 px-4 py-3 border-t border-surface-z2">
    <button data-action="copy" class="btn-solid btn-sm" onclick={onCopyScript}>Copy script</button>
    <button data-action="verify"
            class="btn-outline btn-sm"
            style="color: oklch(var(--color-primary-z5) / 1); border-color: oklch(var(--color-primary-z5) / 0.4);"
            onclick={onVerify}>
      I've run it · verify
    </button>
  </footer>
</section>
