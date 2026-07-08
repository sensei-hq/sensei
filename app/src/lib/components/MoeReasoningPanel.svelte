<script lang="ts">
  // MOE reasoning panel — the analyzer's mixture-of-experts trace for a
  // measured recommendation: headline, body, consensus summary, per-model
  // notes, and (on a negative verdict) a suggested revision. Shared by the
  // observatory-wide Impact screen and the project-scoped Impact tab.
  import type { ImpactReasoning, VerdictTone } from '$lib/impact.js';

  let { reasoning, tone = 'ink' }: {
    reasoning: ImpactReasoning;
    tone?: VerdictTone;
  } = $props();

  const models = $derived(reasoning.models ?? []);
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
  <!-- Header row: 議 + eyebrow + consensus summary -->
  <div class="flex items-center gap-2 mb-2">
    <span class="kanji text-[13px] text-accent">議</span>
    <span class="text-xs uppercase tracking-wider text-ink-soft">MOE panel reasoning</span>
    <span class="flex-1"></span>
    {#if reasoning.consensus}
      <span class="font-mono text-xs text-ink-soft" data-testid="impact-moe-consensus">
        {reasoning.consensus}
      </span>
    {/if}
  </div>

  <!-- Headline — the sharpest single sentence -->
  {#if reasoning.headline}
    <div class="text-[13px] text-ink font-medium leading-snug mb-2" data-testid="impact-moe-headline">
      {reasoning.headline}
    </div>
  {/if}

  <!-- Body — a 2-3 sentence explanation -->
  {#if reasoning.body}
    <p class="text-[13px] text-ink-mute leading-relaxed m-0 mb-3" data-testid="impact-moe-body">
      {reasoning.body}
    </p>
  {/if}

  <!-- Per-model breakdown — name + role + note -->
  {#if models.length > 0}
    <div class="flex flex-col gap-1 pt-3 border-t border-paper-edge" data-testid="impact-moe-models">
      {#each models as m (m.name)}
        <div class="grid grid-cols-[120px_14px_1fr] gap-2 items-start" data-testid={`impact-moe-model-${m.name}`}>
          <span class="font-mono text-xs text-ink truncate">{m.name}</span>
          <span class="kanji text-[13px] text-accent mt-1">議</span>
          <div>
            <span class="text-xs uppercase tracking-wider text-ink-soft">{m.role}</span>
            <p class="text-xs text-ink-mute leading-relaxed m-0 mt-0.5">{m.note}</p>
          </div>
        </div>
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
