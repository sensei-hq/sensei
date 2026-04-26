<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { WIZ_STAGES, type WizardState, type WizUpdate } from '$lib/setup/types.js';
  import { createEmptyState, MOCK_LIBRARIES, MOCK_MCPS, MOCK_STACK } from '$lib/setup/mock.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort, setConfigValue, loadAppState } from '$lib/appstate.svelte.js';
  import {
    Rail, Bottom, Welcome, Assistants,
    Folders, Scan, Projects, Libraries, Registry, Done
  } from '$lib/setup/wizard/index.js';

  // Watermark kanji per stage — faded background character
  const STAGE_KANJI: Record<string, string> = {
    welcome: '先', components: '支', assistants: '助', folders: '径',
    scan: '探', projects: '場', libraries: '庫', registry: '具',
    inference: '推', assignments: '配', done: '観',
  };

  // Landing vs wizard state
  let started = $state(false);
  let daemonReady = $state(false);
  let daemonPort = $state(7744);

  let stageIdx = $state(0);
  let stage = $derived(WIZ_STAGES[stageIdx]);

  // Start empty — daemon populates on mount
  let wizardState = $state<WizardState>(createEmptyState());

  const update: WizUpdate = (patch) => {
    wizardState = { ...wizardState, ...patch };
  };

  const next = () => { stageIdx = Math.min(stageIdx + 1, WIZ_STAGES.length - 1); };
  const back = () => { stageIdx = Math.max(stageIdx - 1, 0); };

  const done = async () => {
    await setConfigValue('setup_complete', '1');
    goto('/observatory');
  };

  const exit = () => { goto('/observatory'); };

  // Hydrate from daemon on mount
  onMount(async () => {
    await loadAppState();
    daemonPort = getPort();
    const api = senseiApi(daemonPort);

    try {
      const health = await api.getHealth();
      if (!health?.ok) throw new Error('daemon not ready');
      daemonReady = true;

      // Assistant detection (grouped by family)
      const families = await api.detectAssistantFamilies();
      if (families.length > 0) {
        const assistantList = families.map(f => ({
          id: f.family, name: f.name, version: null,
          found: f.installed, path: f.config_path ?? null,
        }));
        update({
          assistants: Object.fromEntries(families.map(f => [f.family, f.installed])),
          assistantList,
        });
      }

      // Step 7-8: Libraries and MCP Registry (mock for now — daemon doesn't have these yet)
      update({
        libraries: Object.fromEntries(MOCK_LIBRARIES.map(l => [l.id, true])),
        mcps: Object.fromEntries(MOCK_MCPS.map(m => [m.id, m.installed || m.recommended])),
        detectedStack: MOCK_STACK,
      });
    } catch {
      // Daemon not running — show empty state, user needs to start daemon
      daemonReady = false;
    }
  });

  // When scan completes (step 5), load discovered repos and build project suggestions
  async function onScanComplete() {
    if (!daemonReady) return;
    try {
      // Fetch discovered repos
      const repos = await api.getRepos();
      if (repos.length === 0) return;

      // Fetch project grouping suggestions
      const suggestions = await api.getScanSuggestions();

      // Build project cards from suggestions + standalone repos
      const assignedRepoIds = new Set(suggestions.flatMap((s: any) => s.repo_ids));
      const projects: any[] = [];

      for (const s of suggestions) {
        const projRepos = repos
          .filter(r => s.repo_ids.includes(r.repoId))
          .map(r => ({
            id: r.repoId, name: r.name, path: r.path,
            files: 0, lang: (r as any).stack?.join(', ') || '',
            suggestedRole: (r as any).role || 'unknown',
          }));
        projects.push({
          id: `auto:${s.name}`, name: s.name, kanji: '工',
          path: '', autoDetected: true, confidence: 'high' as const,
          repos: projRepos, confirmed: true,
        });
      }

      // Standalone repos (not in any suggestion)
      for (const r of repos) {
        if (!assignedRepoIds.has(r.repoId)) {
          projects.push({
            id: `auto:${r.repoId}`, name: r.name, kanji: '一',
            path: r.path, autoDetected: true, confidence: 'medium' as const,
            repos: [{ id: r.repoId, name: r.name, path: r.path, files: 0, lang: '', suggestedRole: 'unknown' }],
            confirmed: true,
          });
        }
      }

      update({
        projects,
        roles: Object.fromEntries(
          projects.flatMap((p: any) => p.repos.map((r: any) => [r.id, r.suggestedRole]))
        ),
        scanDone: true,
      });
    } catch { /* non-fatal */ }
  }
</script>

