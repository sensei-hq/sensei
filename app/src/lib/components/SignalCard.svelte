<script lang="ts">
  import type { SignalVariant } from '$lib/types.js';

  interface Props {
    variant: SignalVariant;
    title: string;
    detail: string;
    /** Tool this signal is about — surfaced as an eyebrow label. */
    toolName?: string;
  }

  const { variant, title, detail, toolName }: Props = $props();

  // Each variant maps to a status colour family. Kept in one place so the
  // Health / Insights tabs render consistent cards regardless of caller.
  const VARIANT_STYLE: Record<SignalVariant, { bg: string; edge: string; ink: string; kanji: string }> = {
    warn:        { bg: 'bg-danger-soft',  edge: 'border-danger-edge',  ink: 'text-danger',  kanji: '警' },
    opportunity: { bg: 'bg-warning-soft', edge: 'border-warning-edge', ink: 'text-warning', kanji: '芽' },
    unused:      { bg: 'bg-paper-mute',   edge: 'border-paper-edge',   ink: 'text-ink-mute', kanji: '眠' },
    win:         { bg: 'bg-success-soft', edge: 'border-success-edge', ink: 'text-success', kanji: '勝' },
  };
  const style = $derived(VARIANT_STYLE[variant]);
</script>

<!-- Card body uses text-ink for the title so it stays readable on both
     light and dark modes; the tint colour is carried by the kanji glyph
     and the left border only (like the mockup's SignalCard which uses
     borderLeft: 3px solid <accent> rather than tinting the title text). -->
<article class="{style.bg} border rounded-lg p-4 flex items-start gap-3 border-l-2 {style.edge}">
  <span class="kanji text-xl {style.ink} w-6 text-center shrink-0">{style.kanji}</span>
  <div class="flex-1 min-w-0">
    {#if toolName}
      <div class="text-xs uppercase tracking-wide text-ink-mute font-mono">{toolName}</div>
    {/if}
    <div class="text-sm font-medium text-ink m-0">{title}</div>
    <p class="text-sm text-ink-soft m-0 mt-1 leading-normal">{detail}</p>
  </div>
</article>
