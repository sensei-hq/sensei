<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { hasTauri, installPrerequisites, getPlatform, listenBootstrapEvents } from '$lib/bootstrap.js';
  import { bootstrapState as bs } from '$lib/bootstrap-state.svelte.js';
  import { GATES } from '$lib/bootstrap-gates.js';
  import type { GateStatus } from '$lib/bootstrap-gates.js';

  // Browser mode: apply mock preset
  if (!hasTauri()) {
    bs.applyPreset({
      homebrew: 'ready', postgres: 'missing', ollama: 'missing',
      sensei: 'missing', database: 'pending', senseid: 'pending',
    });
  }

  // Auto-advance when all ready
  $effect(() => {
    if (bs.allReady) {
      setTimeout(() => {
        if (appState.setupComplete) goto('/observatory', { replaceState: true });
        else goto('/setup/welcome', { replaceState: true });
      }, 900);
    }
  });

  // Wire Tauri events → state (via handleEvent)
  onMount(async () => {
    if (!hasTauri()) return;
    try {
      const info = await getPlatform();
      bs.setPlatform(info);
    } catch { /* browser fallback */ }
    const unlisten = await listenBootstrapEvents((event) => bs.handleEvent(event));
    return () => unlisten();
  });

  // Browser-mode retry simulation
  function retry(gateId: string) {
    bs.setGateStatus(gateId, 'checking');
    if (!hasTauri()) {
      setTimeout(() => {
        bs.setGateStatus(gateId, 'ready');
        const idx = GATES.findIndex(g => g.id === gateId);
        if (idx + 1 < GATES.length && bs.statuses[GATES[idx + 1].id] === 'pending') {
          bs.setGateStatus(GATES[idx + 1].id, 'checking');
          setTimeout(() => {
            GATES.slice(idx + 1).forEach(g => bs.setGateStatus(g.id, 'ready'));
          }, 900);
        }
      }, 1100);
    }
  }

  async function runInstallPrereqs() {
    if (!hasTauri()) return;
    bs.installing = true;
    await installPrerequisites();
  }

  function retryAll() {
    bs.missingPrereqGates.forEach(g => retry(g.id));
  }

  function statusColor(s: GateStatus): string {
    if (s === 'ready') return 'var(--jade)';
    if (s === 'missing' || s === 'error') return 'var(--shu)';
    if (s === 'checking' || s === 'starting') return 'var(--sumi-2)';
    return 'var(--sumi-4)';
  }

  function pillBg(s: GateStatus): string {
    if (s === 'ready') return 'rgba(122,158,98,.10)';
    if (s === 'missing' || s === 'error') return 'rgba(192,71,45,.08)';
    if (s === 'checking' || s === 'starting') return 'var(--paper-2)';
    return 'transparent';
  }

  function pillLabel(s: GateStatus): string {
    const map: Record<string, string> = {
      ready: 'ready', checking: 'checking', starting: 'starting',
      missing: 'missing', error: 'blocked', pending: 'waiting',
    };
    return map[s] ?? 'waiting';
  }
</script>

