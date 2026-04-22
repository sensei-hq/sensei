<script lang="ts">
  import type { WizardState, WizUpdate, ScanEvent } from '../types.js';
  import { MOCK_SCAN_EVENTS } from '../mock.js';

  let { wizState, update, onScan, daemonReady = false }: {
    wizState: WizardState;
    update: WizUpdate;
    onScan?: () => Promise<void>;
    daemonReady?: boolean;
  } = $props();

  let eventIndex = $state(0);
  let timerActive = $state(false);

  // ── Derived stats from visible events ────────────────────────
  const visibleEvents = $derived(wizState.scanEvents);

  const roots = $derived(
    wizState.folders.length
  );

  const repos = $derived(
    visibleEvents.filter(e => e.level === 'discover').length
  );

  const projects = $derived(
    new Set(visibleEvents.filter(e => e.parent).map(e => e.parent)).size
  );

  const files = $derived(
    visibleEvents
      .filter(e => e.level === 'queue' || e.level === 'process')
      .reduce((n, e) => {
        const m = e.msg.match(/([\d,]+)\s*files/);
        return m ? n + parseInt(m[1].replace(/,/g, ''), 10) : n;
      }, 0)
  );

  // ── Project cards: group repos by parent ─────────────────────
  interface ProjectCard {
    name: string;
    repos: { name: string; queued: boolean; processed: boolean }[];
  }

  const projectCards = $derived.by((): ProjectCard[] => {
    const map = new Map<string, ProjectCard>();
    for (const ev of visibleEvents) {
      if (!ev.parent) continue;
      if (!map.has(ev.parent)) map.set(ev.parent, { name: ev.parent, repos: [] });
      const card = map.get(ev.parent)!;
      if (ev.level === 'discover') {
        const repoName = ev.msg.split('/').pop()?.replace(' \u00b7 found git repo', '').trim() ?? '';
        if (repoName && !card.repos.some(r => r.name === repoName)) {
          card.repos.push({ name: repoName, queued: false, processed: false });
        }
      }
      if (ev.level === 'queue') {
        const repoName = ev.msg.split(' \u00b7 ')[0].trim();
        const repo = card.repos.find(r => r.name === repoName);
        if (repo) repo.queued = true;
      }
      if (ev.level === 'process') {
        const repoName = ev.msg.split(' \u00b7 ')[0].trim();
        const repo = card.repos.find(r => r.name === repoName);
        if (repo) repo.processed = true;
      }
    }
    return [...map.values()];
  });

  // ── Color map for event levels ───────────────────────────────
  function levelColor(level: ScanEvent['level']): string {
    switch (level) {
      case 'info':     return 'var(--sumi-3)';
      case 'discover': return 'var(--shu)';
      case 'queue':    return 'var(--amber)';
      case 'process':  return 'var(--jade)';
      case 'success':  return 'var(--jade)';
    }
  }

  function levelLabel(level: ScanEvent['level']): string {
    switch (level) {
      case 'info':     return 'INFO';
      case 'discover': return 'DISC';
      case 'queue':    return 'QUEUE';
      case 'process':  return 'PROC';
      case 'success':  return 'DONE';
    }
  }

  // ── Start scan: animate through MOCK_SCAN_EVENTS ─────────────
  // Accumulated events — kept outside of wizState to avoid stale closure issues
  let localEvents = $state<ScanEvent[]>([]);

  // Sync local events to wizard state
  $effect(() => {
    if (localEvents.length > 0) {
      update({ scanEvents: localEvents });
    }
  });

  function addEvent(level: ScanEvent['level'], msg: string) {
    localEvents = [...localEvents, { t: Date.now(), level, msg }];
  }

  async function startScan() {
    localEvents = [];
    update({ scanStarted: true, scanEvents: [] });

    if (!daemonReady) return;

    const { senseiApi } = await import('$lib/api.js');
    const { getPort } = await import('$lib/appstate.svelte.js');
    const port = getPort();
    const api = senseiApi(port);

    addEvent('info', `scan started · ${wizState.folders.length} roots`);

    // Subscribe to SSE progress before triggering scan
    const es = new EventSource(`http://127.0.0.1:${port}/api/index/progress`);
    let scanRootsCompleted = 0;
    const totalRoots = wizState.folders.length;

    es.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const kind = data.kind ?? '';
        const status = data.status ?? '';
        const repoId = data.repo_id ?? '';

        if (kind === 'ScanRoot' && status === 'completed') {
          scanRootsCompleted++;
          addEvent('success', `root scan complete (${scanRootsCompleted}/${totalRoots})`);

          // All roots done — load results
          if (scanRootsCompleted >= totalRoots) {
            setTimeout(async () => {
              es.close();
              if (onScan) await onScan();
              update({ scanDone: true });
              addEvent('success', 'all scans complete');
            }, 1000);
          }
        } else if (kind === 'ProcessRepo' && status === 'running') {
          addEvent('discover', `${repoId} · found`);
        } else if (kind === 'ProcessRepo' && status === 'completed') {
          addEvent('process', `${repoId} · indexed`);
        } else if (kind === 'RepoQueued') {
          addEvent('queue', `${repoId} · ${data.files_total ?? 0} files queued`);
        }
      } catch { /* ignore parse errors */ }
    };

    es.onerror = () => {
      // SSE connection lost — close and finish
      es.close();
      addEvent('info', 'connection to daemon lost');
    };

    // Trigger scan for each folder
    for (const folder of wizState.folders) {
      addEvent('info', `scanning ${folder.path}`);
      await api.scanFolder(folder.path);
    }
  }

  // Mock animation effect (only when not using real daemon)
  $effect(() => {
    if (!timerActive) return;
    if (eventIndex >= MOCK_SCAN_EVENTS.length) {
      timerActive = false;
      return;
    }

    const current = MOCK_SCAN_EVENTS[eventIndex];
    const prevT = eventIndex > 0 ? MOCK_SCAN_EVENTS[eventIndex - 1].t : 0;
    const delay = eventIndex === 0 ? 0 : (current.t - prevT) / 2;

    const timer = setTimeout(() => {
      const next = [...wizState.scanEvents, current];
      const isDone = eventIndex === MOCK_SCAN_EVENTS.length - 1;
      update({
        scanEvents: next,
        scanTick: eventIndex,
        scanDone: isDone,
      });
      eventIndex++;
    }, delay);

    return () => clearTimeout(timer);
  });

  // ── Format time offset ───────────────────────────────────────
  function fmtTime(t: number): string {
    const s = (t / 1000).toFixed(1);
    return `${s}s`;
  }

  // ── Auto-scroll the event log ────────────────────────────────
  let logEl: HTMLElement | undefined = $state();

  $effect(() => {
    if (logEl && visibleEvents.length) {
      logEl.scrollTop = logEl.scrollHeight;
    }
  });
