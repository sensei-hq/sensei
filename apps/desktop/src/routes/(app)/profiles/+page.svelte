<script lang="ts">
  import { profilesDummy } from '$lib/observatory/dummy.js';
  import type { ProfilesData, ProfileLever } from '$lib/observatory/types.js';

  let data: ProfilesData = profilesDummy();

  function verdictBadge(v: string) {
    switch (v) {
      case 'keep': return { cls: 'bg-success-z2 text-success-z7', label: 'keep' };
      case 'review': return { cls: 'bg-warning-z2 text-warning-z7', label: 'review' };
      case 'unused': return { cls: 'bg-surface-z3 text-surface-z5', label: 'unused' };
      case 'remove': return { cls: 'bg-error-z2 text-error-z7', label: 'remove' };
      default: return { cls: 'bg-surface-z3 text-surface-z5', label: v };
    }
  }

  function categoryIcon(c: string) {
    switch (c) {
      case 'mindset': return '🧠';
      case 'persona': return '👤';
      case 'rule': return '📋';
      case 'library': return '📚';
      default: return '·';
    }
  }

  function impactFmt(v: number, quality: string): string {
    if (quality === 'unavailable') return '—';
    const sign = v >= 0 ? '+' : '';
    return `${sign}${Math.round(v * 100)}%`;
  }

  function impactColor(v: number): string {
    if (v > 0.1) return 'text-success-z6';
    if (v > 0) return 'text-surface-z6';
    if (v === 0) return 'text-surface-z4';
    return 'text-error-z6';
  }

  // Copy action prompt to clipboard
  function copyPrompt(prompt: string) {
    navigator.clipboard.writeText(prompt);
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <div>
    <h2 class="text-lg font-semibold text-surface-z8">Profiles</h2>
    <p class="text-xs text-surface-z4">What's helping and what's not — ranked by impact on quality, time, and cost</p>
  </div>

  <!-- Lever impact table -->
  <div class="space-y-2">
    <div class="grid grid-cols-[24px_1fr_70px_60px_70px_70px_80px] gap-2 px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wide text-surface-z4">
      <span></span>
      <span>Lever</span>
      <span>Category</span>
      <span class="text-right">Applied</span>
      <span class="text-right">FTR</span>
      <span class="text-right">Tokens</span>
      <span class="text-center">Verdict</span>
    </div>

    {#each data.levers as lever (lever.name)}
      {@const vb = verdictBadge(lever.verdict)}
      <div class="grid grid-cols-[24px_1fr_70px_60px_70px_70px_80px] gap-2 items-center rounded-lg bg-surface-z2 px-3 py-2.5 text-sm">
        <span class="text-center">{categoryIcon(lever.category)}</span>
        <div class="min-w-0">
          <p class="font-medium text-surface-z7 truncate">{lever.name}</p>
          <p class="text-[10px] text-surface-z4 truncate">{lever.verdictReason}</p>
        </div>
        <span class="text-[10px] text-surface-z5">{lever.category}{lever.type ? ` · ${lever.type}` : ''}</span>
        <span class="text-right text-xs text-surface-z6">{lever.sessionsApplied}x</span>
        <span class="text-right text-xs font-medium {impactColor(lever.ftrImpact.value as number)}">
          {impactFmt(lever.ftrImpact.value as number, lever.ftrImpact.quality)}
        </span>
        <span class="text-right text-xs {(lever.tokenImpact.value as number) > 0 ? 'text-warning-z6' : (lever.tokenImpact.value as number) < 0 ? 'text-success-z6' : 'text-surface-z4'}">
          {impactFmt(lever.tokenImpact.value as number, lever.tokenImpact.quality)}
          {#if lever.tokenImpact.quality === 'estimated'}
            <span class="text-[8px]">~</span>
          {/if}
        </span>
        <span class="text-center rounded px-1.5 py-0.5 text-[10px] font-medium {vb.cls}">{vb.label}</span>
      </div>
    {/each}
  </div>

  <!-- Suggestions -->
  {#if data.suggestions.length > 0}
    <div class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Suggestions</p>
      {#each data.suggestions as suggestion}
        <div class="flex items-start gap-3 rounded-lg bg-surface-z2/50 border border-{suggestion.action.severity === 'warning' ? 'warning' : 'surface'}-z0/30 px-4 py-3">
          <span class="mt-0.5 text-base {suggestion.action.severity === 'warning' ? 'i-solar-danger-triangle-bold-duotone text-warning-z6' : 'i-solar-lightbulb-bold-duotone text-info-z6'}"></span>
          <div class="flex-1 min-w-0">
            <p class="text-sm text-surface-z7">{suggestion.reason}</p>
            <button
              onclick={() => copyPrompt(suggestion.action.prompt)}
              class="mt-1.5 rounded-md bg-primary-z2 px-2.5 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3"
            >
              {suggestion.action.label}
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

</div>