<div class="bootstrap-page">
  <!-- Fixed top: header + progress rail -->
  <div class="fixed-top">
    <div class="content-top">
      <!-- Header -->
      <div class="header">
        <div class="header-tag">
          <span class="kanji" style="font-size: 22px; color: var(--shu);">支</span>
          <span class="tag-text">bootstrap · checking the foundation</span>
        </div>
        <h1 class="display header-title">
          {#if bs.allReady}
            The foundation <span style="color: var(--jade);">holds.</span>
          {:else if bs.firstBlockedIdx >= 0}
            A few pieces are <span style="color: var(--shu);">missing.</span>
          {:else}
            Checking the foundation…
          {/if}
        </h1>
        <p class="header-desc">
          {#if bs.allReady}
            Homebrew, Postgres, Ollama, sensei components, database, and the daemon are all present. Opening the observatory.
          {:else if bs.firstBlockedIdx >= 0}
            Sensei needs these to run locally. Install the missing pieces below — the rest will check themselves once the foundation is in place.
          {:else}
            Verifying Homebrew, Postgres, Ollama, and the sensei components. This takes a few seconds on a cold start.
          {/if}
        </p>
      </div>

      <!-- Progress rail -->
      <div class="progress-rail">
        <div class="progress-count">
          {String(bs.readyCount).padStart(2, '0')} / {String(bs.totalCount).padStart(2, '0')} ready
        </div>
        <div class="progress-bars">
          {#each bs.gates as gate}
            <span class="progress-segment" style="background: {statusColor(gate.status)}; opacity: {gate.status === 'pending' ? 0.5 : 1};"></span>
          {/each}
        </div>
      </div>
    </div>
  </div>

  <!-- Scrollable bottom: gate list + footer -->
  <div class="scroll-area">
    <div class="content-bottom">
    <!-- Gate list -->
    <div class="gate-list">
      {#each bs.visibleGates as gate, i (gate.id)}
        {@const isBlocked = gate.status === 'missing' || gate.status === 'error'}
        {@const isBusy = gate.status === 'checking' || gate.status === 'starting'}
        {@const isReady = gate.status === 'ready'}
        {@const isPending = gate.status === 'pending'}
        {@const showRemedy = i === bs.firstBlockedIdx && isBlocked}

        <div class="gate-row" class:pending={isPending}>
          <!-- Main row -->
          <div class="gate-main">
            <div class="kanji gate-n" style="color: {statusColor(gate.status)};">{gate.n}</div>
            <div class="gate-info">
              <div class="gate-name-row">
                <span class="display gate-name">{gate.name}</span>
                <span class="gate-detail">· {gate.detail}</span>
              </div>
              <div class="mono gate-check">{gate.check}</div>
            </div>
            <div class="status-pill" style="color: {statusColor(gate.status)}; background: {pillBg(gate.status)};">
              {#if isBusy}<span class="spinner"></span>{/if}
              {#if isReady}<span style="font-size: 10px;">✓</span>{/if}
              {#if isBlocked}<span style="font-size: 12px;">·</span>{/if}
              {pillLabel(gate.status)}
            </div>
          </div>

          <!-- Sub-checks (sensei components) -->
          {#if gate.sub && (isBusy || isBlocked || isReady)}
            <div class="sub-checks">
              {#each gate.sub as sub}
                {@const subStatus = isReady ? 'ready' : isBusy ? 'checking' : 'missing'}
                <div class="sub-row">
                  <span class="sub-dot" style="background: {statusColor(subStatus)};"></span>
                  <span class="sub-name">{sub.name}</span>
                  <span class="mono sub-check">{sub.check}</span>
                </div>
              {/each}
            </div>
          {/if}

          <!-- Per-gate remedy (non-prereq only) -->
          {#if showRemedy && gate.remedy !== 'prereq'}
            <div class="remedy">
              {#if gate.remedy === 'install'}
                <div class="display remedy-title">{bs.platformInfo.pkgmgr_remedy.title}</div>
                <p class="remedy-intro">{bs.platformInfo.package_manager} is the base that installs everything else.</p>
                <div class="command-block">
                  <code>{bs.platformInfo.pkgmgr_remedy.command}</code>
                </div>
                <div class="remedy-actions">
                  {#if bs.platformInfo.pkgmgr_remedy.url}
                    <a href={bs.platformInfo.pkgmgr_remedy.url} target="_blank" rel="noreferrer" class="btn-outline btn-sm">Learn more <span style="color: var(--sumi-3);">↗</span></a>
                  {/if}
                  <button class="btn-solid btn-sm" onclick={() => retry(gate.id)}>I've installed it — retry</button>
                </div>

              {:else if gate.remedy === 'db'}
                <div class="display remedy-title">Could not create the sensei database</div>
                <p class="remedy-intro">Postgres is running but sensei couldn't create its database automatically.</p>
                <div class="command-block">
                  <code>createdb sensei && psql sensei -c 'CREATE EXTENSION IF NOT EXISTS vector;'</code>
                </div>
                <div class="remedy-actions">
                  <button class="btn-solid btn-sm" onclick={() => retry(gate.id)}>Retry</button>
                </div>

              {:else if gate.remedy === 'daemon'}
                <div class="display remedy-title">Daemon failed to start</div>
                <p class="remedy-intro">The database is reachable but the daemon did not come up.</p>
                <div class="remedy-actions">
                  <button class="btn-solid btn-sm" onclick={() => retry(gate.id)}>Retry</button>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Consolidated prereq remedy -->
    {#if bs.needsPrereqInstall}
      <div class="brew-remedy">
        <div class="display remedy-title">{bs.platformInfo.prereq_remedy.title}</div>
        <div class="missing-list">
          {#each bs.missingPrereqGates as gate}
            <span class="missing-tag">{gate.name}</span>
          {/each}
        </div>
        <p class="remedy-intro">
          One command installs everything. Already-installed items are skipped.
        </p>
        {#if hasTauri()}
          <div class="remedy-actions">
            <button class="btn-solid btn-sm" onclick={runInstallPrereqs} disabled={bs.installing}>
              {bs.installing ? 'Installing…' : 'Install all'}
            </button>
            <button class="btn-outline btn-sm" onclick={retryAll}>Retry checks</button>
          </div>
        {:else}
          <div class="command-block">
            <code>{bs.platformInfo.prereq_remedy.command}</code>
          </div>
          <div class="remedy-actions">
            <button class="btn-solid btn-sm" onclick={retryAll}>Retry checks</button>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Footer -->
    <div class="footer">
      <div class="footer-note">
        Bootstrap runs on every launch. Once a gate is green it stays that way — the next startup is quick.
      </div>
      {#if bs.allReady}
        <button class="btn-solid" onclick={() => goto('/setup/welcome')}>Continue →</button>
      {/if}
    </div>
    </div>
  </div>
</div>

<style>
  .bootstrap-page {
    flex: 1; min-height: 0; overflow: hidden;
    background: var(--paper); color: var(--sumi);
    font-family: var(--font-ui);
    display: flex; flex-direction: column;
  }
  .fixed-top {
    flex-shrink: 0;
    background: var(--paper);
    padding: 56px 40px 0;
  }
  .content-top {
    max-width: 760px; width: 100%;
    margin: 0 auto;
    display: flex; flex-direction: column; gap: 40px;
    padding-bottom: 40px;
  }
  .scroll-area {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 0 40px 48px;
  }
  .content-bottom {
    max-width: 760px; width: 100%;
    margin: 0 auto;
  }

  /* Header */
  .header-tag { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
  .tag-text { font-size: 11px; letter-spacing: 0.14em; text-transform: uppercase; color: var(--sumi-3); }
  .header-title { font-size: 38px; font-weight: 300; line-height: 1.12; margin: 0 0 14px; letter-spacing: -0.015em; }
  .header-desc { font-size: 14px; color: var(--sumi-3); line-height: 1.7; margin: 0; max-width: 540px; }

  /* Progress rail */
  .progress-rail { display: flex; align-items: center; gap: 12px; }
  .progress-count { font-size: 10px; letter-spacing: 0.14em; text-transform: uppercase; color: var(--sumi-4); font-feature-settings: "tnum"; white-space: nowrap; }
  .progress-bars { flex: 1; display: flex; gap: 3px; }
  .progress-segment { flex: 1; height: 2px; border-radius: 1px; transition: background 0.3s; }

  /* Gate list */
  .gate-list { display: flex; flex-direction: column; border-top: var(--hairline); }

  .gate-row { border-bottom: var(--hairline); padding: 16px 0; transition: opacity 0.3s; }
  .gate-row.pending { opacity: 0.42; }

  .gate-main { display: grid; grid-template-columns: 32px 1fr auto; gap: 16px; align-items: center; }
  .gate-n { font-size: 22px; text-align: center; }
  .gate-name-row { display: flex; align-items: baseline; gap: 10px; }
  .gate-name { font-size: 17px; font-weight: 400; }
  .gate-detail { font-size: 12px; color: var(--sumi-4); }
  .gate-check { font-size: 11px; color: var(--sumi-4); margin-top: 4px; }

  /* Status pill */
  .status-pill {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 4px 10px; border-radius: 4px;
    font-size: 11px; letter-spacing: 0.08em; text-transform: uppercase;
    font-feature-settings: "tnum";
  }

  /* Spinner */
  .spinner {
    display: inline-block; width: 10px; height: 10px; position: relative;
  }
  .spinner::after {
    content: ''; position: absolute; top: 0; left: 0; right: 0; bottom: 0;
    border: 1.5px solid currentColor; border-top-color: transparent;
    border-radius: 50%; animation: spin 0.9s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* Sub-checks */
  .sub-checks {
    margin-top: 12px; margin-left: 48px;
    display: flex; flex-direction: column; gap: 4px;
    padding-left: 14px; border-left: 1px dashed var(--paper-edge);
  }
  .sub-row { display: flex; align-items: center; gap: 10px; }
  .sub-dot { width: 6px; height: 6px; border-radius: 3px; display: inline-block; }
  .sub-name { font-size: 12px; color: var(--sumi-2); }
  .sub-check { font-size: 11px; color: var(--sumi-4); }

  /* Consolidated brew remedy */
  .brew-remedy {
    margin-top: 24px; padding: 20px 24px;
    background: var(--paper-2); border: var(--hairline); border-radius: 6px;
  }
  .missing-list {
    display: flex; gap: 6px; flex-wrap: wrap;
    margin: 8px 0 12px;
  }
  .missing-tag {
    font-size: 11px; letter-spacing: 0.06em; text-transform: uppercase;
    color: var(--shu); background: rgba(192,71,45,.08);
    padding: 3px 8px; border-radius: 3px;
  }

  /* Per-gate remedy */
  .remedy {
    margin-top: 16px; margin-left: 48px; padding: 18px 20px;
    background: var(--paper-2); border: var(--hairline); border-radius: 6px;
  }
  .remedy-title { font-size: 15px; margin-bottom: 4px; }
  .remedy-intro { font-size: 13px; color: var(--sumi-3); line-height: 1.6; margin: 0 0 14px; }

  .command-block {
    background: var(--paper); border: var(--hairline); border-radius: 5px;
    padding: 10px 12px; margin-bottom: 12px;
  }
  .command-block code {
    font-family: var(--font-mono); font-size: 12px; color: var(--sumi);
    word-break: break-all;
  }

  .remedy-actions { display: flex; gap: 10px; align-items: center; }

  /* Footer */
  .footer {
    display: flex; justify-content: space-between; align-items: center; gap: 16px;
    padding-top: 22px; border-top: var(--hairline);
  }
  .footer-note { font-size: 11px; color: var(--sumi-4); line-height: 1.6; }

  /* Button overrides for small size */
  .btn-sm { padding: 8px 14px !important; font-size: 12px !important; }
</style>
