<script lang="ts">
  import { onMount } from 'svelte';
  import { loadAppState, getPort } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  type Tool = { name: string; description: string; params: string[] };

  let tools = $state<Tool[]>([]);
  let loading = $state(true);
  let tab = $state<'playground' | 'replay' | 'insights'>('playground');
  let selectedTool = $state<Tool | null>(null);
  let toolResult = $state<string>('');
  let toolParams = $state<Record<string, string>>({});
  let executing = $state(false);

  onMount(async () => {
    await loadAppState();
    const api = senseiApi(getPort());
    const data = await api.mcpListTools();
    tools = data.tools;
    loading = false;
  });

  async function executeTool() {
    if (!selectedTool) return;
    executing = true;
    toolResult = '';
    const api = senseiApi(getPort());
    const result = await api.mcpCallTool(selectedTool.name, toolParams);
    toolResult = JSON.stringify(result, null, 2);
    executing = false;
  }
</script>

<div class="page">
  <header class="page-header">
    <p class="date-label">Instruments</p>
    <h1 class="display page-title">具 Instruments</h1>
  </header>

  <!-- Tab bar -->
  <div class="tab-bar">
    <button class="tab" class:active={tab === 'playground'} onclick={() => tab = 'playground'}>
      <span class="kanji tab-kanji">具</span> Playground
    </button>
    <button class="tab" class:active={tab === 'replay'} onclick={() => tab = 'replay'}>
      <span class="kanji tab-kanji">録</span> Replay
    </button>
    <button class="tab" class:active={tab === 'insights'} onclick={() => tab = 'insights'}>
      <span class="kanji tab-kanji">照</span> Insights
    </button>
  </div>

  {#if tab === 'playground'}
    {#if loading}
      <p class="hint">Loading tools...</p>
    {:else if tools.length === 0}
      <div class="empty-state">
        <span class="kanji empty-kanji">具</span>
        <p class="display empty-title">No MCP tools available.</p>
        <p class="empty-body">
          Tools appear when the sensei daemon is running and MCP services are configured.
          Check your instruments in the setup wizard.
        </p>
      </div>
    {:else}
      <div class="playground-grid">
        <!-- Tool list -->
        <div class="tool-list">
          {#each tools as tool (tool.name)}
            <button
              class="tool-card"
              class:selected={selectedTool?.name === tool.name}
              onclick={() => { selectedTool = tool; toolParams = {}; toolResult = ''; }}
            >
              <span class="tool-name">{tool.name}</span>
              <span class="tool-desc">{tool.description}</span>
            </button>
          {/each}
        </div>

        <!-- Tool detail + execution -->
        <div class="tool-detail">
          {#if selectedTool}
            <h3 class="tool-detail-name">{selectedTool.name}</h3>
            <p class="tool-detail-desc">{selectedTool.description}</p>

            {#if selectedTool.params.length > 0}
              <div class="params-form">
                {#each selectedTool.params as param}
                  <div class="param-field">
                    <label class="param-label" for="param-{param}">{param}</label>
                    <input
                      id="param-{param}"
                      class="param-input"
                      type="text"
                      placeholder={param}
                      bind:value={toolParams[param]}
                    />
                  </div>
                {/each}
              </div>
            {/if}

            <button class="btn-solid" onclick={executeTool} disabled={executing}>
              {executing ? 'Running...' : 'Execute'}
            </button>

            {#if toolResult}
              <div class="result-panel">
                <p class="result-label">Response</p>
                <pre class="result-code">{toolResult}</pre>
              </div>
            {/if}
          {:else}
            <p class="hint">Select a tool to try it.</p>
          {/if}
        </div>
      </div>
    {/if}
  {:else if tab === 'replay'}
    <div class="empty-state">
      <span class="kanji empty-kanji">録</span>
      <p class="display empty-title">Session replay</p>
      <p class="empty-body">
        Tool calls from your assistant sessions will appear here.
        Each call shows the tool, arguments, response, and whether the assistant used the result.
      </p>
    </div>
  {:else}
    <div class="empty-state">
      <span class="kanji empty-kanji">照</span>
      <p class="display empty-title">Tool insights</p>
      <p class="empty-body">
        Aggregated usage and effectiveness metrics across sessions.
        Which tools are used most, which responses get ignored, and where tool usage correlates with FTR.
      </p>
    </div>
  {/if}
</div>

<style>
  .page {
    max-width: 960px;
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

  /* ── Tab bar ────────────────────────────────────────────── */
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: var(--hairline);
    margin-bottom: 28px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 18px;
    border: none;
    background: none;
    color: var(--sumi-3);
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .tab:hover { color: var(--sumi-2); }
  .tab.active { color: var(--sumi); border-bottom-color: var(--shu); }
  .tab-kanji { font-size: 12px; }

  /* ── Empty state ────────────────────────────────────────── */
  .empty-state { text-align: center; padding: 80px 20px; }
  .empty-kanji { font-size: 64px; color: var(--shu); opacity: 0.3; }
  .empty-title { font-size: 20px; font-weight: 400; margin: 16px 0 8px; }
  .empty-body { font-size: 13px; color: var(--sumi-3); max-width: 380px; margin: 0 auto; line-height: 1.65; }

  /* ── Playground grid ────────────────────────────────────── */
  .playground-grid { display: grid; grid-template-columns: 260px 1fr; gap: 24px; }
  .tool-list { display: flex; flex-direction: column; gap: 2px; }
  .tool-card {
    text-align: left;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 0.1s;
  }
  .tool-card:hover { background: var(--paper-2); }
  .tool-card.selected { background: var(--paper-2); }
  .tool-name { display: block; font-size: 13px; font-weight: 500; color: var(--sumi); font-family: var(--font-mono); }
  .tool-desc { display: block; font-size: 11px; color: var(--sumi-3); margin-top: 2px; }

  /* ── Tool detail ────────────────────────────────────────── */
  .tool-detail {
    padding: 24px;
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius-lg);
  }
  .tool-detail-name { font-size: 16px; font-family: var(--font-mono); margin: 0 0 6px; }
  .tool-detail-desc { font-size: 13px; color: var(--sumi-2); margin: 0 0 20px; line-height: 1.5; }
  .params-form { display: flex; flex-direction: column; gap: 12px; margin-bottom: 20px; }
  .param-field { display: flex; flex-direction: column; gap: 4px; }
  .param-label { font-size: 11px; color: var(--sumi-3); font-family: var(--font-mono); }
  .param-input {
    padding: 8px 12px;
    border: var(--border-input);
    border-radius: var(--radius);
    background: var(--paper);
    color: var(--sumi);
    font-size: 13px;
    font-family: var(--font-mono);
    outline: none;
  }
  .param-input:focus { border: var(--border-focus); }
  .result-panel { margin-top: 20px; }
  .result-label {
    font-size: 9.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin: 0 0 8px;
  }
  .result-code {
    padding: 16px;
    background: var(--paper);
    border: var(--border-card);
    border-radius: var(--radius);
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--sumi);
    overflow: auto;
    max-height: 400px;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
  }

  .hint { font-size: 13px; color: var(--sumi-3); }
</style>
