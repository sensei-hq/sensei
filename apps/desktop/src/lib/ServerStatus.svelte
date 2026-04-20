<script lang="ts">
  import { onMount } from 'svelte';
  import { senseiApi } from '$lib/api.js';
  import { version as APP_VERSION } from '$app/environment';

  let { port = $bindable(7744) } = $props();

  type ServerStatus = 'checking' | 'online' | 'offline';
  let serverStatus = $state<ServerStatus>('checking');
  let prevStatus = $state<ServerStatus>('checking');
  let indexingQueue = $state<string[]>([]);
  let prevQueue = $state<string[]>([]);
  let startError = $state<string | null>(null);
  let healthDetail = $state<{ name: string | null; version: string | null; backend: string | null; ollamaRunning: boolean; ollamaModel: boolean } | null>(null);
  let showDetail = $state(false);
  let hasTriggeredInit = $state(false);
  let starting = $state(false);
  let versionMismatch = $state(false);

  type LogEntry = { time: string; text: string; kind: 'info' | 'success' | 'warn' };
  let activityLog = $state<LogEntry[]>([]);

  function addLog(text: string, kind: LogEntry['kind'] = 'info') {
    const time = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    activityLog = [{ time, text, kind }, ...activityLog].slice(0, 50);
  }

  function shortId(repoId: string): string {
    return repoId.split('/').at(-1) ?? repoId.slice(0, 20);
  }

  async function triggerAllUnindexed() {
    try {
      const api = senseiApi(port);
      const projects = await api.getProjects();
      const scanned = projects.map(p => ({ name: p.name, path: p.path, repoId: p.repo_id }));

      if (scanned.length === 0) return;

      let serverProjects: { repoId: string; path: string; indexedAt?: string }[] = [];
      try {
        const res = await fetch(`http://127.0.0.1:${port}/api/projects`);
        if (res.ok) serverProjects = await res.json() as typeof serverProjects;
      } catch { /* ignore */ }

      const indexedPaths = new Set(serverProjects.filter(p => p.indexedAt).map(p => p.path));
      const toIndex = scanned.filter(p => !indexedPaths.has(p.path));
      if (toIndex.length === 0) return;

      addLog(`Starting indexer for ${toIndex.length} repo(s)…`, 'info');
      for (const p of toIndex) {
        const repoId = p.repoId ?? p.path.replace(/^\//, '');
        await fetch(`http://127.0.0.1:${port}/api/projects`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ repoId, name: p.name, path: p.path }),
        }).catch(() => {});
        await fetch(`http://127.0.0.1:${port}/api/index`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ repoId, repoPath: p.path }),
        }).catch(() => {});
      }
    } catch { /* ignore */ }
  }

  export async function checkServer() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const status = await invoke<{ online: boolean; name: string | null; version: string | null; indexing: string[]; backend: string | null; ollama_running: boolean; ollama_model: boolean }>('check_indexer', { port });
      const newQueue = status.indexing ?? [];
      if (status.online) {
        if (prevStatus !== 'online') addLog('Indexer connected', 'success');
        serverStatus = 'online';
        startError = null;
        healthDetail = { name: status.name, version: status.version, backend: status.backend, ollamaRunning: status.ollama_running, ollamaModel: status.ollama_model };
        versionMismatch = !!status.version && status.version !== APP_VERSION;
        for (const id of newQueue) {
          if (!prevQueue.includes(id)) addLog(`Indexing: ${shortId(id)}…`, 'info');
        }
        for (const id of prevQueue) {
          if (!newQueue.includes(id)) addLog(`Indexed: ${shortId(id)} ✓`, 'success');
        }
        prevQueue = newQueue;
        indexingQueue = newQueue;
        if (!hasTriggeredInit) {
          hasTriggeredInit = true;
          await triggerAllUnindexed();
        }
      } else {
        if (prevStatus === 'online') addLog('Indexer disconnected', 'warn');
        serverStatus = 'offline';
        indexingQueue = [];
        prevQueue = [];
        healthDetail = null;
      }
      prevStatus = serverStatus;
    } catch {
      try {
        const res = await fetch(`http://127.0.0.1:${port}/health`, { signal: AbortSignal.timeout(2000) });
        serverStatus = res.ok ? 'online' : 'offline';
      } catch {
        serverStatus = 'offline';
      }
    }
  }

  async function startIndexer() {
    startError = null;
    starting = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('start_indexer', { port });
      await checkServer();
    } catch (e) {
      startError = String(e);
    } finally {
      starting = false;
    }
  }

  onMount(() => {
    checkServer();
    const interval = setInterval(checkServer, 10_000);
    return () => clearInterval(interval);
  });
</script>

