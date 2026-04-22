<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { WIZ_STAGES, type WizardState, type WizUpdate } from '$lib/setup/types.js';
  import { createMockState, MOCK_ACPS } from '$lib/setup/mock.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort, setConfigValue } from '$lib/appstate.svelte.js';
  import {
    Rail, Bottom, Welcome, Components, Assistants,
    Folders, Scan, Projects, Libraries, Registry, Done
  } from '$lib/setup/wizard/index.js';

  let stageIdx = $state(0);
  let stage = $derived(WIZ_STAGES[stageIdx]);

  // Start with mock data; onMount tries to hydrate from daemon
  let state = $state<WizardState>(createMockState());

  const update: WizUpdate = (patch) => {
    state = { ...state, ...patch };
  };

  const next = () => { stageIdx = Math.min(stageIdx + 1, WIZ_STAGES.length - 1); };
  const back = () => { stageIdx = Math.max(stageIdx - 1, 0); };

  const done = async () => {
    await setConfigValue('setup_complete', '1');
    goto('/overview');
  };

  const exit = () => { goto('/overview'); };

  // Try to hydrate from real daemon
  onMount(async () => {
    try {
      const api = senseiApi(getPort());
      const health = await api.getHealth();
      if (health?.ok) {
        // Daemon is running — try to get real ACP data
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
    } catch {
      // Daemon not running — use mock data
    }
  });
</script>

<div class="wizard">
  <header class="chrome drag-region">
    <div class="traffic no-drag">
      <span class="dot close"></span>
      <span class="dot minimize"></span>
      <span class="dot zoom"></span>
    </div>
    <span class="title">Sensei 先生 · setup</span>
  </header>

  <div class="body">
    <Rail stages={WIZ_STAGES} currentIndex={stageIdx} onNavigate={(i) => stageIdx = i} onExit={exit} />

    <div class="main">
      <div class="content">
        {#if stage.id === 'welcome'}<Welcome />
        {:else if stage.id === 'components'}<Components wizState={state} {update} />
        {:else if stage.id === 'assistants'}<Assistants wizState={state} {update} />
        {:else if stage.id === 'folders'}<Folders wizState={state} {update} />
        {:else if stage.id === 'scan'}<Scan wizState={state} {update} />
        {:else if stage.id === 'projects'}<Projects wizState={state} {update} />
        {:else if stage.id === 'libraries'}<Libraries wizState={state} {update} />
        {:else if stage.id === 'registry'}<Registry wizState={state} {update} />
        {:else if stage.id === 'done'}<Done wizState={state} />
        {/if}
      </div>

      <Bottom {stage} stageIndex={stageIdx} total={WIZ_STAGES.length}
              onBack={back} onNext={next} onDone={done} />
    </div>
  </div>
</div>

<style>
  .wizard {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--paper);
    overflow: hidden;
    font-family: var(--font-ui);
    color: var(--sumi);
  }

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

  .title {
    flex: 1; text-align: center;
    font-size: 12px; color: var(--sumi-3);
    letter-spacing: 0.02em;
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
