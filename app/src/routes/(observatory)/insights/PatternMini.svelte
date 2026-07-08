<script lang="ts">
  import type { PatternCardVm } from './insights-board.svelte.js';

  let { pattern }: { pattern: PatternCardVm } = $props();

  // 'rule' patterns are promoted (settled, success-toned); 'suggested' are
  // still candidates (soon, warning-toned).
  const isRule = $derived(pattern.lifecycle === 'rule');
  const caption = $derived(
    isRule
      ? `${pattern.instanceCount} places · adopted rule`
      : `${pattern.instanceCount} places · candidate to promote`,
  );
</script>

<!-- A detected pattern — a code shape sensei keeps seeing but hasn't ruled on. -->
<article
  data-component="pattern-mini"
  data-pattern-id={pattern.id}
  data-lifecycle={pattern.lifecycle}
  class="rounded bg-paper-soft border border-paper-edge border-l-2 py-2 px-3"
  class:border-l-warning={!isRule}
  class:border-l-success={isRule}
>
  <div class="flex items-center gap-2 mb-1">
    <span class="font-mono text-sm text-ink truncate">{pattern.name}</span>
    <span class="flex-1"></span>
    {#if pattern.family}
      <span class="text-xs uppercase tracking-wider text-ink-faint">{pattern.family}</span>
    {/if}
  </div>
  <p class="text-xs text-ink-soft m-0 leading-normal">{caption}</p>
</article>