{#if !started}
  <!-- ═══ Landing: "A quiet empty room" ═══════════════════════════ -->
  <div class="landing">
    <!-- Drag spacer for macOS traffic lights — no rendered chrome -->
    <div class="drag-spacer drag-region"></div>

    <main class="landing-body">
      <div class="landing-content">
        <div class="landing-left">
          <div class="logo">
            <span class="kanji" style="font-size: 24px; color: var(--shu);">先生</span>
            <span class="display" style="font-size: 18px;">Sensei</span>
          </div>

          <p class="landing-label">WELCOME</p>

          <h1 class="display landing-hero">
            A quiet<br>
            <span style="color: var(--shu);">empty room.</span>
          </h1>

          <p class="landing-desc">
            Point sensei at your folders and keep working.
            It watches in silence, learns the shape of each project,
            and later begins to teach.
          </p>

          <button class="begin-btn no-drag" onclick={() => started = true}>
            Begin setup →
          </button>

          <p class="landing-meta">~4 minutes · nothing leaves your machine</p>
        </div>

        <div class="landing-right">
          <div class="info-card">
            <p class="info-label">WHAT SENSEI DOES</p>

            <div class="info-row">
              <span class="kanji info-kanji">観</span>
              <div>
                <p class="info-title">Watches</p>
                <p class="info-text">Every assistant session — prompts, tool calls, diffs.</p>
              </div>
            </div>

            <div class="info-row">
              <span class="kanji info-kanji">師</span>
              <div>
                <p class="info-title">Notices</p>
                <p class="info-text">Which prompts work, which patterns repeat, where you rework.</p>
              </div>
            </div>

            <div class="info-row">
              <span class="kanji info-kanji">教</span>
              <div>
                <p class="info-title">Teaches</p>
                <p class="info-text">After ~3 sessions per project, offers concrete suggestions.</p>
              </div>
            </div>

            <p class="info-footer">
              Works with <span class="mono">claude-code</span>, <span class="mono">cursor</span>, <span class="mono">codex</span>, <span class="mono">aider</span>.
            </p>
          </div>
        </div>
      </div>
    </main>
  </div>

{:else}
  <!-- ═══ Wizard ══════════════════════════════════════════════════ -->
  <div class="wizard">
    <!-- Drag spacer for macOS traffic lights -->
    <div class="drag-spacer drag-region"></div>

    <div class="body">
      <Rail stages={WIZ_STAGES} currentIndex={stageIdx} onNavigate={(i) => stageIdx = i} onExit={exit} />

      <div class="main">
        <div class="content">
          {#if stage.watermark}
            <span class="watermark kanji">{STAGE_KANJI[stage.id] ?? ''}</span>
          {/if}
          <div class="content-inner">
            {#if stage.id === 'welcome'}<Welcome />
            {:else if stage.id === 'assistants'}<Assistants wizState={wizardState} {update} {stage} />
            {:else if stage.id === 'folders'}<Folders wizState={wizardState} {update} {stage} />
            {:else if stage.id === 'scan'}<Scan wizState={wizardState} {update} onScan={onScanComplete} {daemonReady} />
            {:else if stage.id === 'projects'}<Projects wizState={wizardState} {update} {stage} />
            {:else if stage.id === 'libraries'}<Libraries wizState={wizardState} {update} {stage} />
            {:else if stage.id === 'instruments'}<Registry wizState={wizardState} {update} {stage} />
            {:else if stage.id === 'done'}<Done wizState={wizardState} />
            {/if}
          </div>
        </div>

        <Bottom {stage} stageIndex={stageIdx} total={WIZ_STAGES.length}
                onBack={back} onNext={next} onDone={done} />
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Drag spacer (macOS traffic light safe area) ─────────── */
  .drag-spacer {
    height: 32px;
    flex-shrink: 0;
  }

  /* ── Landing page ───────────────────────────────────────── */
  .landing {
    width: 100%; height: 100vh;
    display: flex; flex-direction: column;
    background: var(--paper);
    font-family: var(--font-ui);
    color: var(--sumi);
    overflow: hidden;
  }
  .landing-body {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 64px;
  }
  .landing-content {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 80px;
    max-width: 900px;
    width: 100%;
    align-items: center;
  }

  .logo {
    display: flex; align-items: baseline; gap: 8px;
    margin-bottom: 32px;
  }

  .landing-label {
    font-size: 10px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin-bottom: 12px;
  }
  .landing-hero {
    font-size: 46px;
    font-weight: 400;
    line-height: 1.1;
    margin: 0 0 24px;
  }
  .landing-desc {
    font-size: 14px;
    line-height: 1.7;
    color: var(--sumi-2);
    margin-bottom: 36px;
    max-width: 360px;
  }
  .begin-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 14px 28px;
    font-size: 14px;
    font-weight: 500;
    background: var(--paper-3);
    color: var(--sumi);
    border-radius: var(--radius);
    cursor: pointer;
    border: var(--border-card);
    transition: opacity 0.15s;
  }
  .begin-btn:hover { opacity: 0.85; }

  .landing-meta {
    margin-top: 16px;
    font-size: 12px;
    color: var(--sumi-4);
  }

  /* Info card */
  .info-card {
    background: var(--paper-2);
    border-radius: var(--radius-lg);
    padding: 28px 28px 20px;
    border: var(--border-card);
  }
  .info-label {
    font-size: 9px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin-bottom: 24px;
  }
  .info-row {
    display: flex;
    gap: 14px;
    margin-bottom: 20px;
    align-items: flex-start;
  }
  .info-kanji {
    font-size: 22px;
    color: var(--shu);
    flex-shrink: 0;
    margin-top: 2px;
  }
  .info-title {
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 3px;
  }
  .info-text {
    font-size: 12px;
    line-height: 1.5;
    color: var(--sumi-2);
  }
  .info-footer {
    margin-top: 20px;
    padding-top: 16px;
    border-top: var(--ink-line);
    font-size: 11px;
    color: var(--sumi-3);
  }

  /* ── Wizard ─────────────────────────────────────────────── */
  .wizard {
    width: 100%; height: 100vh;
    display: flex; flex-direction: column;
    background: var(--paper);
    font-family: var(--font-ui);
    color: var(--sumi);
    overflow: hidden;
  }
  .body {
    flex: 1;
    display: grid;
    grid-template-columns: 260px 1fr;
    min-height: 0;
  }
  .main {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: 44px 64px 32px;
    position: relative;
  }
  .content-inner {
    position: relative;
    z-index: 1;
  }
  .watermark {
    position: absolute;
    right: 64px;
    bottom: 32px;
    font-size: 240px;
    color: var(--shu);
    opacity: 0.035;
    line-height: 1;
    user-select: none;
    pointer-events: none;
    z-index: 0;
  }
</style>
