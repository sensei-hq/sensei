<script lang="ts">
  import { PageHeader, TurnBar } from '$lib/components';
  import type { ProjectSession } from '$lib/types.js';

  // Same vocabulary as the daily observatory Recent Sessions — one word
  // for "did this go right the first time" so both surfaces read the same.
  function reworkLabel(c: number): string {
    if (c === 0) return 'first-try';
    return `${c}× rework`;
  }

  let { data } = $props();

  // Local filter — All / FTR-pass / FTR-fail — keeps the daemon
  // round-trip out of the hot path. `outcome` is always shown regardless.
  let filter = $state<'all' | 'pass' | 'fail'>('all');

  const sessions: ProjectSession[] = $derived(data.sessions);
  const visible = $derived(
    filter === 'all'
      ? sessions
      : sessions.filter(s => (filter === 'pass' ? s.ftr === true : s.ftr === false)),
  );

  // Aggregates for the header — total sessions, pass count, average turns
  // + corrections give a one-glance quality read.
  const total = $derived(sessions.length);
  const passCount = $derived(sessions.filter(s => s.ftr === true).length);
  const failCount = $derived(sessions.filter(s => s.ftr === false).length);
  const avgTurns = $derived(
    total === 0 ? 0 : Math.round(sessions.reduce((a, s) => a + (s.turns || 0), 0) / total),
  );
  const totalCorrections = $derived(sessions.reduce((a, s) => a + (s.corrections || 0), 0));

  function fmtDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }
  function fmtDuration(started: string, completed: string | null): string {
    if (!completed) return '—';
    const ms = new Date(completed).getTime() - new Date(started).getTime();
    if (ms < 60_000) return `${Math.round(ms / 1000)}s`;
    if (ms < 3_600_000) return `${Math.round(ms / 60_000)}m`;
    return `${(ms / 3_600_000).toFixed(1)}h`;
  }
  function shortModel(model: string | null): string {
    if (!model) return '—';
    // Common model names — abbreviate for a tight column.
    return model
      .replace(/^claude-/, '')
      .replace(/^gpt-/, 'gpt-')
      .replace(/-\d{8}$/, '');
  }
</script>

<PageHeader title="Sessions">
  {#snippet right()}
    <div class="text-sm text-ink-mute flex gap-3">
      <span>{total} total</span>
      <span class="text-success">✓ {passCount}</span>
      <span class="text-danger">✗ {failCount}</span>
      <span class="opacity-70">avg {avgTurns} turns · {totalCorrections} corrections</span>
    </div>
  {/snippet}
</PageHeader>

<div class="px-6 py-6">
  <div class="flex gap-2 mb-4" role="tablist" aria-label="FTR filter">
    {#each [['all','All'],['pass','FTR ✓'],['fail','FTR ✗']] as [id, label]}
      {@const active = filter === id}
      <button
        type="button"
        class="px-3 py-1 rounded-full border text-xs cursor-pointer transition-colors duration-fast"
        class:bg-primary={active}
        class:text-on-primary={active}
        class:border-primary={active}
        class:bg-transparent={!active}
        class:text-ink-soft={!active}
        class:border-paper-mute={!active}
        role="tab"
        aria-selected={active}
        data-testid={`sessions-filter-${id}`}
        onclick={() => (filter = id as 'all' | 'pass' | 'fail')}
      >{label}</button>
    {/each}
  </div>

  {#if visible.length === 0}
    <p class="text-sm text-ink-soft">
      {filter === 'all' ? 'No sessions recorded for this project yet.' : 'No sessions match this filter.'}
    </p>
  {:else}
    <div class="grid grid-cols-[60px_1fr_100px_80px_100px_60px_80px] gap-3 px-3 py-2 text-xs text-ink-mute tracking-wide uppercase">
      <span>Date</span>
      <span>Task</span>
      <span>Model</span>
      <span>Timeline</span>
      <span class="text-right">Rework</span>
      <span class="text-right">FTR</span>
      <span class="text-right">Outcome</span>
    </div>
    {#each visible as s (s.id)}
      <div class="session-row grid grid-cols-[60px_1fr_100px_80px_100px_60px_80px] gap-3 px-3 py-2 border-b border-paper-mute text-sm items-center" data-testid={`session-row-${s.id}`}>
        <span class="text-xs text-ink-soft font-mono">{fmtDate(s.startedAt)}</span>
        <div class="min-w-0">
          <div class="truncate">{s.task}</div>
          {#if s.completedAt}
            <div class="text-xs text-ink-soft opacity-70">{fmtDuration(s.startedAt, s.completedAt)}</div>
          {/if}
        </div>
        <span class="text-xs font-mono text-ink-soft truncate" title={s.model ?? undefined}>
          {shortModel(s.model)}
        </span>
        <div class="flex items-center gap-2" title={`${s.turns} turns · ${s.corrections} rework`}>
          <TurnBar turns={s.turns} corrections={s.corrections} width={60} height={8} />
          <span class="font-mono text-xs text-ink-soft">{s.turns}</span>
        </div>
        <span class="text-right font-mono text-xs"
              class:text-success={s.corrections === 0}
              class:text-warning={s.corrections > 0 && s.corrections < 3}
              class:text-danger={s.corrections >= 3}>
          {reworkLabel(s.corrections)}
        </span>
        <span class="text-right font-mono text-sm"
              class:text-success={s.ftr === true}
              class:text-danger={s.ftr === false}>
          {s.ftr === true ? '✓' : s.ftr === false ? '✗' : '—'}
        </span>
        <span class="text-right text-xs text-ink-soft truncate">{s.outcome ?? '—'}</span>
      </div>
    {/each}
  {/if}
</div>

<style>
  .session-row:last-child {
    border-bottom: none;
  }
  .session-row:hover {
    background: var(--paper-mute);
  }
</style>
