<script lang="ts">
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { SessionData } from '$lib/types.js';

  type Session = SessionData['sessions'][number];

  let sessions = $state<Session[]>([]);
  let stats = $state<{ count: number; ftr: number; corrections: number; projects: number }>({
    count: 0, ftr: 0, corrections: 0, projects: 0,
  });
  let loading = $state(true);
  let filter = $state<'all' | 'completed' | 'corrected' | 'abandoned'>('all');

  onMount(async () => {
    await appState.load();
    const api = senseiApi(appState.port);
    const data = await api.getSessions();
    sessions = data.sessions ?? [];
    if (data.stats) {
      const s = data.stats as Record<string, number>;
      stats = {
        count: s.total_sessions ?? sessions.length,
        ftr: s.ftr_rate ?? 0,
        corrections: s.total_corrections ?? 0,
        projects: s.project_count ?? 0,
      };
    }
    loading = false;
  });

  let filtered = $derived(
    filter === 'all' ? sessions : sessions.filter(s => s.outcome === filter)
  );

  function formatTime(iso: string): string {
    if (!iso) return '';
    const d = new Date(iso);
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
      + ' · ' + d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  }
</script>

<div class="page">
  <header class="page-header">
    <p class="date-label">Sessions</p>
    <h1 class="display page-title">刻 Sessions</h1>
  </header>

  <!-- Stats strip -->
  <div class="stats-strip">
    <div class="stat">
      <span class="stat-value display">{stats.count}</span>
      <span class="stat-label">sessions (7d)</span>
    </div>
    <div class="stat">
      <span class="stat-value display">{Math.round(stats.ftr * 100)}%</span>
      <span class="stat-label">FTR</span>
    </div>
    <div class="stat">
      <span class="stat-value display">{stats.corrections}</span>
      <span class="stat-label">corrections</span>
    </div>
    <div class="stat">
      <span class="stat-value display">{stats.projects}</span>
      <span class="stat-label">projects</span>
    </div>
  </div>

  <!-- Filters -->
  <div class="filter-row">
    {#each ['all', 'completed', 'corrected', 'abandoned'] as f}
      <button
        class="filter-chip"
        class:active={filter === f}
        onclick={() => filter = f as any}
      >{f}</button>
    {/each}
  </div>

  <!-- Sessions list -->
  {#if loading}
    <p class="hint">Loading sessions...</p>
  {:else if filtered.length === 0}
    <div class="empty-state">
      <span class="kanji empty-kanji">刻</span>
      <p class="display empty-title">No sessions yet.</p>
      <p class="empty-body">Start a session with your assistant. Each session becomes a moment of learning.</p>
    </div>
  {:else}
    <div class="sessions-list">
      {#each filtered as session (session.id)}
        <div class="session-row">
          <span class="ftr-dot" class:green={session.ftr === 1} class:amber={session.ftr !== 1}></span>
          <div class="session-info">
            <span class="session-title">{session.task || session.id.slice(0, 8)}</span>
            <span class="session-project">{session.project || 'unknown'}</span>
          </div>
          <span class="session-outcome">{session.outcome ?? '—'}</span>
          <span class="session-time">{formatTime(session.startedAt)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page {
    max-width: 820px;
    margin: 0 auto;
    padding: 48px 48px 64px;
  }
  .page-header { margin-bottom: 32px; }
  .date-label {
    font-size: 10.5px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin: 0 0 8px;
  }
  .page-title {
    font-size: 24px;
    font-weight: 400;
    margin: 0;
  }

  /* ── Stats ──────────────────────────────────────────────── */
  .stats-strip {
    display: flex;
    gap: 32px;
    margin-bottom: 28px;
    padding: 20px 24px;
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius-lg);
  }
  .stat { display: flex; flex-direction: column; gap: 2px; }
  .stat-value { font-size: 22px; font-weight: 400; }
  .stat-label { font-size: 11px; color: var(--sumi-3); }

  /* ── Filters ────────────────────────────────────────────── */
  .filter-row {
    display: flex;
    gap: 6px;
    margin-bottom: 24px;
  }
  .filter-chip {
    padding: 5px 14px;
    border-radius: 100px;
    border: var(--border-card);
    background: transparent;
    color: var(--sumi-2);
    font-size: 12px;
    cursor: pointer;
    text-transform: capitalize;
  }
  .filter-chip:hover { background: var(--paper-2); }
  .filter-chip.active {
    background: var(--sumi);
    color: var(--paper);
    border-color: var(--sumi);
  }

  /* ── Empty state ────────────────────────────────────────── */
  .empty-state {
    text-align: center;
    padding: 80px 20px;
  }
  .empty-kanji {
    font-size: 64px;
    color: var(--shu);
    opacity: 0.3;
  }
  .empty-title { font-size: 20px; font-weight: 400; margin: 16px 0 8px; }
  .empty-body { font-size: 13px; color: var(--sumi-3); max-width: 360px; margin: 0 auto; line-height: 1.65; }

  /* ── Session rows ───────────────────────────────────────── */
  .sessions-list { display: flex; flex-direction: column; gap: 1px; }
  .session-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: var(--radius);
    transition: background 0.1s;
  }
  .session-row:hover { background: var(--paper-2); }
  .ftr-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .ftr-dot.green { background: var(--jade); }
  .ftr-dot.amber { background: var(--amber); }
  .session-info { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .session-title {
    font-size: 13px;
    color: var(--sumi);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .session-project { font-size: 11px; color: var(--sumi-3); }
  .session-outcome { font-size: 11px; color: var(--sumi-3); text-transform: capitalize; width: 80px; text-align: right; }
  .session-time { font-size: 11px; color: var(--sumi-4); width: 140px; text-align: right; }
</style>
