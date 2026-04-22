<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { WIZ_STAGES, type WizardState, type WizUpdate } from '$lib/setup/types.js';
  import { createMockState } from '$lib/setup/mock.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort, setConfigValue } from '$lib/appstate.svelte.js';
  import {
    Rail, Bottom, Welcome, Components, Assistants,
    Folders, Scan, Projects, Libraries, Registry, Done
  } from '$lib/setup/wizard/index.js';

  // Landing vs wizard state
  let started = $state(false);

  let stageIdx = $state(0);
  let stage = $derived(WIZ_STAGES[stageIdx]);

  let wizardState = $state<WizardState>(createMockState());

  const update: WizUpdate = (patch) => {
    wizardState = { ...wizardState, ...patch };
  };

  const next = () => { stageIdx = Math.min(stageIdx + 1, WIZ_STAGES.length - 1); };
  const back = () => { stageIdx = Math.max(stageIdx - 1, 0); };

  const done = async () => {
    await setConfigValue('setup_complete', '1');
    goto('/overview');
  };

  const exit = () => { goto('/overview'); };

  onMount(async () => {
    try {
      const api = senseiApi(getPort());
      const health = await api.getHealth();
      if (health?.ok) {
        const acps = await api.detectAcps();
        if (acps.length > 0) {
          update({
            acps: Object.fromEntries(acps.map(a => [a.id, a.installed])),
            components: [
              { id: 'cli', name: 'sensei-cli', version: String(health.version ?? ''), status: 'ready', icon: '$' },
              { id: 'mcp', name: 'MCP bridge', version: String(health.version ?? ''), status: 'ready', icon: '↔' },
              { id: 'daemon', name: 'sensei-daemon', version: String(health.version ?? ''), status: 'ready', icon: '◇' },
            ],
          });
        }
      }
    } catch { /* daemon not running */ }
  });
</script>

{#if !started}
  <!-- ═══ Landing: "A quiet empty room" ═══════════════════════════ -->
  <div class="landing">
    <header class="chrome drag-region">
      <div class="traffic no-drag">
        <span class="dot close"></span>
        <span class="dot minimize"></span>
        <span class="dot zoom"></span>
      </div>
      <span class="chrome-title">Sensei 先生</span>
    </header>

    <main class="landing-body">
      <div class="landing-content">
        <div class="landing-left">
          <div class="logo">
            <span class="kanji" style="font-size: 28px; color: var(--shu);">先</span>
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
    <header class="chrome drag-region">
      <div class="traffic no-drag">
        <span class="dot close"></span>
        <span class="dot minimize"></span>
        <span class="dot zoom"></span>
      </div>
      <span class="chrome-title">Sensei 先生 · setup</span>
    </header>

    <div class="body">
      <Rail stages={WIZ_STAGES} currentIndex={stageIdx} onNavigate={(i) => stageIdx = i} onExit={exit} />

      <div class="main">
        <div class="content">
          {#if stage.id === 'welcome'}<Welcome />
          {:else if stage.id === 'components'}<Components wizState={wizardState} {update} />
          {:else if stage.id === 'assistants'}<Assistants wizState={wizardState} {update} />
          {:else if stage.id === 'folders'}<Folders wizState={wizardState} {update} />
          {:else if stage.id === 'scan'}<Scan wizState={wizardState} {update} />
          {:else if stage.id === 'projects'}<Projects wizState={wizardState} {update} />
          {:else if stage.id === 'libraries'}<Libraries wizState={wizardState} {update} />
          {:else if stage.id === 'registry'}<Registry wizState={wizardState} {update} />
          {:else if stage.id === 'done'}<Done wizState={wizardState} />
          {/if}
        </div>

        <Bottom {stage} stageIndex={stageIdx} total={WIZ_STAGES.length}
                onBack={back} onNext={next} onDone={done} />
      </div>
    </div>
  </div>
{/if}

<style>
  /* ── Shared chrome ──────────────────────────────────────── */
  .chrome {
    height: 38px;
    display: flex;
    align-items: center;
    padding: 0 14px;
    border-bottom: var(--hairline);
    background: var(--paper);
    gap: 8px;
    flex-shrink: 0;
  }
  .traffic { display: flex; gap: 7px; }
  .dot {
    width: 12px; height: 12px; border-radius: 50%;
    display: block; background: var(--paper-edge);
  }
  .dot.close { background: oklch(0.72 0.14 28); }
  .dot.minimize { background: oklch(0.82 0.13 85); }
  .dot.zoom { background: oklch(0.72 0.11 145); }
  .chrome-title {
    flex: 1; text-align: center;
    font-size: 12px; color: var(--sumi-3);
    letter-spacing: 0.02em;
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
    background: var(--sumi);
    color: var(--paper);
    border-radius: var(--radius);
    cursor: pointer;
    border: none;
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
    border: var(--hairline);
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
    border-top: var(--hairline);
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
  }
</style>
