<script lang="ts">
  import type { PageData } from './$types';
  import type { LogSession, BootstrapTrace } from '$lib/types.js';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { SvelteSet } from 'svelte/reactivity';

  let { data }: { data: PageData } = $props();

  // ── State ─────────────────────────────────────────────────────────────────
  // selectedId is null until user clicks; derived activeId falls back to first session
  let selectedId    = $state<string | null>(null);
  let showModal     = $state(false);
  let addCtx        = $state('');
  let expandedDates = new SvelteSet(['Today']);

  // ── Helpers ───────────────────────────────────────────────────────────────
  const MODULE_META: Record<string, { kanji: string; label: string }> = {
    bootstrap: { kanji: '健', label: 'Bootstrap' },
    wizard:    { kanji: '導', label: 'Setup Wizard' },
    projects:  { kanji: '組', label: 'Projects' },
  };

  function moduleLabel(mod: string) {
    return MODULE_META[mod] ?? { kanji: '？', label: mod };
  }

  function dateKey(s: LogSession): string {
    const d         = new Date(s.started_at);
    const now       = new Date();
    const today     = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    const dDay      = new Date(d.getFullYear(), d.getMonth(), d.getDate());
    if (dDay.getTime() === today.getTime())     return 'Today';
    if (dDay.getTime() === yesterday.getTime()) return 'Yesterday';
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  function formatTime(ts: string): string {
    return new Date(ts).toLocaleTimeString('en-US', {
      hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
    });
  }

  function fmtMs(ms: number): string {
    return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
  }

  const anonymize = (s: string) => s.replace(/\/Users\/[^/]+\//g, '~/');

  type Group  = { mod: string; sessions: LogSession[] };
  type Bucket = { date: string; groups: Group[] };

  function groupByDateThenModule(sessions: LogSession[]): Bucket[] {
    const dateOrder: string[] = [];
    const dateMap: Record<string, { modOrder: string[]; modMap: Record<string, LogSession[]> }> = {};
    for (const s of sessions) {
      const dk = dateKey(s);
      if (!dateMap[dk]) { dateMap[dk] = { modOrder: [], modMap: {} }; dateOrder.push(dk); }
      const { modOrder, modMap } = dateMap[dk];
      if (!modMap[s.module]) { modMap[s.module] = []; modOrder.push(s.module); }
      modMap[s.module].push(s);
    }
    return dateOrder.map(dk => ({
      date:   dk,
      groups: dateMap[dk].modOrder.map(mod => ({ mod, sessions: dateMap[dk].modMap[mod] })),
    }));
  }

  // ── Derived ───────────────────────────────────────────────────────────────
  const grouped         = $derived(groupByDateThenModule(data.sessions));
  // activeId falls back to first session when nothing is explicitly selected
  const activeId        = $derived(selectedId ?? data.sessions[0]?.id ?? null);
  const selectedSession = $derived(data.sessions.find(s => s.id === activeId) ?? null);

  function fixCount(session: LogSession): number {
    return (session.traces as BootstrapTrace[]).filter(t => t.fix_attempted).length;
  }

  function outcomeDot(outcome: string): string {
    return outcome === 'success' ? '#4CAF50' : outcome === 'partial' ? '#FF9800' : '#F44336';
  }

  // ── Issue body ─────────────────────────────────────────────────────────────
  function buildTitle(session: LogSession): string {
    return `Bootstrap diagnostic — ${anonymize(session.system_info.os)} · ${session.system_info.arch} · v${session.app_version}`;
  }

  function buildBody(session: LogSession, additionalCtx: string): string {
    const traces = session.traces as BootstrapTrace[];
    const traceRows = traces.map(t =>
      `| ${t.step} | ${t.action_type} | \`${anonymize(t.cmd)}\` | ${fmtMs(t.ms)} | ${t.ok ? '✓' : '✗'} |`
    ).join('\n');

    const fixTraces = traces.filter(t => t.err || t.fix_attempted);
    const fixMd = fixTraces.length > 0
      ? [
          '',
          '### Fix details',
          '',
          ...fixTraces.flatMap(t => [
            `**${t.step}** (${t.action_type} · ${fmtMs(t.ms)}) — ${t.ok ? '✓' : t.fix_ok ? '✓ fixed' : '✗ failed'}`,
            ...(t.err           ? [`- stderr: \`${anonymize(t.err)}\``] : []),
            ...(t.fix_attempted ? [`- fix applied: \`$ ${anonymize(t.fix_approach ?? '')}\` → ${t.fix_ok ? 'success' : 'failed'}`] : []),
          ]),
        ].join('\n')
      : '';

    const ctxSection = additionalCtx ? `\n\n## Additional context\n\n${additionalCtx}` : '';

    return `## System

| | |
|---|---|
| OS | ${anonymize(session.system_info.os)} |
| Arch | ${session.system_info.arch} |
| RAM | ${session.system_info.ram_gb} GB |
| CPU cores | ${session.system_info.cpu_cores} |
| App version | v${session.app_version} |
| Outcome | ${session.outcome} |
| Duration | ${fmtMs(session.duration_ms)} |

## Bootstrap trace

| Step | Type | Command | Duration | Result |
|------|------|---------|----------|--------|
${traceRows}
${fixMd}${ctxSection}`;
  }

  async function submitToGitHub() {
    if (!selectedSession) return;
    const title = buildTitle(selectedSession);
    const body  = buildBody(selectedSession, addCtx);
    const url   = `https://github.com/sensei-hq/app/issues/new?title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`;
    await openUrl(url);
  }

  async function copyMarkdown() {
    if (!selectedSession) return;
    await navigator.clipboard.writeText(buildBody(selectedSession, addCtx));
  }
</script>

<!-- ── Layout ─────────────────────────────────────────────────────────────── -->
<div class="logs-root">

  <!-- ── Sidebar ──────────────────────────────────────────────────────────── -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <span class="kanji sidebar-kanji">録</span>
      <span class="sidebar-title">Diagnostic Logs</span>
    </div>

    <div class="session-list">
      {#each grouped as bucket (bucket.date)}
        <!-- Date group header -->
        <button
          class="date-header"
          onclick={() => {
            if (expandedDates.has(bucket.date)) {
              expandedDates.delete(bucket.date);
            } else {
              expandedDates.add(bucket.date);
            }
          }}
        >
          <span class="date-label">{bucket.date}</span>
          <span class="date-chevron">{expandedDates.has(bucket.date) ? '▾' : '▸'}</span>
        </button>

        {#if expandedDates.has(bucket.date)}
          {#each bucket.groups as group (group.mod)}
            <!-- Module sub-header -->
            <div class="module-header">
              <span class="kanji module-kanji">{moduleLabel(group.mod).kanji}</span>
              <span class="module-label">{moduleLabel(group.mod).label.toUpperCase()}</span>
            </div>

            <!-- Session rows -->
            {#each group.sessions as session (session.id)}
              <button
                class="session-row"
                class:selected={activeId === session.id}
                onclick={() => { selectedId = session.id; showModal = false; }}
              >
                <span
                  class="outcome-dot"
                  style:background={outcomeDot(session.outcome)}
                ></span>
                <span class="session-time">{formatTime(session.started_at)}</span>
                <span class="session-meta">
                  {fmtMs(session.duration_ms)} · {session.traces.length} steps
                  {#if fixCount(session) > 0}
                    · {fixCount(session)} fix{fixCount(session) > 1 ? 'es' : ''}
                  {/if}
                </span>
              </button>
            {/each}
          {/each}
        {/if}
      {/each}

      {#if data.sessions.length === 0}
        <p class="empty-state">No sessions yet. Run the health check to generate logs.</p>
      {/if}
    </div>
  </aside>

  <!-- ── Trace panel ───────────────────────────────────────────────────────── -->
  <main class="trace-panel">
    {#if selectedSession}
      <div class="trace-header">
        <div class="trace-header-left">
          <span class="kanji trace-kanji">{moduleLabel(selectedSession.module).kanji}</span>
          <div>
            <div class="trace-title">{moduleLabel(selectedSession.module).label}</div>
            <div class="trace-meta">
              {selectedSession.started_at.replace('T', ' ').replace('Z', ' UTC')}
              · {fmtMs(selectedSession.duration_ms)}
              · <span
                  style:color={outcomeDot(selectedSession.outcome)}
                >{selectedSession.outcome}</span>
            </div>
          </div>
        </div>
        <button
          class="report-btn"
          onclick={() => { showModal = true; }}
        >
          Report this session ↗
        </button>
      </div>

      <table class="trace-table">
        <thead>
          <tr>
            <th>Step</th>
            <th>Type</th>
            <th>Command</th>
            <th>Duration</th>
            <th>Result</th>
          </tr>
        </thead>
        <tbody>
          {#each selectedSession.traces as trace (trace.id)}
            {@const t = trace as BootstrapTrace}
            <tr class:trace-fail={!t.ok}>
              <td class="step-cell">{t.step ?? ''}</td>
              <td>
                <span class="badge badge-{t.action_type ?? 'check'}">{t.action_type ?? ''}</span>
              </td>
              <td class="cmd-cell"><code>{anonymize(t.cmd ?? '')}</code></td>
              <td class="dur-cell">{fmtMs(t.ms ?? 0)}</td>
              <td class="result-cell">
                {#if t.ok}
                  <span class="result-ok">✓</span>
                {:else if t.fix_ok}
                  <span class="result-fixed">✓ fixed</span>
                {:else}
                  <span class="result-fail">✗</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <div class="no-session">Select a session from the sidebar.</div>
    {/if}
  </main>
</div>

<!-- ── Report Modal ──────────────────────────────────────────────────────── -->
{#if showModal && selectedSession}
  <div
    class="modal-overlay"
    role="dialog"
    aria-modal="true"
    onclick={(e) => { if (e.target === e.currentTarget) showModal = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') showModal = false; }}
    tabindex="-1"
  >
    <div class="modal-panel">
      <div class="modal-header">
        <span class="modal-title">Report Session</span>
        <button class="modal-close" onclick={() => { showModal = false; }}>✕</button>
      </div>

      <div class="modal-body">
        <!-- Left: issue preview -->
        <div class="preview-col">
          <div class="preview-label">ISSUE PREVIEW — ANONYMIZED</div>
          <div class="preview-title">
            {buildTitle(selectedSession)}
          </div>
          <pre class="body-preview">{buildBody(selectedSession, addCtx)}</pre>
        </div>

        <!-- Right: submission panel -->
        <div class="submit-col">
          <div class="included-panel">
            <div class="included-title">Included in report</div>
            <ul class="included-list">
              <li>System info (OS, arch, RAM)</li>
              <li>{selectedSession.traces.length} trace steps</li>
              {#if fixCount(selectedSession) > 0}
                <li>{fixCount(selectedSession)} fix attempt(s)</li>
              {/if}
              <li>App version v{selectedSession.app_version}</li>
            </ul>
          </div>

          <label class="ctx-label" for="add-ctx">Additional context</label>
          <textarea
            id="add-ctx"
            class="ctx-input"
            bind:value={addCtx}
            placeholder="Describe what you were doing when the problem occurred…"
            rows={4}
          ></textarea>

          <p class="privacy-note">
            Paths anonymized: <code>/Users/name/</code> → <code>~/</code>
          </p>

          <button class="submit-btn" onclick={submitToGitHub}>
            Submit to GitHub ↗
          </button>
          <button class="copy-btn" onclick={copyMarkdown}>
            Copy markdown
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Layout ─────────────────────────────────────────────────────── */
  .logs-root {
    display: flex;
    height: 100%;
    overflow: hidden;
    background: var(--paper, #faf9f7);
    color: var(--sumi, #1a1a1a);
    font-family: var(--font-sans, system-ui, sans-serif);
  }

  /* ── Sidebar ─────────────────────────────────────────────────────────── */
  .sidebar {
    width: 248px;
    min-width: 248px;
    border-right: 1px solid var(--hairline, rgba(0,0,0,.08));
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 12px 10px;
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .sidebar-kanji { font-size: 22px; }
  .sidebar-title { font-size: 13px; font-weight: 600; letter-spacing: .04em; }
  .session-list { flex: 1; overflow-y: auto; padding: 4px 0; }

  .date-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 12px 4px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--sumi-60, #666);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: .08em;
    text-align: left;
    text-transform: uppercase;
  }
  .date-header:hover { color: var(--sumi, #1a1a1a); }
  .date-chevron { font-size: 10px; }

  .module-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px 2px 20px;
  }
  .module-kanji { font-size: 14px; opacity: .6; }
  .module-label { font-size: 10px; letter-spacing: .1em; color: var(--sumi-60, #666); }

  .session-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px 6px 28px;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    border-radius: 4px;
    margin: 1px 4px;
    width: calc(100% - 8px);
  }
  .session-row:hover     { background: var(--hover-bg, rgba(0,0,0,.04)); }
  .session-row.selected  { background: var(--selected-bg, rgba(0,0,0,.07)); }
  .outcome-dot  { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
  .session-time { font-size: 12px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .session-meta { font-size: 11px; color: var(--sumi-50, #888); flex: 1; }

  .empty-state { padding: 20px 12px; font-size: 12px; color: var(--sumi-50, #888); }

  /* ── Trace panel ──────────────────────────────────────────────────────── */
  .trace-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .trace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .trace-header-left { display: flex; align-items: center; gap: 10px; }
  .trace-kanji { font-size: 28px; }
  .trace-title { font-size: 15px; font-weight: 600; }
  .trace-meta  { font-size: 12px; color: var(--sumi-50, #888); }
  .report-btn  {
    padding: 7px 14px;
    background: var(--sumi, #1a1a1a);
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .report-btn:hover { opacity: .85; }

  .trace-table {
    flex: 1;
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    overflow-y: auto;
    display: block;
  }
  .trace-table thead { position: sticky; top: 0; background: var(--paper, #faf9f7); }
  .trace-table th {
    padding: 8px 12px;
    text-align: left;
    font-size: 11px;
    letter-spacing: .06em;
    color: var(--sumi-50, #888);
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .trace-table td  { padding: 7px 12px; border-bottom: 1px solid var(--hairline, rgba(0,0,0,.05)); }
  .trace-fail td   { background: rgba(244,67,54,.04); }
  .cmd-cell code   { font-size: 11px; font-family: var(--font-mono, monospace); }
  .dur-cell        { font-variant-numeric: tabular-nums; }
  .result-ok       { color: #4CAF50; font-weight: 600; }
  .result-fixed    { color: #FF9800; font-weight: 600; }
  .result-fail     { color: #F44336; font-weight: 600; }
  .badge           { font-size: 10px; padding: 2px 6px; border-radius: 3px; background: rgba(0,0,0,.07); }
  .no-session { display: flex; align-items: center; justify-content: center; height: 100%; color: var(--sumi-50, #888); }

  /* ── Modal ────────────────────────────────────────────────────────────── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal-panel {
    width: 84vw;
    max-width: 960px;
    max-height: 90vh;
    background: var(--paper, #faf9f7);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0,0,0,.28);
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .modal-title { font-size: 14px; font-weight: 600; }
  .modal-close { background: none; border: none; font-size: 16px; cursor: pointer; color: var(--sumi-50, #888); }
  .modal-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  /* Left column */
  .preview-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 16px;
    overflow: hidden;
    border-right: 1px solid var(--hairline, rgba(0,0,0,.08));
  }
  .preview-label { font-size: 10px; letter-spacing: .1em; color: var(--sumi-50, #888); margin-bottom: 8px; }
  .preview-title {
    font-size: 13px;
    font-weight: 500;
    padding: 8px;
    background: rgba(0,0,0,.04);
    border-radius: 4px;
    margin-bottom: 8px;
  }
  .body-preview {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    white-space: pre-wrap;
    background: rgba(0,0,0,.04);
    padding: 10px;
    border-radius: 4px;
    margin: 0;
  }

  /* Right column */
  .submit-col {
    width: 264px;
    min-width: 264px;
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 12px;
    overflow-y: auto;
  }
  .included-panel {
    padding: 10px;
    background: rgba(0,0,0,.04);
    border-radius: 6px;
  }
  .included-title { font-size: 11px; font-weight: 600; letter-spacing: .06em; margin-bottom: 6px; }
  .included-list  { font-size: 12px; padding-left: 16px; margin: 0; color: var(--sumi-60, #555); }
  .ctx-label      { font-size: 12px; font-weight: 500; }
  .ctx-input      {
    width: 100%;
    resize: vertical;
    font-size: 12px;
    padding: 8px;
    border: 1px solid var(--hairline, rgba(0,0,0,.15));
    border-radius: 6px;
    background: transparent;
    font-family: inherit;
  }
  .privacy-note { font-size: 11px; color: var(--sumi-50, #888); margin: 0; }
  .privacy-note code { font-size: 10px; }
  .submit-btn, .copy-btn {
    width: 100%;
    padding: 9px 14px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .submit-btn {
    background: var(--sumi, #1a1a1a);
    color: #fff;
    border: none;
  }
  .copy-btn {
    background: none;
    border: 1px solid var(--hairline, rgba(0,0,0,.2));
    color: var(--sumi, #1a1a1a);
  }
  .submit-btn:hover { opacity: .85; }
  .copy-btn:hover   { background: rgba(0,0,0,.04); }
</style>
