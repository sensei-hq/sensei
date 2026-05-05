<script lang="ts">
  import { onDestroy } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import { EventManager } from '$lib/events.js';
  import { ScanProjectState, ScanActivityState } from '$lib/scan-state.svelte.js';
  import type { StateEvent, ScanProject, ActivityEvent } from '$lib/types.js';

  const projects = new ScanProjectState();
  const activities = new ScanActivityState();

  let rootCount = $state(0);
  let started = $state(false);
  let unsub: (() => void) | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const LEVEL_COLORS: Record<string, string> = {
    discover: 'var(--sumi-3)',
    queue: 'var(--amber)',
    process: 'var(--shu)',
    info: 'var(--sumi-2)',
    success: 'var(--jade)',
    error: 'var(--shu)',
  };

  /**
   * Poll /api/index/status until the task queue is idle (pending=0, running=0).
   * This is the reliable signal that all scan+index tasks have finished.
   * We wait 1.5 s before starting to poll to let the daemon enqueue tasks first.
   */
  function startDonePoller(api: ReturnType<typeof senseiApi>) {
    let pollsSinceEnqueue = 0;
    setTimeout(() => {
      pollTimer = setInterval(async () => {
        try {
          const s = await api.getIndexStatus();
          pollsSinceEnqueue++;
          // Need at least 2 consecutive idle polls (debounce transient empty queue)
          if (s.queue.pending === 0 && s.queue.running === 0 && pollsSinceEnqueue >= 2) {
            wizardState.scan.done = true;
            if (pollTimer) clearInterval(pollTimer);
          }
        } catch { /* daemon unreachable — keep polling */ }
      }, 1500);
    }, 1500);
  }

  async function startScan() {
    started = true;
    const api = senseiApi(appState.port);

    // Get root count from scan roots
    const roots = await api.getScanRoots();
    rootCount = roots.length;

    // Connect to SSE
    const events = new EventManager<StateEvent<any>>(
      `http://127.0.0.1:${appState.port}/api/scan/events`,
      (data) => JSON.parse(data),
    );

    unsub = events.subscribe(event => {
      if (event.entity === 'project') projects.apply(event as StateEvent<ScanProject>);
      if (event.entity === 'activity') activities.apply(event as StateEvent<ActivityEvent>);
    });

    // Trigger scan for each root
    for (const root of roots) {
      await api.scanFolder(root.path);
    }

    // Begin polling for task queue idle — marks scan done when complete
    startDonePoller(api);
  }

  onDestroy(() => {
    unsub?.();
    if (pollTimer) clearInterval(pollTimer);
  });
</script>

