<script lang="ts">
  import { Eyebrow } from '$lib/components';
  import { insightToneClasses } from './today-view.svelte.js';
  import type { ObservatoryInsight } from '$lib/types.js';

  interface Props {
    insight: ObservatoryInsight;
    /** Where the card routes for the full triage view. */
    href?: string;
  }
  let { insight, href = '/insights' }: Props = $props();

  const tone = $derived(insightToneClasses(insight.tone));
</script>

<a
  {href}
  data-component="insight-card"
  data-tone={insight.tone}
  class="grid grid-cols-[auto_1fr_auto] items-start gap-3 py-3 px-4 border-b border-paper-edge text-left text-inherit hover:bg-paper-soft"
>
  <span class="kanji text-xl w-6 leading-none {tone.glyph}">{insight.kanji}</span>
  <div>
    <p class="m-0 mb-1"><Eyebrow>{insight.label}</Eyebrow></p>
    <p class="text-sm text-ink-mute leading-snug m-0">{insight.text}</p>
  </div>
  <span data-insight-tag class="font-mono text-xs px-2 py-1 rounded-sm whitespace-nowrap {tone.tag}">
    {insight.tag}
  </span>
</a>
