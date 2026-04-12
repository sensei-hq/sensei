<script lang="ts">
  import { onMount } from 'svelte';

  interface AcpStatus {
    id: string;
    name: string;
    installed: boolean;
    mcp_configured: boolean;
    config_path: string;
  }

  const ACP_META: Record<string, { icon: string; description: string }> = {
    'claude-desktop': { icon: 'i-solar-stars-minimalistic-bold-duotone',  description: 'Anthropic Claude desktop app' },
    'claude-code':    { icon: 'i-solar-code-square-bold-duotone',          description: 'Claude Code CLI & VS Code extension' },
    'cursor':         { icon: 'i-solar-cursor-bold-duotone',               description: 'AI-first code editor' },
    'windsurf':       { icon: 'i-solar-wind-bold-duotone',                 description: 'Codeium Windsurf IDE' },
    'zed':            { icon: 'i-solar-bolt-bold-duotone',                 description: 'Zed collaborative editor' },
    'kiro':           { icon: 'i-solar-rocket-bold-duotone',               description: 'AWS Kiro AI IDE' },
    'opencode':       { icon: 'i-solar-terminal-bold-duotone',             description: 'SST OpenCode terminal agent' },
  };

  let acps = $state<AcpStatus[]>([]);
  let loading = $state(true);
  let configuringId = $state<string | null>(null);
  let configureError = $state<string | null>(null);
  let configuredIds = $state<Set<string>>(new Set());

  async function load() {
    loading = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      acps = await invoke<AcpStatus[]>('check_acp_configs');
    } catch {
      // browser preview — show empty state
      acps = [];
    } finally {
      loading = false;
    }
  }

  async function configure(id: string) {
    configuringId = id;
    configureError = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('configure_mcp', { acps: [id] });
      configuredIds = new Set([...configuredIds, id]);
      // Reload status to reflect updated config
      await load();
    } catch (e) {
      configureError = String(e);
    } finally {
      configuringId = null;
    }
  }

  function shortPath(p: string): string {
    const home = p.match(/^\/Users\/[^/]+/) ?? p.match(/^\/home\/[^/]+/);
    return home ? p.replace(home[0], '~') : p;
  }

  onMount(load);
</script>

<div class="flex h-full flex-col min-h-0">
  <!-- Header -->
  <div class="border-b border-surface-z0/50 px-4 py-2 shrink-0 flex items-center justify-between">
    <h1 class="text-sm font-semibold text-surface-z8">ACP Registry</h1>
    <button
      onclick={load}
      class="flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs text-surface-z5 hover:bg-surface-z3/60 hover:text-surface-z7 transition-colors">
      <span class="i-solar-refresh-bold-duotone text-sm {loading ? 'animate-spin' : ''}"></span>
      Refresh
    </button>
  </div>

  <div class="flex-1 overflow-y-auto">
    <div class="max-w-2xl mx-auto px-6 py-6 space-y-4">

      <p class="text-xs text-surface-z4 leading-relaxed">
        AI coding platforms (ACPs) need the sensei MCP server configured to access your codebase knowledge graph.
        Configure each installed editor below.
      </p>

      {#if configureError}
        <div class="rounded-xl border border-error-z3 bg-error-z1 px-4 py-3 text-xs text-error-z6">
          {configureError}
        </div>
      {/if}

      {#if loading}
        <div class="flex items-center gap-2 py-8 justify-center text-surface-z4 text-sm">
          <span class="i-solar-refresh-bold-duotone animate-spin text-base"></span>
          Detecting editors…
        </div>
      {:else}
        <!-- Installed ACPs first -->
        {#each acps.filter(a => a.installed) as acp}
          {@const meta = ACP_META[acp.id] ?? { icon: 'i-solar-cpu-bold-duotone', description: '' }}
          <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 px-4 py-4">
            <div class="flex items-start justify-between gap-4">
              <div class="flex items-start gap-3 min-w-0">
                <div class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary-z2 text-primary-z7">
                  <span class="text-lg {meta.icon}"></span>
                </div>
                <div class="min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="text-sm font-semibold text-surface-z8">{acp.name}</span>
                    <span class="rounded-full bg-success-z1 px-2 py-0.5 text-[10px] font-medium text-success-z7">Detected</span>
                    {#if acp.mcp_configured}
                      <span class="rounded-full bg-primary-z1 px-2 py-0.5 text-[10px] font-medium text-primary-z7">MCP configured</span>
                    {:else}
                      <span class="rounded-full bg-warning-z1 px-2 py-0.5 text-[10px] font-medium text-warning-z7">MCP not configured</span>
                    {/if}
                  </div>
                  <p class="mt-0.5 text-xs text-surface-z4">{meta.description}</p>
                  {#if acp.config_path}
                    <p class="mt-1 font-mono text-[10px] text-surface-z3 break-all">{shortPath(acp.config_path)}</p>
                  {/if}
                </div>
              </div>

              <button
                onclick={() => configure(acp.id)}
                disabled={configuringId === acp.id}
                class="shrink-0 rounded-lg px-3 py-1.5 text-xs font-medium transition-colors
                       {acp.mcp_configured
                         ? 'border border-surface-z3 text-surface-z5 hover:bg-surface-z3/60'
                         : 'bg-primary-z6 text-white hover:bg-primary-z7'}
                       disabled:opacity-50">
                {#if configuringId === acp.id}
                  <span class="i-solar-refresh-bold-duotone animate-spin mr-1"></span>
                {/if}
                {acp.mcp_configured ? 'Reconfigure' : 'Configure'}
              </button>
            </div>
          </div>
        {/each}

        <!-- Not installed ACPs -->
        {#if acps.some(a => !a.installed)}
          <div class="pt-2">
            <p class="mb-2 px-1 text-[10px] font-semibold uppercase tracking-widest text-surface-z3">Not installed</p>
            <div class="rounded-2xl border border-surface-z2 divide-y divide-surface-z2">
              {#each acps.filter(a => !a.installed) as acp}
                {@const meta = ACP_META[acp.id] ?? { icon: 'i-solar-cpu-bold-duotone', description: '' }}
                <div class="flex items-center gap-3 px-4 py-3 opacity-50">
                  <span class="text-base {meta.icon} text-surface-z4"></span>
                  <div class="flex-1 min-w-0">
                    <span class="text-sm text-surface-z6">{acp.name}</span>
                    <p class="text-[10px] text-surface-z3">{meta.description}</p>
                  </div>
                  <span class="text-[10px] text-surface-z3">Not installed</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        {#if acps.length === 0}
          <div class="rounded-2xl border border-surface-z2 bg-surface-z2/30 px-6 py-10 text-center">
            <span class="i-solar-cpu-bold-duotone text-3xl text-surface-z3 mx-auto block mb-3"></span>
            <p class="text-sm text-surface-z5">No editors detected.</p>
            <p class="text-xs text-surface-z3 mt-1">Make sure you're running this in the desktop app, not a browser.</p>
          </div>
        {/if}
      {/if}
    </div>
  </div>
</div>
