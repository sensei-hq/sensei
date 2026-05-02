<script lang="ts">
  import type { PageData } from './$types';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    fmtMs, outcomeColor, anonymize, formatTime, timeOfDay,
    sessionSummary, groupByDate, moduleLabel,
    isBootstrapTrace, actionMeta, buildTitle, buildBody,
  } from './helpers.js';
  import { LogsPageState } from './state.svelte.js';

  let { data }: { data: PageData } = $props();

  const state = new LogsPageState();
  $effect(() => state.setSessions(data.sessions));

  const dateGroups = $derived(groupByDate(state.sessions));

  async function submitToGitHub() {
    if (!state.session) return;
    const url = `https://github.com/sensei-hq/app/issues/new?title=${encodeURIComponent(buildTitle(state.session))}&body=${encodeURIComponent(buildBody(state.session, state.addCtx))}`;
    await openUrl(url);
  }

  async function copyMarkdown() {
    if (!state.session) return;
    await navigator.clipboard.writeText(buildBody(state.session, state.addCtx));
    state.setCopied(true);
    setTimeout(() => { state.setCopied(false); }, 1400);
  }
</script>

<!-- ── Page shell ────────────────────────────────────────────────────────── -->
<div class="logs-page">

  <!-- ── Sidebar ─────────────────────────────────────────────────────────── -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <span class="kanji sidebar-kanji">診</span>
      <span class="sidebar-label">diagnostic logs</span>
    </div>

    <div class="sidebar-scroll">
      {#each dateGroups as dg (dg.date)}
        {@const open = state.expandedDates.has(dg.date)}

        <!-- Date header -->
        <button
          class="date-header"
          onclick={() => state.toggleDate(dg.date)}
        >
          <span class="date-chevron" class:collapsed={!open}>▾</span>
          <span class="date-label">{dg.date}</span>
        </button>

        {#if open}
          {#each dg.sessions as s (s.id)}
            {@const sel = s.id === state.selectedId}
            {@const isCurrent = s.id === state.sessions[0]?.id}
            <button
              class="session-row"
              class:selected={sel}
              onclick={() => state.selectSession(s.id)}
            >
              {#if isCurrent}
                <span class="current-badge">current</span>
              {/if}
              <div class="session-row-top">
                <span class="outcome-dot" style:background={outcomeColor(s.outcome)}></span>
                <span class="session-time">{timeOfDay(s.started_at)}</span>
              </div>
              <div class="session-meta">{moduleLabel(s.module)} · {sessionSummary(s)}</div>
            </button>
          {/each}
        {/if}
      {/each}

      {#if state.sessions.length === 0}
        <div class="sidebar-empty">No sessions yet</div>
      {/if}
    </div>
  </aside>

  <!-- ── Main panel ──────────────────────────────────────────────────────── -->
  <main class="main">
    {#if state.session}
      {@const s   = state.session}
      {@const si  = s.system_info}
      {@const bt  = s.traces.filter(isBootstrapTrace)}
      {@const fixes = bt.filter(t => t.fix_attempted).length}

      <!-- Session header -->
      <div class="session-header">
        <div class="header-top">
          <div class="header-info">
            <div class="module-label-text">{moduleLabel(s.module)}</div>
            <h2 class="display session-title">{formatTime(s.started_at)}</h2>
            <div class="sys-info-row">
              {#each [anonymize(si.os), si.arch, `${si.ram_gb} GB`, `${si.cpu_cores} cores`, `v${s.app_version}`] as v, i}
                {#if i > 0}<span class="sys-sep">·</span>{/if}
                <span class="sys-value">{v}</span>
              {/each}
            </div>
          </div>

          <button class="report-btn" onclick={() => state.openModal()}>
            Report this session ↗
          </button>
        </div>

        <!-- Stats row -->
        <div class="stats-row">
          {#each [
            { label: 'Total time', value: fmtMs(s.duration_ms) },
            { label: 'Traces',     value: String(bt.length) },
            { label: 'Auto-fixes', value: String(fixes), color: fixes > 0 ? 'var(--amber)' : undefined },
            { label: 'Outcome',    value: s.outcome, color: outcomeColor(s.outcome) },
          ] as stat (stat.label)}
            <div class="stat">
              <div class="stat-label">{stat.label}</div>
              <div class="stat-value" style:color={stat.color ?? 'var(--sumi)'}>{stat.value}</div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Column headers -->
      <div class="col-headers">
        <span>action</span>
        <span>step</span>
        <span>command</span>
        <span class="col-right">duration</span>
        <span></span>
      </div>

      <!-- Trace list -->
      <div class="trace-list">
        {#each bt as t (t.id)}
          {@const am       = actionMeta(t.action_type ?? 'check')}
          {@const isOpen   = state.expandedTraceId === t.id}
          {@const hasDetail = !!(t.out || t.err || t.fix_attempted)}

          <div class="trace-item">
            <!-- Main row -->
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div
              class="trace-row"
              class:clickable={hasDetail}
              role={hasDetail ? 'button' : 'presentation'}
              tabindex={hasDetail ? 0 : -1}
              onclick={() => { if (hasDetail) state.toggleTrace(t.id); }}
              onkeydown={(e) => { if (hasDetail && (e.key === 'Enter' || e.key === ' ')) state.toggleTrace(t.id); }}
            >
              <span class="action-badge" style:color={am.color} style:background={am.bg}>
                {am.label}
              </span>
              <span class="trace-step">{t.step ?? ''}</span>
              <span class="trace-cmd">{anonymize(t.cmd ?? '')}</span>
              <span class="trace-dur">{fmtMs(t.ms ?? 0)}</span>
              <div class="trace-status">
                {#if t.ok}
                  <span class="ok">✓</span>
                {:else if t.fix_ok}
                  <span class="fixed">FIXED</span>
                {:else}
                  <span class="fail">✗</span>
                {/if}
              </div>
            </div>

            <!-- Expanded detail -->
            {#if isOpen && hasDetail}
              <div class="trace-detail">
                {#if t.out}
                  <div class="detail-block">
                    <div class="detail-block-label">stdout</div>
                    <pre class="detail-pre">{anonymize(t.out)}</pre>
                  </div>
                {/if}
                {#if t.err}
                  <div class="detail-block">
                    <div class="detail-block-label err">stderr</div>
                    <pre class="detail-pre err-text">{t.err}</pre>
                  </div>
                {/if}
                {#if t.fix_attempted}
                  <div class="detail-block fix-block">
                    <div class="detail-block-label fix">auto-fix attempted</div>
                    <pre class="detail-pre" style:color={t.fix_ok ? 'var(--jade)' : 'var(--shu)'}>$ {t.fix_approach ?? ''}  <span class="fix-result">→ {t.fix_ok ? 'succeeded' : 'failed'}</span></pre>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
        <div class="trace-spacer"></div>
      </div>

    {:else}
      <div class="empty">No sessions recorded yet. Run the app to generate logs.</div>
    {/if}
  </main>
</div>

<!-- ── Report modal ──────────────────────────────────────────────────────── -->
{#if state.showModal && state.session}
  {@const s  = state.session}
  {@const si = s.system_info}
  <div
    class="modal-overlay"
    role="dialog"
    aria-modal="true"
    onclick={(e) => { if (e.target === e.currentTarget) state.closeModal(); }}
    onkeydown={(e) => { if (e.key === 'Escape') state.closeModal(); }}
    tabindex="-1"
  >
    <div class="modal">
      <div class="modal-header">
        <div>
          <div class="display modal-title">Report session</div>
          <div class="modal-sub">
            {formatTime(s.started_at)} · {s.traces.length} traces ·
            {s.traces.filter(t => isBootstrapTrace(t) && t.fix_attempted).length} auto-fixes
          </div>
        </div>
        <button class="modal-close" onclick={() => state.closeModal()}>×</button>
      </div>

      <div class="modal-body">
        <!-- Issue preview -->
        <div class="preview-col">
          <div class="col-label">Issue preview — anonymized</div>
          <div class="preview-title">{buildTitle(s)}</div>
          <pre class="preview-body">{buildBody(s, state.addCtx)}</pre>
        </div>

        <!-- Compose column -->
        <div class="compose-col">
          <div class="snapshot">
            <div class="snapshot-label">Included in report</div>
            {#each [
              ['Session',  formatTime(s.started_at)],
              ['OS',       anonymize(si.os)],
              ['Arch',     si.arch],
              ['RAM',      `${si.ram_gb} GB`],
              ['Traces',   String(s.traces.length)],
              ['Fixes',    String(s.traces.filter(t => isBootstrapTrace(t) && t.fix_attempted).length)],
              ['App',      `v${s.app_version}`],
            ] as [k, v]}
              <div class="snapshot-row">
                <span class="snapshot-key">{k}</span>
                <span class="snapshot-val">{v}</span>
              </div>
            {/each}
          </div>

          <div>
            <div class="col-label">Additional context</div>
            <textarea
              class="ctx-input"
              bind:value={state.addCtx}
              placeholder="What were you doing? Any other details…"
              rows={4}
            ></textarea>
          </div>

          <p class="privacy-note">
            Paths like <code>/Users/</code> are replaced with <code>~/</code>. No personal data included.
          </p>

          <div class="modal-actions">
            <button class="btn-solid action-btn" onclick={submitToGitHub}>
              Submit to GitHub ↗
            </button>
            <button class="outline-btn action-btn" onclick={copyMarkdown}>
              {state.copied ? 'Copied ✓' : 'Copy markdown'}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Shell ──────────────────────────────────────────────────────── */
  .logs-page {
    height: 100%;
    display: flex;
    overflow: hidden;
    font-family: var(--font-ui);
    color: var(--sumi);
  }

  /* ── Sidebar ────────────────────────────────────────────────────── */
  .sidebar {
    width: 248px;
    flex-shrink: 0;
    border-right: var(--hairline);
    background: var(--paper-2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .sidebar-header {
    padding: 22px 20px 14px;
    border-bottom: var(--hairline);
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .sidebar-kanji {
    font-size: 18px;
    color: var(--shu);
  }
  .sidebar-label {
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--sumi-3);
  }
  .sidebar-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }
  .sidebar-empty {
    padding: 16px 20px;
    font-size: 12px;
    color: var(--sumi-4);
  }

  /* Date headers */
  .date-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 7px 20px 5px;
    background: transparent;
    border: none;
    cursor: pointer;
    font-family: var(--font-ui);
    text-align: left;
  }
  .date-chevron {
    font-size: 9px;
    color: var(--sumi-4);
    line-height: 1;
    display: inline-block;
    transition: transform 0.15s;
  }
  .date-chevron.collapsed { transform: rotate(-90deg); }
  .date-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--sumi-2);
  }

  /* Session rows */
  .session-row {
    display: block;
    width: 100%;
    padding: 7px 20px 7px 36px;
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    cursor: pointer;
    font-family: var(--font-ui);
    text-align: left;
    transition: background 0.12s;
  }
  .session-row:hover { background: var(--paper-3); }
  .session-row.selected {
    background: var(--paper);
    border-left-color: var(--shu);
  }
  .current-badge {
    display: block;
    font-size: 9px;
    letter-spacing: 0.13em;
    text-transform: uppercase;
    color: var(--shu);
    margin-bottom: 2px;
  }
  .session-row-top {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 2px;
  }
  .outcome-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .session-time {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--sumi-2);
  }
  .session-row.selected .session-time { color: var(--sumi); }
  .session-meta {
    margin-left: 12px;
    font-size: 10.5px;
    color: var(--sumi-4);
  }

  /* ── Main panel ─────────────────────────────────────────────────── */
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Session header */
  .session-header {
    flex-shrink: 0;
    padding: 22px 36px 0;
    border-bottom: var(--hairline);
  }
  .header-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .module-label-text {
    font-size: 9.5px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--sumi-4);
    margin-bottom: 6px;
  }
  .session-title {
    font-size: 22px;
    font-weight: 400;
    margin: 0 0 6px;
    letter-spacing: -0.01em;
  }
  .sys-info-row {
    display: flex;
    gap: 0;
    align-items: center;
    flex-wrap: wrap;
    font-size: 12px;
    color: var(--sumi-4);
  }
  .sys-sep {
    margin: 0 8px;
    opacity: 0.4;
  }
  .report-btn {
    flex-shrink: 0;
    margin-top: 4px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--paper);
    background: var(--sumi);
    border: none;
    padding: 9px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .report-btn:hover { opacity: 0.85; }

  /* Stats row */
  .stats-row {
    display: flex;
    gap: 24px;
    padding-bottom: 16px;
  }
  .stat { display: flex; flex-direction: column; gap: 2px; }
  .stat-label {
    font-size: 9.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--sumi-4);
  }
  .stat-value {
    font-size: 14px;
    font-weight: 500;
  }

  /* Column headers */
  .col-headers {
    display: grid;
    grid-template-columns: 68px 148px 1fr 52px 40px;
    gap: 12px;
    padding: 7px 36px;
    border-bottom: var(--hairline);
    background: var(--paper-2);
    flex-shrink: 0;
    font-size: 9.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--sumi-4);
  }
  .col-right { text-align: right; }

  /* Trace list */
  .trace-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 36px;
  }
  .trace-spacer { height: 32px; }

  .trace-item { border-bottom: var(--hairline); }

  .trace-row {
    display: grid;
    grid-template-columns: 68px 148px 1fr 52px 40px;
    gap: 12px;
    align-items: center;
    padding: 9px 0;
    cursor: default;
  }
  .trace-row.clickable { cursor: pointer; }

  .action-badge {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.12em;
    padding: 2px 6px;
    border-radius: 3px;
    white-space: nowrap;
    justify-self: start;
  }
  .trace-step {
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--sumi-2);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .trace-cmd {
    font-size: 11.5px;
    font-family: var(--font-mono);
    color: var(--sumi-4);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .trace-dur {
    font-size: 11px;
    color: var(--sumi-4);
    font-feature-settings: "tnum";
    text-align: right;
  }
  .trace-status { text-align: center; }
  .ok   { color: var(--jade); font-size: 13px; line-height: 1; }
  .fail { color: var(--shu);  font-size: 13px; line-height: 1; }
  .fixed {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--amber);
  }

  /* Expanded detail */
  .trace-detail {
    margin-left: 80px;
    margin-bottom: 12px;
    padding: 12px 14px;
    background: var(--paper-2);
    border-radius: 5px;
    border: var(--hairline);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .detail-block { display: flex; flex-direction: column; gap: 4px; }
  .fix-block { padding-top: 10px; border-top: var(--hairline); }
  .detail-block-label {
    font-size: 9.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--sumi-4);
  }
  .detail-block-label.err { color: var(--shu); }
  .detail-block-label.fix { color: var(--amber); }
  .detail-pre {
    margin: 0;
    font-size: 11.5px;
    font-family: var(--font-mono);
    color: var(--sumi-2);
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .err-text { color: var(--shu) !important; }
  .fix-result { color: var(--sumi-4); font-size: 10px; }

  /* Empty state */
  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--sumi-4);
  }

  /* ── Modal ──────────────────────────────────────────────────────── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.30);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 30;
    padding: 24px;
  }
  .modal {
    width: 84vw;
    max-width: 960px;
    max-height: 90vh;
    background: var(--paper);
    border-radius: 12px;
    border: var(--hairline);
    display: flex;
    flex-direction: column;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.18);
  }
  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 16px 28px 12px;
    border-bottom: var(--hairline);
    flex-shrink: 0;
  }
  .modal-title { font-size: 18px; font-weight: 400; margin-bottom: 4px; }
  .modal-sub { font-size: 11.5px; color: var(--sumi-4); }
  .modal-close {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--sumi-3);
    font-size: 20px;
    padding: 4px 8px;
    line-height: 1;
    font-family: var(--font-ui);
    margin-left: 16px;
    flex-shrink: 0;
  }
  .modal-body {
    flex: 1;
    overflow: auto;
    padding: 20px 28px 24px;
    display: flex;
    gap: 20px;
  }

  .preview-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    overflow: hidden;
  }
  .col-label {
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--sumi-4);
  }
  .preview-title {
    background: var(--paper-2);
    border: var(--hairline);
    border-radius: 5px;
    padding: 8px 12px;
    font-size: 13px;
    color: var(--sumi);
    font-weight: 500;
    flex-shrink: 0;
  }
  .preview-body {
    flex: 1;
    background: var(--paper-2);
    border: var(--hairline);
    border-radius: 5px;
    padding: 12px 14px;
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--sumi-2);
    line-height: 1.7;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    min-height: 0;
    margin: 0;
  }

  .compose-col {
    width: 264px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .snapshot {
    background: var(--paper-2);
    border: var(--hairline);
    border-radius: 6px;
    padding: 12px 14px;
  }
  .snapshot-label {
    font-size: 9.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--sumi-4);
    margin-bottom: 8px;
  }
  .snapshot-row {
    display: flex;
    justify-content: space-between;
    font-size: 11.5px;
    margin-bottom: 3px;
  }
  .snapshot-key { color: var(--sumi-4); }
  .snapshot-val { color: var(--sumi-2); font-family: var(--font-mono); font-size: 11px; }

  .ctx-input {
    width: 100%;
    resize: none;
    font-family: var(--font-ui);
    font-size: 12.5px;
    color: var(--sumi);
    background: var(--paper-2);
    border: var(--hairline);
    border-radius: 5px;
    padding: 8px 10px;
    line-height: 1.5;
  }
  .ctx-input:focus { outline: none; border-color: var(--sumi-3); }

  .privacy-note {
    font-size: 11px;
    color: var(--sumi-4);
    line-height: 1.7;
    margin: 0;
  }
  .privacy-note code {
    font-family: var(--font-mono);
    font-size: 10.5px;
    background: var(--paper-3);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .modal-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: auto;
  }
  .action-btn { width: 100%; font-size: 13px; padding: 10px 16px; }
  .outline-btn {
    background: transparent;
    border: var(--hairline);
    color: var(--sumi);
    border-radius: 6px;
    cursor: pointer;
    font-family: var(--font-ui);
    transition: color 0.2s;
  }
  .outline-btn:hover { background: var(--paper-2); }
</style>
