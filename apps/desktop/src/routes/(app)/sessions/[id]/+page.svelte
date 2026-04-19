<script lang="ts">
  import { page } from '$app/stores';
  import { sessionDetailDummy } from '$lib/observatory/dummy.js';
  import type { SessionDetail, SessionEvent } from '$lib/observatory/types.js';

  // TODO: swap for real API call
  let detail: SessionDetail = sessionDetailDummy($page.params.id ?? 's-001');

  function eventIcon(type: string): { icon: string; cls: string } {
    switch (type) {
      case 'turn': return { icon: '💬', cls: 'bg-surface-z3' };
      case 'tool_used': return { icon: '🔧', cls: 'bg-info-z2' };
      case 'revision_requested': return { icon: '⚠', cls: 'bg-warning-z2' };
      case 'mindset_applied': return { icon: '🧠', cls: 'bg-primary-z2' };
      case 'persona_applied': return { icon: '👤', cls: 'bg-accent-z2' };
      case 'rule_checked': return { icon: '📋', cls: 'bg-success-z2' };
      default: return { icon: '·', cls: 'bg-surface-z3' };
    }
  }

  function eventLabel(event: SessionEvent): string {
    if (event.type === 'turn') return event.classification ?? 'turn';
    if (event.type === 'tool_used') return event.toolName ?? 'tool';
    if (event.type === 'revision_requested') return 'correction';
    if (event.type === 'mindset_applied') return `${event.data.mindset ?? 'mindset'} applied`;
    if (event.type === 'persona_applied') return `${event.data.persona ?? 'persona'} applied`;
    if (event.type === 'rule_checked') return `${event.data.rule ?? 'rule'} checked`;
    return event.type;
  }

  function metricDisplay(value: number | null, quality: string): string {
    if (quality === 'unavailable') return '—';
    if (value === null) return '—';
    return String(value);
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <!-- Header -->
  <div>
    <a href="/sessions" class="text-[10px] text-primary-z5 hover:text-primary-z6">&larr; Sessions</a>
    <h2 class="mt-1 text-lg font-semibold text-surface-z8">{detail.task}</h2>
    <p class="text-xs text-surface-z4">{detail.project} &middot; {detail.startedAt.slice(0, 10)}</p>
  </div>

  <!-- Summary metrics -->
  <div class="grid grid-cols-5 gap-3">
    {#each [
      { label: 'FTR', value: detail.ftr !== null ? `${Math.round(detail.ftr * 100)}%` : '—' },
      { label: 'Turns', value: String(detail.turns) },
      { label: 'Corrections', value: String(detail.corrections) },
      { label: 'Tokens', value: detail.tokens.quality !== 'unavailable' ? `${Math.round((detail.tokens.value as number) / 1000)}k` : '—' },
      { label: 'Cost', value: detail.cost.quality !== 'unavailable' ? `$${(detail.cost.value as number).toFixed(2)}` : '—' },
    ] as stat}
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z5 uppercase tracking-wide font-medium">{stat.label}</p>
        <p class="mt-1 text-xl font-semibold {stat.value === '—' ? 'text-surface-z3' : 'text-primary-z6'}">{stat.value}</p>
      </div>
    {/each}
  </div>

  <div class="grid grid-cols-[1fr_300px] gap-6">

    <!-- Event timeline -->
    <div class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Timeline</p>
      <div class="space-y-1">
        {#each detail.events as event (event.id)}
          {@const ei = eventIcon(event.type)}
          <div class="flex items-start gap-3 rounded-lg bg-surface-z2 px-3 py-2 text-sm {event.type === 'revision_requested' ? 'border border-warning-z3/50' : ''}">
            <span class="mt-0.5 flex h-6 w-6 items-center justify-center rounded-md text-xs {ei.cls}">{ei.icon}</span>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-medium text-surface-z7">{eventLabel(event)}</span>
                {#if event.isMcp === true}
                  <span class="rounded bg-primary-z2 px-1 py-0.5 text-[8px] font-medium text-primary-z7">MCP</span>
                {:else if event.isMcp === false}
                  <span class="rounded bg-warning-z2 px-1 py-0.5 text-[8px] font-medium text-warning-z7">fallback</span>
                {/if}
                <span class="text-[10px] text-surface-z3 ml-auto shrink-0">{event.timestamp.slice(11, 19)}</span>
              </div>
              {#if event.toolParams}
                <p class="text-[10px] text-surface-z4 mt-0.5 font-mono truncate">{JSON.stringify(event.toolParams)}</p>
              {/if}
              {#if event.toolResponse}
                {#if event.toolResponse.quality === 'exact'}
                  <p class="text-[10px] text-surface-z5 mt-0.5 truncate">→ {event.toolResponse.value}</p>
                {:else if event.toolResponse.quality === 'unavailable'}
                  <p class="text-[10px] text-surface-z3 mt-0.5 italic">{event.toolResponse.hint ?? 'Response not captured'}</p>
                {/if}
              {/if}
              {#if event.type === 'revision_requested'}
                <p class="text-[10px] text-warning-z6 mt-0.5">{event.data.reason ?? 'User requested correction'}</p>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>

    <!-- Right sidebar: profiles + rules -->
    <div class="space-y-5">

      <!-- Profiles applied -->
      <div class="space-y-2">
        <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Profiles</p>
        {#each detail.profilesApplied as p}
          <div class="flex items-center gap-2 rounded-lg bg-surface-z2 px-3 py-2 text-sm">
            <span class="h-1.5 w-1.5 rounded-full {p.applied ? 'bg-success-z5' : 'bg-surface-z3'}"></span>
            <span class="flex-1 text-surface-z7">{p.name}</span>
            <span class="text-[10px] {p.applied ? 'text-success-z6' : 'text-surface-z4'}">
              {p.applied ? p.category : 'not applied'}
            </span>
          </div>
        {/each}
      </div>

      <!-- Rules checked -->
      <div class="space-y-2">
        <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Rules</p>
        {#each detail.rulesChecked as rule}
          <div class="flex items-center gap-2 rounded-lg bg-surface-z2 px-3 py-2 text-sm">
            <span class="h-1.5 w-1.5 rounded-full {rule.adhered ? 'bg-success-z5' : 'bg-warning-z5'}"></span>
            <span class="flex-1 text-surface-z7">{rule.rule}</span>
            {#if !rule.adhered && rule.detail}
              <span class="text-[10px] text-warning-z6 truncate max-w-32">{rule.detail}</span>
            {/if}
          </div>
        {/each}
      </div>

    </div>
  </div>

</div>
