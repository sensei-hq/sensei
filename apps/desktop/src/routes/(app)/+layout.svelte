<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';

  let { children } = $props();

  const navItems = [
    { icon: 'i-solar-folder-with-files-bold-duotone', label: 'Projects',  href: '/projects'  },
    { icon: 'i-solar-lightbulb-bold-duotone',         label: 'Ideas',     href: '/ideas'     },
    { icon: 'i-solar-history-bold-duotone',           label: 'Sessions',  href: '/sessions'  },
    { icon: 'i-solar-graph-up-bold-duotone',          label: 'Graph',     href: '/graph'     },
    { icon: 'i-solar-box-bold-duotone',               label: 'Libraries', href: '/libraries' },
  ];

  const DEFAULT_PORT = 7744;
  let senseiPort = $state(DEFAULT_PORT);

  type ServerStatus = 'checking' | 'online' | 'offline';
  let serverStatus = $state<ServerStatus>('checking');
  let indexingQueue = $state<string[]>([]);
  let startError = $state<string | null>(null);
  let healthDetail = $state<{ name: string | null; version: string | null; backend: string | null; ollamaRunning: boolean; ollamaModel: boolean } | null>(null);
  let showDetail = $state(false);

  async function checkServer() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const status = await invoke<{ online: boolean; name: string | null; version: string | null; indexing: string[]; backend: string | null; ollama_running: boolean; ollama_model: boolean }>('check_indexer', { port: senseiPort });
      if (status.online) {
        serverStatus = 'online';
        startError = null;
        indexingQueue = status.indexing;
        healthDetail = { name: status.name, version: status.version, backend: status.backend, ollamaRunning: status.ollama_running, ollamaModel: status.ollama_model };
      } else {
        serverStatus = 'offline';
        indexingQueue = [];
        healthDetail = null;
      }
    } catch {
      // Fallback for browser preview (no Tauri)
      try {
        const res = await fetch(`${SENSEI_API}/health`, { signal: AbortSignal.timeout(2000) });
        serverStatus = res.ok ? 'online' : 'offline';
      } catch {
        serverStatus = 'offline';
      }
    }
  }

  let starting = $state(false);

  async function startIndexer() {
    startError = null;
    starting = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('start_indexer', { port: senseiPort });
      await checkServer();
    } catch (e) {
      startError = String(e);
    } finally {
      starting = false;
    }
  }

  onMount(() => {
    const stored = parseInt(localStorage.getItem('sensei:port') ?? '', 10);
    if (!isNaN(stored) && stored > 0) senseiPort = stored;
    checkServer();
    const interval = setInterval(checkServer, 10_000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex h-screen overflow-hidden select-none bg-surface-z1">

  <!-- ══ SIDEBAR ════════════════════════════════════════════════════════ -->
  <aside class="flex w-48 shrink-0 flex-col border-r border-surface-z0/50 sidebar-vibrancy">

    <!-- Traffic light area + logo -->
    <div class="drag-region flex items-end px-4 pb-3 pt-9">
      <div class="no-drag flex items-center gap-2">
        <div class="flex h-6 w-6 items-center justify-center rounded-lg bg-primary-z6 text-xs font-bold text-white">⬡</div>
        <span class="text-sm font-bold tracking-tight text-surface-z8">sensei</span>
      </div>
    </div>

    <!-- Nav -->
    <nav class="flex-1 space-y-0.5 px-2 py-2 overflow-y-auto">
      <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Workspace</p>
      {#each navItems as item}
        <a
          href={item.href}
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors no-drag
                 {$page.url.pathname.startsWith(item.href)
                   ? 'bg-primary-z2 font-medium text-primary-z7'
                   : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}"
        >
          <span class="text-base {item.icon}"></span>
          {item.label}
        </a>
      {/each}

      <div class="pt-4">
        <p class="mb-1.5 px-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Tools</p>
        <a href="/settings#acp-registry"
          class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm no-drag transition-colors
                 {$page.url.pathname === '/settings' ? 'bg-primary-z2 font-medium text-primary-z7' : 'text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7'}">
          <span class="text-base i-solar-cpu-bold-duotone"></span>
          ACP Registry
        </a>
      </div>
    </nav>

    <!-- Bottom -->
    <div class="border-t border-surface-z0/50 px-3 py-2.5 space-y-1">
      <!-- Server status -->
      <div class="space-y-1">
        <div class="flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-xs cursor-pointer
                    {serverStatus === 'online' ? 'text-surface-z4 hover:text-surface-z6' : serverStatus === 'offline' ? 'text-warning-z6 hover:text-warning-z7' : 'text-surface-z3'}"
             onclick={() => showDetail = !showDetail}>
          <span class="shrink-0 h-1.5 w-1.5 rounded-full
                       {serverStatus === 'online' ? 'bg-success-z5' : serverStatus === 'offline' ? 'bg-warning-z5' : 'bg-surface-z4 animate-pulse'}">
          </span>
          {#if serverStatus === 'checking'}
            <span>Connecting…</span>
          {:else if serverStatus === 'online'}
            <span class="flex-1 truncate">{indexingQueue.length > 0 ? `Indexing ${indexingQueue.length}…` : 'Indexer running'}</span>
            <span class="i-solar-alt-arrow-{showDetail ? 'up' : 'down'}-bold-duotone text-[10px] shrink-0 opacity-60"></span>
          {:else}
            <span class="flex-1">{starting ? 'Starting…' : 'Indexer offline'}</span>
            {#if !starting}
              <button
                onclick={(e) => { e.stopPropagation(); startIndexer(); }}
                class="rounded px-1.5 py-0.5 text-[10px] font-medium bg-warning-z2 text-warning-z7 hover:bg-warning-z3 transition-colors">
                Start
              </button>
            {/if}
            <span class="i-solar-alt-arrow-{showDetail ? 'up' : 'down'}-bold-duotone text-[10px] shrink-0 opacity-60"></span>
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
                  <span class="text-surface-z4">Indexing</span>
                  {#each indexingQueue as repoId}
                    <div class="flex items-center gap-1.5">
                      <span class="i-solar-refresh-bold-duotone animate-spin text-[10px] text-primary-z5 shrink-0"></span>
                      <span class="font-mono text-surface-z6 truncate">{repoId}</span>
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
                value={senseiPort}
                onchange={(e) => {
                  const p = parseInt((e.target as HTMLInputElement).value, 10);
                  if (p > 0) { senseiPort = p; localStorage.setItem('sensei:port', String(p)); checkServer(); }
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

      <a
        href="/settings"
        class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-surface-z5 no-drag transition-colors hover:bg-surface-z3/60 hover:text-surface-z7"
      >
        <span class="text-base i-solar-settings-minimalistic-bold-duotone"></span>
        Settings
      </a>
    </div>
  </aside>

  <!-- ══ MAIN ═════════════════════════════════════════════════════════ -->
  <div class="flex min-w-0 flex-1 flex-col bg-surface-z1 overflow-hidden">
    <!-- Title bar drag region -->
    <div class="drag-region h-7 shrink-0 border-b border-surface-z0/30"></div>
    <main class="flex-1 overflow-hidden min-h-0">
      {@render children()}
    </main>
  </div>

</div>
