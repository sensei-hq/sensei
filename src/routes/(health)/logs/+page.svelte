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
<div class="h-full flex overflow-hidden text-surface-z9">

  <!-- ── Sidebar ─────────────────────────────────────────────────────────── -->
  <aside class="w-[248px] shrink-0 border-r border-surface-z2 bg-surface-z2 flex flex-col overflow-hidden">
    <div class="px-5 pt-[22px] pb-[14px] border-b border-surface-z2 flex items-center gap-2.5 shrink-0">
      <span class="kanji text-[18px] text-primary-z5">診</span>
      <span class="text-[11px] tracking-[0.14em] uppercase text-surface-z6">diagnostic logs</span>
    </div>

    <div class="flex-1 overflow-y-auto py-2">
      {#each dateGroups as dg (dg.date)}
        {@const open = state.expandedDates.has(dg.date)}

        <!-- Date header -->
        <button
          class="flex items-center gap-1.5 w-full px-5 pt-[7px] pb-[5px] bg-transparent border-none cursor-pointer text-left"
          onclick={() => state.toggleDate(dg.date)}
        >
          <span class="text-[9px] text-surface-z5 leading-none inline-block transition-transform duration-150" class:collapsed={!open}>▾</span>
          <span class="text-[11px] font-semibold tracking-[0.06em] uppercase text-surface-z7">{dg.date}</span>
        </button>

        {#if open}
          {#each dg.sessions as s (s.id)}
            {@const sel = s.id === state.selectedId}
            {@const isCurrent = s.id === state.sessions[0]?.id}
            <button
              class="session-row block w-full py-[7px] px-5 pl-9 border-none border-l-2 border-l-transparent cursor-pointer text-left transition-colors duration-[120ms]"
              class:selected={sel}
              onclick={() => state.selectSession(s.id)}
            >
              {#if isCurrent}
                <span class="block text-[9px] tracking-[0.13em] uppercase text-primary-z5 mb-0.5">current</span>
              {/if}
              <div class="flex items-center gap-1.5 mb-0.5">
                <span class="w-1.5 h-1.5 rounded-full shrink-0" style:background={outcomeColor(s.outcome)}></span>
                <span class="session-time text-[12.5px] font-medium text-surface-z7">{timeOfDay(s.started_at)}</span>
              </div>
              <div class="ml-3 text-[10.5px] text-surface-z5">{moduleLabel(s.module)} · {sessionSummary(s)}</div>
            </button>
          {/each}
        {/if}
      {/each}

      {#if state.sessions.length === 0}
        <div class="px-5 py-4 text-xs text-surface-z5">No sessions yet</div>
      {/if}
    </div>
  </aside>

  <!-- ── Main panel ──────────────────────────────────────────────────────── -->
  <main class="flex-1 min-w-0 flex flex-col overflow-hidden">
    {#if state.session}
      {@const s   = state.session}
      {@const si  = s.system_info}
      {@const bt  = s.traces.filter(isBootstrapTrace)}
      {@const fixes = bt.filter(t => t.fix_attempted).length}

      <!-- Session header -->
      <div class="shrink-0 pt-[22px] px-9 border-b border-surface-z2">
        <div class="flex items-start justify-between mb-4">
          <div>
            <div class="text-[9.5px] tracking-[0.14em] uppercase text-surface-z5 mb-1.5">{moduleLabel(s.module)}</div>
            <h2 class="display text-[22px] font-normal mb-1.5 tracking-[-0.01em]">{formatTime(s.started_at)}</h2>
            <div class="flex items-center flex-wrap text-xs text-surface-z5">
              {#each [anonymize(si.os), si.arch, `${si.ram_gb} GB`, `${si.cpu_cores} cores`, `v${s.app_version}`] as v, i}
                {#if i > 0}<span class="mx-2 opacity-40">·</span>{/if}
                <span>{v}</span>
              {/each}
            </div>
          </div>

          <button class="report-btn shrink-0 mt-1 text-[12.5px] font-medium border-none px-4 py-[9px] rounded-md cursor-pointer bg-surface-z9 text-surface-z1" onclick={() => state.openModal()}>
            Report this session ↗
          </button>
        </div>

        <!-- Stats row -->
        <div class="flex gap-6 pb-4">
          {#each [
            { label: 'Total time', value: fmtMs(s.duration_ms) },
            { label: 'Traces',     value: String(bt.length) },
            { label: 'Auto-fixes', value: String(fixes), color: fixes > 0 ? 'oklch(var(--color-warning-z5) / 1)' : undefined },
            { label: 'Outcome',    value: s.outcome, color: outcomeColor(s.outcome) },
          ] as stat (stat.label)}
            <div class="flex flex-col gap-0.5">
              <div class="text-[9.5px] tracking-[0.12em] uppercase text-surface-z5">{stat.label}</div>
              <div class="text-sm font-medium" style:color={stat.color ?? 'oklch(var(--color-surface-z9) / 1)'}>{stat.value}</div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Column headers -->
      <div class="grid grid-cols-[68px_148px_1fr_52px_40px] gap-3 px-9 py-[7px] border-b border-surface-z2 bg-surface-z2 shrink-0 text-[9.5px] tracking-[0.12em] uppercase text-surface-z5">
        <span>action</span>
        <span>step</span>
        <span>command</span>
        <span class="text-right">duration</span>
        <span></span>
      </div>

      <!-- Trace list -->
      <div class="flex-1 overflow-y-auto px-9">
        {#each bt as t (t.id)}
          {@const am       = actionMeta(t.action_type ?? 'check')}
          {@const isOpen   = state.expandedTraceId === t.id}
          {@const hasDetail = !!(t.out || t.err || t.fix_attempted)}

          <div class="border-b border-surface-z2">
            <!-- Main row -->
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div
              class="trace-row grid grid-cols-[68px_148px_1fr_52px_40px] gap-3 items-center py-[9px] {hasDetail ? 'cursor-pointer' : 'cursor-default'}"
              role={hasDetail ? 'button' : 'presentation'}
              tabindex={hasDetail ? 0 : -1}
              onclick={() => { if (hasDetail) state.toggleTrace(t.id); }}
              onkeydown={(e) => { if (hasDetail && (e.key === 'Enter' || e.key === ' ')) state.toggleTrace(t.id); }}
            >
              <span class="text-[9px] font-semibold tracking-[0.12em] px-1.5 py-[2px] rounded-[3px] whitespace-nowrap justify-self-start" style:color={am.color} style:background={am.bg}>
                {am.label}
              </span>
              <span class="text-xs font-mono text-surface-z7 overflow-hidden whitespace-nowrap text-ellipsis">{t.step ?? ''}</span>
              <span class="text-[11.5px] font-mono text-surface-z5 overflow-hidden whitespace-nowrap text-ellipsis">{anonymize(t.cmd ?? '')}</span>
              <span class="text-[11px] text-surface-z5 tabular-nums text-right">{fmtMs(t.ms ?? 0)}</span>
              <div class="text-center">
                {#if t.ok}
                  <span class="text-success-z5 text-[13px] leading-none">✓</span>
                {:else if t.fix_ok}
                  <span class="text-[9px] font-semibold tracking-[0.06em] text-warning-z5">FIXED</span>
                {:else}
                  <span class="text-primary-z5 text-[13px] leading-none">✗</span>
                {/if}
              </div>
            </div>

            <!-- Expanded detail -->
            {#if isOpen && hasDetail}
              <div class="ml-20 mb-3 px-[14px] py-3 bg-surface-z2 rounded-md border border-surface-z2 flex flex-col gap-2.5">
                {#if t.out}
                  <div class="flex flex-col gap-1">
                    <div class="text-[9.5px] tracking-[0.1em] uppercase text-surface-z5">stdout</div>
                    <pre class="m-0 text-[11.5px] font-mono text-surface-z7 leading-[1.6] whitespace-pre-wrap break-all">{anonymize(t.out)}</pre>
                  </div>
                {/if}
                {#if t.err}
                  <div class="flex flex-col gap-1">
                    <div class="detail-block-label err text-[9.5px] tracking-[0.1em] uppercase">stderr</div>
                    <pre class="err-text m-0 text-[11.5px] font-mono leading-[1.6] whitespace-pre-wrap break-all">{t.err}</pre>
                  </div>
                {/if}
                {#if t.fix_attempted}
                  <div class="flex flex-col gap-1 pt-[10px] border-t border-surface-z2">
                    <div class="detail-block-label fix text-[9.5px] tracking-[0.1em] uppercase">auto-fix attempted</div>
                    <pre class="m-0 text-[11.5px] font-mono leading-[1.6] whitespace-pre-wrap break-all" style:color={t.fix_ok ? 'oklch(var(--color-success-z5) / 1)' : 'oklch(var(--color-primary-z5) / 1)'}>$ {t.fix_approach ?? ''}  <span class="text-surface-z5 text-[10px]">→ {t.fix_ok ? 'succeeded' : 'failed'}</span></pre>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
        <div class="h-8"></div>
      </div>

    {:else}
      <div class="flex-1 flex items-center justify-center text-[13px] text-surface-z5">No sessions recorded yet. Run the app to generate logs.</div>
    {/if}
  </main>
</div>

<!-- ── Report modal ──────────────────────────────────────────────────────── -->
{#if state.showModal && state.session}
  {@const s  = state.session}
  {@const si = s.system_info}
  <div
    class="modal-overlay fixed inset-0 flex items-center justify-center z-30 p-6"
    role="dialog"
    aria-modal="true"
    onclick={(e) => { if (e.target === e.currentTarget) state.closeModal(); }}
    onkeydown={(e) => { if (e.key === 'Escape') state.closeModal(); }}
    tabindex="-1"
  >
    <div class="modal w-[84vw] max-w-[960px] max-h-[90vh] bg-surface-z1 rounded-xl border border-surface-z2 flex flex-col">
      <div class="flex items-start justify-between pt-4 pb-3 px-7 border-b border-surface-z2 shrink-0">
        <div>
          <div class="display text-[18px] font-normal mb-1">Report session</div>
          <div class="text-[11.5px] text-surface-z5">
            {formatTime(s.started_at)} · {s.traces.length} traces ·
            {s.traces.filter(t => isBootstrapTrace(t) && t.fix_attempted).length} auto-fixes
          </div>
        </div>
        <button class="bg-transparent border-none cursor-pointer text-surface-z6 text-[20px] px-2 py-1 leading-none ml-4 shrink-0" onclick={() => state.closeModal()}>×</button>
      </div>

      <div class="flex-1 overflow-auto pt-5 pb-6 px-7 flex gap-5">
        <!-- Issue preview -->
        <div class="flex-1 flex flex-col gap-2 min-h-0 overflow-hidden">
          <div class="text-[10px] tracking-[0.12em] uppercase text-surface-z5">Issue preview — anonymized</div>
          <div class="bg-surface-z2 border border-surface-z2 rounded-md px-3 py-2 text-[13px] text-surface-z9 font-medium shrink-0">{buildTitle(s)}</div>
          <pre class="flex-1 bg-surface-z2 border border-surface-z2 rounded-md px-[14px] py-3 font-mono text-[11.5px] text-surface-z7 leading-[1.7] overflow-auto whitespace-pre-wrap break-words min-h-0 m-0">{buildBody(s, state.addCtx)}</pre>
        </div>

        <!-- Compose column -->
        <div class="w-[264px] shrink-0 flex flex-col gap-3">
          <div class="bg-surface-z2 border border-surface-z2 rounded-md px-[14px] py-3">
            <div class="text-[9.5px] tracking-[0.12em] uppercase text-surface-z5 mb-2">Included in report</div>
            {#each [
              ['Session',  formatTime(s.started_at)],
              ['OS',       anonymize(si.os)],
              ['Arch',     si.arch],
              ['RAM',      `${si.ram_gb} GB`],
              ['Traces',   String(s.traces.length)],
              ['Fixes',    String(s.traces.filter(t => isBootstrapTrace(t) && t.fix_attempted).length)],
              ['App',      `v${s.app_version}`],
            ] as [k, v]}
              <div class="flex justify-between text-[11.5px] mb-[3px]">
                <span class="text-surface-z5">{k}</span>
                <span class="text-surface-z7 font-mono text-[11px]">{v}</span>
              </div>
            {/each}
          </div>

          <div>
            <div class="text-[10px] tracking-[0.12em] uppercase text-surface-z5 mb-2">Additional context</div>
            <textarea
              class="ctx-input w-full resize-none font-sans text-[12.5px] text-surface-z9 bg-surface-z2 border border-surface-z2 rounded-md px-[10px] py-2 leading-[1.5]"
              bind:value={state.addCtx}
              placeholder="What were you doing? Any other details…"
              rows={4}
            ></textarea>
          </div>

          <p class="privacy-note text-[11px] text-surface-z5 leading-[1.7] m-0">
            Paths like <code>/Users/</code> are replaced with <code>~/</code>. No personal data included.
          </p>

          <div class="flex flex-col gap-2 mt-auto">
            <button class="btn-solid w-full text-[13px] px-4 py-[10px]" onclick={submitToGitHub}>
              Submit to GitHub ↗
            </button>
            <button class="outline-btn w-full text-[13px] px-4 py-[10px] bg-transparent border border-surface-z2 text-surface-z9 rounded-md cursor-pointer font-sans transition-colors duration-200" onclick={copyMarkdown}>
              {state.copied ? 'Copied ✓' : 'Copy markdown'}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Date chevron collapsed rotation */
  .collapsed { transform: rotate(-90deg); }

  /* Session row hover/selected states */
  .session-row:hover { background: oklch(var(--color-surface-z3) / 1); }
  .session-row.selected { background: oklch(var(--color-surface-z1) / 1); border-left-color: oklch(var(--color-primary-z5) / 1); }
  .session-row.selected .session-time { color: oklch(var(--color-surface-z9) / 1); }

  /* Report button hover */
  .report-btn:hover { opacity: 0.85; }

  /* Detail label variants */
  .detail-block-label.err { color: oklch(var(--color-primary-z5) / 1); }
  .detail-block-label.fix { color: oklch(var(--color-warning-z5) / 1); }

  /* Error text override */
  .err-text { color: oklch(var(--color-primary-z5) / 1) !important; }

  /* Input focus ring */
  .ctx-input:focus { outline: none; border-color: oklch(var(--color-surface-z6) / 1); }

  /* Code snippets in privacy note */
  .privacy-note code {
    font-family: var(--font-mono); font-size: 10.5px;
    background: oklch(var(--color-surface-z3) / 1);
    padding: 1px 5px; border-radius: 3px;
  }

  /* Outline button hover */
  .outline-btn:hover { background: oklch(var(--color-surface-z2) / 1); }

  /* Modal overlay backdrop */
  .modal-overlay { background: rgba(0, 0, 0, 0.30); }

  /* Modal drop shadow */
  .modal { box-shadow: 0 24px 60px rgba(0, 0, 0, 0.18); }
</style>
