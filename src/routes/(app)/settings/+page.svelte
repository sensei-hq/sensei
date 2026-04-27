<script lang="ts">
  import { onMount } from 'svelte';
  import { loadAppState, getPort } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  type Assistant = { family: string; name: string; version?: string; configured: boolean };
  type ConfigEntry = { key: string; value: string };

  let assistants = $state<Assistant[]>([]);
  let config = $state<Record<string, string>>({});
  let extensions = $state<Array<{ name: string; kind: string; enabled: boolean }>>([]);
  let loading = $state(true);
  let section = $state<'general' | 'assistants' | 'inference' | 'extensions'>('general');

  onMount(async () => {
    await loadAppState();
    const api = senseiApi(getPort());
    const [cfg, assts, items] = await Promise.all([
      api.getConfig(),
      api.detectAssistants(),
      api.getInstalledItems(),
    ]);
    config = cfg;
    assistants = (assts as any[]).map(a => ({
      family: a.family ?? a.name,
      name: a.name ?? a.family,
      version: a.version,
      configured: a.configured ?? a.found ?? false,
    }));
    extensions = (items as any[]).map(i => ({
      name: i.name,
      kind: i.kind ?? 'unknown',
      enabled: i.enabled ?? true,
    }));
    loading = false;
  });
</script>

<div class="page">
  <header class="page-header">
    <p class="date-label">Settings</p>
    <h1 class="display page-title">設 Settings</h1>
  </header>

  <!-- Section nav -->
  <div class="section-nav">
    {#each [['general', 'General'], ['assistants', 'Assistants'], ['inference', 'Inference'], ['extensions', 'Extensions']] as [key, label]}
      <button class="section-btn" class:active={section === key} onclick={() => section = key as any}>{label}</button>
    {/each}
  </div>

  {#if loading}
    <p class="hint">Loading settings...</p>
  {:else if section === 'general'}
    <div class="settings-panel">
      <h3 class="panel-title">Preferences</h3>
      <div class="settings-list">
        {#each Object.entries(config) as [key, value]}
          <div class="setting-row">
            <span class="setting-key">{key}</span>
            <span class="setting-value">{value}</span>
          </div>
        {/each}
        {#if Object.keys(config).length === 0}
          <p class="hint">No configuration values set. Preferences from the setup wizard will appear here.</p>
        {/if}
      </div>
    </div>

  {:else if section === 'assistants'}
    <div class="settings-panel">
      <h3 class="panel-title">Assistants</h3>
      <p class="panel-desc">AI coding tools detected on this machine.</p>
      {#if assistants.length === 0}
        <p class="hint">No assistants detected. Run the setup wizard to configure assistants.</p>
      {:else}
        <div class="assistant-list">
          {#each assistants as asst}
            <div class="assistant-row">
              <div class="assistant-info">
                <span class="assistant-name">{asst.name}</span>
                <span class="assistant-family">{asst.family}</span>
              </div>
              {#if asst.version}
                <span class="assistant-version">{asst.version}</span>
              {/if}
              <span class="status-dot" class:configured={asst.configured} class:unconfigured={!asst.configured}></span>
              <span class="assistant-status">{asst.configured ? 'configured' : 'detected'}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

  {:else if section === 'inference'}
    <div class="settings-panel">
      <h3 class="panel-title">Inference</h3>
      <p class="panel-desc">Local and external model configuration.</p>
      <div class="empty-state-inline">
        <span class="kanji" style="font-size: 32px; color: var(--shu); opacity: 0.4;">想</span>
        <p class="hint">Inference configuration will be available once model assignments are supported.</p>
      </div>
    </div>

  {:else if section === 'extensions'}
    <div class="settings-panel">
      <h3 class="panel-title">Extensions</h3>
      <p class="panel-desc">Skills, commands, agents, and hooks installed in sensei.</p>
      {#if extensions.length === 0}
        <p class="hint">No extensions installed yet.</p>
      {:else}
        <div class="extension-list">
          {#each extensions as ext}
            <div class="extension-row">
              <span class="extension-kind">{ext.kind}</span>
              <span class="extension-name">{ext.name}</span>
              <span class="extension-enabled" class:on={ext.enabled}>{ext.enabled ? 'on' : 'off'}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page {
    max-width: 720px;
    margin: 0 auto;
    padding: 48px 48px 64px;
  }
  .page-header { margin-bottom: 24px; }
  .date-label {
    font-size: 10.5px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin: 0 0 8px;
  }
  .page-title { font-size: 24px; font-weight: 400; margin: 0; }

  /* ── Section nav ────────────────────────────────────────── */
  .section-nav {
    display: flex;
    gap: 0;
    border-bottom: var(--hairline);
    margin-bottom: 32px;
  }
  .section-btn {
    padding: 8px 18px;
    border: none;
    background: none;
    color: var(--sumi-3);
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .section-btn:hover { color: var(--sumi-2); }
  .section-btn.active { color: var(--sumi); border-bottom-color: var(--shu); }

  /* ── Panel ──────────────────────────────────────────────── */
  .settings-panel {
    padding: 28px;
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius-lg);
  }
  .panel-title { font-size: 16px; margin: 0 0 4px; }
  .panel-desc { font-size: 13px; color: var(--sumi-3); margin: 0 0 24px; }
  .hint { font-size: 13px; color: var(--sumi-3); line-height: 1.6; }

  /* ── Settings rows ──────────────────────────────────────── */
  .settings-list { display: flex; flex-direction: column; gap: 2px; }
  .setting-row {
    display: flex;
    justify-content: space-between;
    padding: 10px 0;
    border-bottom: var(--ink-line);
  }
  .setting-row:last-child { border-bottom: none; }
  .setting-key { font-size: 13px; color: var(--sumi); font-family: var(--font-mono); }
  .setting-value { font-size: 13px; color: var(--sumi-2); }

  /* ── Assistants ──────────────────────────────────────────── */
  .assistant-list { display: flex; flex-direction: column; gap: 4px; }
  .assistant-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
    border-bottom: var(--ink-line);
  }
  .assistant-row:last-child { border-bottom: none; }
  .assistant-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
  .assistant-name { font-size: 13px; color: var(--sumi); }
  .assistant-family { font-size: 11px; color: var(--sumi-3); }
  .assistant-version { font-size: 12px; color: var(--sumi-3); font-family: var(--font-mono); }
  .status-dot { width: 7px; height: 7px; border-radius: 50%; }
  .status-dot.configured { background: var(--jade); }
  .status-dot.unconfigured { background: var(--amber); }
  .assistant-status { font-size: 11px; color: var(--sumi-3); width: 80px; }

  /* ── Extensions ──────────────────────────────────────────── */
  .extension-list { display: flex; flex-direction: column; gap: 2px; }
  .extension-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: var(--ink-line);
  }
  .extension-row:last-child { border-bottom: none; }
  .extension-kind {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--sumi-4);
    width: 70px;
  }
  .extension-name { font-size: 13px; color: var(--sumi); flex: 1; }
  .extension-enabled { font-size: 11px; color: var(--sumi-3); }
  .extension-enabled.on { color: var(--jade); }

  .empty-state-inline {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 40px 0;
  }
</style>