<div class="space-y-1">
  <div class="flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs
              {serverStatus === 'online' ? 'text-surface-z4' : serverStatus === 'offline' ? 'text-warning-z6' : 'text-surface-z3'}">
    <span class="shrink-0 h-1.5 w-1.5 rounded-full
                 {serverStatus === 'online' ? 'bg-success-z5' : serverStatus === 'offline' ? 'bg-warning-z5' : 'bg-surface-z4 animate-pulse'}">
    </span>
    {#if serverStatus === 'checking'}
      <button onclick={() => showDetail = !showDetail} class="flex flex-1 items-center gap-1 text-left hover:text-surface-z6 transition-colors">
        <span class="flex-1">Connecting…</span>
      </button>
    {:else if serverStatus === 'online'}
      <button onclick={() => showDetail = !showDetail} class="flex flex-1 items-center gap-1 text-left hover:text-surface-z6 transition-colors">
        <span class="flex-1 truncate">{indexingQueue.length > 0 ? `Indexing ${indexingQueue.length}…` : 'Indexer running'}</span>
        <span class="i-solar-alt-arrow-{showDetail ? 'up' : 'down'}-bold-duotone text-[10px] shrink-0 opacity-60"></span>
      </button>
    {:else}
      <button onclick={() => showDetail = !showDetail} class="flex flex-1 items-center gap-1 text-left hover:text-warning-z7 transition-colors">
        <span class="flex-1">{starting ? 'Starting…' : 'Indexer offline'}</span>
        <span class="i-solar-alt-arrow-{showDetail ? 'up' : 'down'}-bold-duotone text-[10px] shrink-0 opacity-60"></span>
      </button>
      {#if !starting}
        <button
          onclick={() => startIndexer()}
          class="rounded px-1.5 py-0.5 text-[10px] font-medium bg-warning-z2 text-warning-z7 hover:bg-warning-z3 transition-colors">
          Start
        </button>
      {/if}
    {/if}
  </div>

  {#if showDetail}
    <div class="rounded-lg bg-surface-z2 px-3 py-2 space-y-1.5 text-[10px]">
      {#if healthDetail}
        {#if healthDetail.name}
          <div class="flex justify-between">
            <span class="text-surface-z4">Server</span>
            <span class="text-surface-z7 font-mono">{healthDetail.name} {healthDetail.version ?? ''}</span>
          </div>
        {/if}
        {#if versionMismatch}
          <div class="rounded bg-warning-z2 px-2 py-1 text-warning-z7">
            Version mismatch: app {APP_VERSION}, daemon {healthDetail?.version}. Restart daemon: senseid stop && senseid start
          </div>
        {/if}
        <div class="flex justify-between">
          <span class="text-surface-z4">Backend</span>
          <span class="text-surface-z7 font-mono">{healthDetail.backend ?? '—'}</span>
        </div>
        <div class="flex justify-between">
          <span class="text-surface-z4">Ollama</span>
          <span class="{healthDetail.ollamaRunning ? 'text-success-z6' : 'text-warning-z6'}">
            {healthDetail.ollamaRunning ? 'running' : 'offline'}
          </span>
        </div>
        <div class="flex justify-between">
          <span class="text-surface-z4">Model</span>
          <span class="{healthDetail.ollamaModel ? 'text-success-z6' : 'text-warning-z6'}">
            {healthDetail.ollamaModel ? 'ready' : 'not loaded'}
          </span>
        </div>
        {#if indexingQueue.length > 0}
          <div class="border-t border-surface-z3 pt-1.5 space-y-1">
            <span class="text-surface-z4">Indexing now</span>
            {#each indexingQueue as repoId}
              <div class="flex items-center gap-1.5">
                <span class="i-solar-refresh-bold-duotone animate-spin text-[10px] text-primary-z5 shrink-0"></span>
                <span class="font-mono text-surface-z6 truncate">{shortId(repoId)}</span>
              </div>
            {/each}
          </div>
        {/if}
        {#if serverStatus === 'online'}
          <div class="border-t border-surface-z3 pt-1.5">
            <button
              onclick={triggerAllUnindexed}
              class="w-full rounded-md bg-primary-z2 px-2 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3 transition-colors text-left">
              <span class="i-solar-refresh-bold-duotone mr-1"></span>Index all unindexed repos
            </button>
          </div>
        {/if}
        {#if activityLog.length > 0}
          <div class="border-t border-surface-z3 pt-1.5 space-y-1">
            <span class="text-surface-z4">Activity</span>
            {#each activityLog.slice(0, 8) as entry}
              <div class="flex items-start gap-1.5 leading-tight">
                <span class="shrink-0 text-surface-z3 tabular-nums">{entry.time.slice(-8)}</span>
                <span class="truncate {entry.kind === 'success' ? 'text-success-z6' : entry.kind === 'warn' ? 'text-warning-z6' : 'text-surface-z5'}">
                  {entry.text}
                </span>
              </div>
            {/each}
          </div>
        {/if}
        <div class="border-t border-surface-z3 pt-1.5"></div>
      {/if}
      <div class="flex items-center gap-2">
        <span class="text-surface-z4 shrink-0">Port</span>
        <input
          type="number"
          value={port}
          onchange={(e) => {
            const p = parseInt((e.target as HTMLInputElement).value, 10);
            if (p > 0) { port = p; localStorage.setItem('sensei:port', String(p)); checkServer(); }
          }}
          class="flex-1 rounded border border-surface-z3 bg-surface-z1 px-1.5 py-0.5 font-mono text-surface-z7 outline-none focus:border-primary-z4"
        />
      </div>
    </div>
  {/if}

  {#if startError}
    <p class="px-2.5 text-[10px] text-error-z5 break-words">{startError}</p>
  {/if}
</div>
