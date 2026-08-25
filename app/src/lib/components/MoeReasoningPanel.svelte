<script lang="ts">
  // Impact reasoning panel — the analyzer's single FTR-delta verdict for a
  // measured recommendation: headline, body, the real models that ran, and (on
  // a negative verdict) a suggested revision. There is ONE verdict, not an
  // N-model vote, so this shows no fabricated consensus tally or per-model
  // roles/notes (the #109 fabrication audit). Shared by the observatory-wide
  // Impact screen and the project-scoped Impact tab.
  import type { ImpactReasoning, VerdictTone } from '$lib/impact.js';

  let { reasoning, tone = 'ink' }: {
    reasoning: ImpactReasoning;
    tone?: VerdictTone;
  } = $props();

  const modelsUsed = $derived(reasoning.modelsUsed ?? []);
</script>

<div
  class={[
    'py-4 px-5 rounded bg-paper-mute border border-paper-edge border-l-2',
    {
      'border-l-success': tone === 'success',
      'border-l-warning': tone === 'warning',
      'border-l-ink-mute': tone === 'ink',
    },
  ]}
  data-testid="impact-moe-panel"
>
  <!-- Header row: 議 + eyebrow -->
  <div class="flex items-center gap-2 mb-2">
    <span class="kanji text-sm text-accent">議</span>
    <span class="text-xs uppercase tracking-wider text-ink-soft">impact reasoning</span>
  </div>

  <!-- Headline — the sharpest single sentence -->
  {#if reasoning.headline}
    <div class="text-sm text-ink font-medium leading-snug mb-2" data-testid="impact-moe-headline">
      {reasoning.headline}
    </div>
  {/if}

  <!-- Body — a 2-3 sentence explanation -->
  {#if reasoning.body}
    <p class="text-sm text-ink-mute leading-relaxed m-0 mb-3" data-testid="impact-moe-body">
      {reasoning.body}
    </p>
  {/if}

  <!-- Models used — the real models that ran in the measured sessions. Just
       the names: there is one verdict, so no per-model role/note breakdown. -->
  {#if modelsUsed.length > 0}
    <div class="flex flex-wrap items-center gap-2 pt-3 border-t border-paper-edge" data-testid="impact-moe-models">
      <span class="text-xs uppercase tracking-wider text-ink-soft">models used</span>
      {#each modelsUsed as name (name)}
        <span class="font-mono text-xs text-ink px-1.5 py-0.5 rounded bg-paper border border-paper-edge" data-testid={`impact-moe-model-${name}`}>
          {name}
        </span>
      {/each}
    </div>
  {/if}

  <!-- Suggested revision — surfaced only on a negative verdict -->
  {#if reasoning.suggestedRevision}
    <div class="mt-3 py-2 px-3 rounded bg-paper border border-paper-edge" data-testid="impact-moe-revision">
      <div class="text-xs uppercase tracking-wider text-accent mb-1">Suggested revision</div>
      <p class="text-xs text-ink-mute leading-relaxed m-0">{reasoning.suggestedRevision}</p>
    </div>
  {/if}
</div>