<div class="scan-page">
  {#if !started}
    <div class="hero-card">
      <div class="kanji hero-kanji">探</div>
      <p class="hero-text">
        The daemon will recurse your folders, identify folders, and extract the code graph.
      </p>
      <button class="btn-solid" onclick={startScan}>Begin scan →</button>
    </div>
  {:else}
    <!-- Stats bar -->
    <div class="stats-bar">
      <div class="stat"><div class="stat-value display">{rootCount}</div><div class="stat-label">ROOTS</div></div>
      <div class="stat"><div class="stat-value display">{activities.discovered}</div><div class="stat-label">DISCOVERED</div></div>
      <div class="stat"><div class="stat-value display">{activities.queued}</div><div class="stat-label">QUEUED</div></div>
      <div class="stat"><div class="stat-value display">{activities.processed}</div><div class="stat-label">PROCESSED</div></div>
    </div>

    <div class="panels">
      <!-- Left: Project cards -->
      <div class="panel-left">
        {#each projects.items as proj (proj.id)}
          {@const path = projects.projectPath(proj)}
          {@const folderCount = proj.folders.length}
          {@const readyCount = proj.folders.filter(f => f.status === 'indexed').length}
          {@const totalFiles = proj.folders.reduce((s, f) => s + f.filesTotal, 0)}
          {@const completedFiles = proj.folders.reduce((s, f) => s + f.filesCompleted, 0)}

          <div class="project-card">
            <div class="project-header">
              <div>
                <div class="project-name">{proj.name}</div>
                <div class="project-meta">{path} · {folderCount} folders · {readyCount} ready</div>
              </div>
              <span class="project-status" class:active={proj.status === 'active'}>{proj.status.toUpperCase()}</span>
            </div>

            {#if totalFiles > 0}
              <div class="project-progress-text">{completedFiles.toLocaleString()} / {totalFiles.toLocaleString()}</div>
            {/if}

            <div class="folder-list">
              {#each proj.folders as f (f.id)}
                <div class="folder-row">
                  <span class="folder-name">{f.name}</span>
                  {#if f.stack.length > 0}
                    <span class="folder-stack">{f.stack.join(', ')}</span>
                  {:else if f.filesTotal > 0}
                    <span class="folder-stack">{f.filesTotal}f</span>
                  {/if}
                  {#if f.filesTotal > 0}
                    <div class="folder-bar">
                      <div class="folder-fill" style="width: {(f.filesCompleted / f.filesTotal) * 100}%"></div>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/each}

        {#if projects.items.length === 0 && started}
          <div class="empty">Discovering projects...</div>
        {/if}
      </div>

      <!-- Right: Activity feed -->
      <div class="panel-right">
        <div class="activity-header">
          <span class="mono">SSE · /EVENTS</span>
          <span class="mono activity-elapsed">{activities.totalElapsed.toFixed(1)}s</span>
        </div>
        <div class="activity-feed">
          {#each activities.recent as evt (evt.id)}
            <div class="activity-row">
              <span class="mono activity-time">+{evt.elapsed.toFixed(2)}s</span>
              <span class="activity-level" style="color: {LEVEL_COLORS[evt.level] ?? 'var(--sumi-3)'}">{evt.level}</span>
              <span class="activity-msg">{evt.message}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .scan-page { max-width: 960px; }

  .hero-card {
    background: var(--paper-2); border-radius: var(--radius-lg);
    padding: var(--space-12) var(--space-10);
    display: flex; flex-direction: column; align-items: center; text-align: center; gap: var(--space-6);
  }
  .hero-kanji { font-size: 80px; color: var(--shu); opacity: 0.25; line-height: 1; }
  .hero-text { font-size: 15px; color: var(--sumi-2); max-width: 440px; line-height: 1.6; margin: 0; }

  .stats-bar {
    display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--space-4);
    margin-bottom: var(--space-6); padding-bottom: var(--space-6); border-bottom: var(--hairline);
  }
  .stat { text-align: center; }
  .stat-value { font-size: 28px; line-height: 1.2; }
  .stat-label { font-size: 10px; letter-spacing: 0.12em; color: var(--sumi-3); margin-top: var(--space-1); }

  .panels { display: grid; grid-template-columns: 1fr 340px; gap: var(--space-6); }

  /* Left: Projects */
  .panel-left { display: flex; flex-direction: column; gap: var(--space-4); }

  .project-card {
    background: var(--paper-2); border-radius: var(--radius-lg); padding: var(--space-5);
    border: var(--border-card);
  }
  .project-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: var(--space-3); }
  .project-name { font-size: 15px; font-weight: 600; }
  .project-meta { font-size: 12px; color: var(--sumi-3); margin-top: 2px; }
  .project-status { font-size: 10px; letter-spacing: 0.1em; color: var(--sumi-3); font-weight: 600; }
  .project-status.active { color: var(--jade); }
  .project-progress-text { font-size: 13px; color: var(--sumi-2); font-family: var(--font-mono); margin-bottom: var(--space-3); }

  .folder-list { display: flex; flex-direction: column; gap: var(--space-2); }
  .folder-row { display: flex; align-items: center; gap: var(--space-3); }
  .folder-name { font-size: 13px; font-weight: 500; min-width: 120px; }
  .folder-stack { font-size: 11px; color: var(--sumi-3); min-width: 80px; }
  .folder-bar { flex: 1; height: 4px; background: var(--paper-3); border-radius: 2px; overflow: hidden; }
  .folder-fill { height: 100%; background: var(--amber); border-radius: 2px; transition: width 0.3s; }

  .empty { font-size: 13px; color: var(--sumi-4); padding: var(--space-6); text-align: center; }

  /* Right: Activity */
  .panel-right {
    background: var(--paper-2); border-radius: var(--radius-lg); padding: var(--space-4);
    border: var(--border-card); max-height: 600px; display: flex; flex-direction: column;
  }
  .activity-header {
    display: flex; justify-content: space-between; padding-bottom: var(--space-3);
    border-bottom: var(--ink-line); margin-bottom: var(--space-3);
    font-size: 11px; color: var(--sumi-3);
  }
  .activity-elapsed { color: var(--sumi-2); }
  .activity-feed { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 2px; }
  .activity-row { display: flex; gap: var(--space-3); font-size: 11px; padding: 2px 0; align-items: baseline; }
  .activity-time { color: var(--sumi-4); min-width: 52px; text-align: right; }
  .activity-level { font-weight: 500; min-width: 56px; }
  .activity-msg { color: var(--sumi-2); flex: 1; }
</style>