</script>

<section class="step">
  <div class="step-label"><span class="kanji">五</span> STEP</div>
  <h1 class="display headline">
    {#if wizState.scanDone}Scan complete{:else}Scan{/if}
  </h1>
  <p class="subtitle">
    {#if !wizState.scanStarted}
      Ready to scan {roots} root{roots !== 1 ? 's' : ''}.
    {:else if wizState.scanDone}
      The map is drawn.
    {:else}
      Scanning...
    {/if}
  </p>

  {#if !wizState.scanStarted}
    <!-- Pre-scan: hero card with start button -->
    <div class="hero-card">
      <div class="hero-kanji kanji">探</div>
      <p class="hero-text">
        The daemon will recurse your folders, identify repositories,
        and extract the code graph. Two workers, ~2M files / minute
        on this machine.
      </p>
      <button class="begin-btn" onclick={startScan}>
        Begin scan &rarr;
      </button>
    </div>
  {:else}
    <!-- Stats bar -->
    <div class="stats-bar">
      <div class="stat">
        <div class="stat-value">{roots}</div>
        <div class="stat-label">ROOTS</div>
      </div>
      <div class="stat">
        <div class="stat-value">{repos}</div>
        <div class="stat-label">DISCOVERED</div>
      </div>
      <div class="stat">
        <div class="stat-value">{projects}</div>
        <div class="stat-label">PROJECTS</div>
      </div>
      <div class="stat">
        <div class="stat-value">{files}</div>
        <div class="stat-label">PROCESSED</div>
      </div>
    </div>

    <!-- Main content: project cards + event log -->
    <div class="scan-body">
      <!-- Left: project cards -->
      <div class="project-list">
        {#each projectCards as card}
          <div class="project-card">
            <div class="project-header">
              <span class="project-kanji kanji" style="color: var(--shu);">工</span>
              <span class="project-name">{card.name}</span>
            </div>
            <div class="repo-list">
              {#each card.repos as repo}
                <div class="repo-row">
                  <span class="repo-name">{repo.name}</span>
                  <div class="mini-bar">
                    <div
                      class="mini-fill"
                      style="width: {repo.processed ? '100%' : repo.queued ? '50%' : '10%'}; background: {repo.processed ? 'var(--jade)' : repo.queued ? 'var(--amber)' : 'var(--paper-edge)'};"
                    ></div>
                  </div>
                  <span class="repo-status">
                    {#if repo.processed}done{:else if repo.queued}queued{:else}found{/if}
                  </span>
                </div>
              {/each}
            </div>
          </div>
        {/each}

        {#if projectCards.length === 0}
          <div class="empty-hint">Waiting for scan events...</div>
        {/if}
      </div>

      <!-- Right: event log -->
      <div class="event-log" bind:this={logEl}>
        {#each visibleEvents as ev, i}
          <div class="log-line">
            <span class="log-time">{fmtTime(ev.t)}</span>
            <span class="log-level" style="color: {levelColor(ev.level)};">
              {levelLabel(ev.level)}
            </span>
            <span class="log-msg">{ev.msg}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</section>

<style>
  .step {
    padding: var(--space-10) var(--space-12);
    max-width: 960px;
  }

  .step-label {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    margin-bottom: var(--space-2);
  }

  .step-label .kanji {
    color: var(--shu);
    margin-right: 4px;
  }

  .headline {
    font-size: 40px;
    color: var(--sumi);
    margin: 0 0 var(--space-2) 0;
    line-height: 1.15;
  }

  .subtitle {
    font-size: 15px;
    color: var(--sumi-3);
    margin: 0 0 var(--space-8) 0;
  }

  /* ── Hero card (pre-scan) ─────────────────────────────────── */

  .hero-card {
    background: var(--paper-2);
    border: var(--hairline);
    border-radius: var(--radius-lg);
    padding: var(--space-12) var(--space-10);
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-6);
  }

  .hero-kanji {
    font-size: 80px;
    color: var(--shu);
    opacity: 0.25;
    line-height: 1;
  }

  .hero-text {
    font-size: 15px;
    color: var(--sumi-2);
    max-width: 440px;
    line-height: 1.6;
    margin: 0;
  }

  .begin-btn {
    font-size: 14px;
    font-family: var(--font-ui);
    background: var(--sumi);
    color: var(--paper);
    padding: 12px 28px;
    border-radius: var(--radius);
    border: none;
    cursor: pointer;
    letter-spacing: 0.2px;
    transition: opacity 0.14s;
  }

  .begin-btn:hover {
    opacity: 0.88;
  }

  /* ── Stats bar ────────────────────────────────────────────── */

  .stats-bar {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-4);
    margin-bottom: var(--space-8);
    border-bottom: var(--hairline);
    padding-bottom: var(--space-6);
  }

  .stat {
    text-align: center;
  }

  .stat-value {
    font-family: var(--font-display);
    font-size: 32px;
    color: var(--sumi);
    line-height: 1.2;
  }

  .stat-label {
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    margin-top: var(--space-1);
  }

  /* ── Scan body (two-column) ───────────────────────────────── */

  .scan-body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-6);
    min-height: 0;
  }

  /* ── Project cards ────────────────────────────────────────── */

  .project-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .project-card {
    background: var(--paper-2);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-5);
  }

  .project-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .project-kanji {
    font-size: 18px;
  }

  .project-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--sumi);
  }

  .repo-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .repo-row {
    display: grid;
    grid-template-columns: 1fr 80px auto;
    gap: var(--space-3);
    align-items: center;
    font-size: 12px;
  }

  .repo-name {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--sumi-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mini-bar {
    height: 4px;
    background: var(--paper-3);
    border-radius: 2px;
    overflow: hidden;
  }

  .mini-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.4s ease, background 0.4s ease;
  }

  .repo-status {
    font-size: 10px;
    color: var(--sumi-3);
    text-align: right;
    white-space: nowrap;
  }

  .empty-hint {
    font-size: 13px;
    color: var(--sumi-4);
    padding: var(--space-6);
    text-align: center;
  }

  /* ── Event log ────────────────────────────────────────────── */

  .event-log {
    max-height: 360px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-family: var(--font-mono);
    font-size: 11px;
    padding-right: var(--space-2);
  }

  .log-line {
    display: grid;
    grid-template-columns: 42px 42px 1fr;
    gap: var(--space-2);
    padding: 3px 0;
    line-height: 1.5;
  }

  .log-time {
    color: var(--sumi-4);
    text-align: right;
  }

  .log-level {
    font-weight: 500;
    font-size: 10px;
    letter-spacing: 0.04em;
  }

  .log-msg {
    color: var(--sumi-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
