<script lang="ts">
  import { onMount } from 'svelte';
  import { getSolutions, clearAllSolutions } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort, getSidebarMaxItems, getGlobalSkills, setSidebarMaxItems, setGlobalSkills, resetAppState } from '$lib/appstate.svelte.js';

  let projectCount = $state(0);
  let solutionCount = $derived(getSolutions().length);
  let port = $derived(getPort());
  let sidebarMax = $derived(getSidebarMaxItems());
  let daemonHealth = $state<Record<string, unknown>>({});

  const ALL_SKILLS = [
    'zero-errors-policy', 'managing-project-sessions', 'pattern-based-development',
    'detecting-doc-drift', 'identifying-patterns', 'decomposing-broad-tasks',
    'managing-context', 'running-agentic-sessions', 'compressing-content', 'indexing-codebase',
  ];
  let globalSkills = $derived(getGlobalSkills());

  function toggleGlobalSkill(name: string) {
    const current = getGlobalSkills();
    const updated = current.includes(name) ? current.filter(s => s !== name) : [...current, name];
    setGlobalSkills(updated);
  }

  onMount(async () => {
    const api = senseiApi(port);
    daemonHealth = await api.getHealth();
    // Get project count from daemon (source of truth)
    const projects = await api.getProjects();
    projectCount = projects.length;
  });

  async function resetSetup() {
    if (!confirm('This will clear all imported projects, solutions, and reset setup. Continue?')) return;
    await resetAppState();
    clearAllSolutions();
    window.location.replace('/');
  }

  function updateSidebarMax(val: number) {
    setSidebarMaxItems(val);
  }

  async function checkDaemon() {
    const api = senseiApi(port);
    daemonHealth = await api.getHealth();
  }

  const version = '0.1.0';
</script>

<div class="flex h-full flex-col min-h-0">
  <div class="border-b border-surface-z0/50 px-4 py-2 shrink-0">
    <h1 class="text-sm font-semibold text-surface-z8">Settings</h1>
  </div>

  <div class="flex-1 overflow-y-auto">
    <div class="max-w-xl mx-auto px-6 py-6 space-y-8">

      <!-- Display -->
      <section>
        <h2 class="text-xs font-semibold uppercase tracking-widest text-surface-z4 mb-3">Display</h2>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 divide-y divide-surface-z2">
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <p class="text-sm font-medium text-surface-z8">Sidebar max items</p>
              <p class="text-xs text-surface-z4 mt-0.5">Maximum solutions shown per category before "show more"</p>
            </div>
            <div class="flex items-center gap-2">
              <button onclick={() => updateSidebarMax(sidebarMax - 1)} class="w-7 h-7 rounded-lg bg-surface-z3 text-surface-z6 text-sm hover:bg-surface-z4">-</button>
              <span class="text-sm font-semibold text-surface-z7 w-6 text-center">{sidebarMax}</span>
              <button onclick={() => updateSidebarMax(sidebarMax + 1)} class="w-7 h-7 rounded-lg bg-surface-z3 text-surface-z6 text-sm hover:bg-surface-z4">+</button>
            </div>
          </div>
        </div>
      </section>

      <!-- Global Skills -->
      <section>
        <h2 class="text-xs font-semibold uppercase tracking-widest text-surface-z4 mb-3">Global Skills</h2>
        <p class="text-xs text-surface-z4 mb-3">Skills enabled globally are installed across all repos. Per-solution skills pages only show skills not already enabled here.</p>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 divide-y divide-surface-z2">
          {#each ALL_SKILLS as skill}
            <div class="flex items-center justify-between px-4 py-2.5">
              <span class="text-sm text-surface-z7">{skill}</span>
              <button
                onclick={() => toggleGlobalSkill(skill)}
                class="rounded-md px-2 py-1 text-[10px] font-medium transition-colors {globalSkills.includes(skill) ? 'bg-success-z2 text-success-z7' : 'bg-surface-z3 text-surface-z5 hover:bg-surface-z4'}"
              >
                {globalSkills.includes(skill) ? 'Enabled' : 'Disabled'}
              </button>
            </div>
          {/each}
        </div>
      </section>

      <!-- Daemon -->
      <section>
        <h2 class="text-xs font-semibold uppercase tracking-widest text-surface-z4 mb-3">Daemon</h2>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 divide-y divide-surface-z2">
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <p class="text-sm font-medium text-surface-z8">Status</p>
              <p class="text-xs text-surface-z4 mt-0.5">senseid Rust daemon</p>
            </div>
            <div class="flex items-center gap-2">
              {#if daemonHealth.ok}
                <span class="w-2 h-2 rounded-full bg-success-z5"></span>
                <span class="text-xs text-success-z6">Online</span>
                <span class="text-[10px] text-surface-z4">v{daemonHealth.version ?? '?'}</span>
              {:else}
                <span class="w-2 h-2 rounded-full bg-error-z5"></span>
                <span class="text-xs text-error-z6">Offline</span>
              {/if}
              <button onclick={checkDaemon} class="text-[10px] text-primary-z6 hover:text-primary-z7 ml-1">Check</button>
            </div>
          </div>
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <p class="text-sm font-medium text-surface-z8">Port</p>
              <p class="text-xs text-surface-z4 mt-0.5">Daemon API port</p>
            </div>
            <span class="text-sm font-mono text-surface-z6">{port}</span>
          </div>
        </div>
      </section>

      <!-- Workspace -->
      <section>
        <h2 class="text-xs font-semibold uppercase tracking-widest text-surface-z4 mb-3">Workspace</h2>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 divide-y divide-surface-z2">
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <p class="text-sm font-medium text-surface-z8">Imported projects</p>
              <p class="text-xs text-surface-z4 mt-0.5">Repos loaded from your last scan</p>
            </div>
            <span class="text-sm font-semibold text-surface-z7">{projectCount}</span>
          </div>
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <p class="text-sm font-medium text-surface-z8">Solutions</p>
              <p class="text-xs text-surface-z4 mt-0.5">Repos grouped into solutions</p>
            </div>
            <span class="text-sm font-semibold text-surface-z7">{solutionCount}</span>
          </div>
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <p class="text-sm font-medium text-surface-z8">Reset workspace</p>
              <p class="text-xs text-surface-z4 mt-0.5">Clear all imported data and restart setup</p>
            </div>
            <button
              onclick={resetSetup}
              class="rounded-lg border border-error-z3 px-3 py-1.5 text-xs font-medium text-error-z6 transition-colors hover:bg-error-z1">
              Reset
            </button>
          </div>
        </div>
      </section>

      <!-- ACP Registry -->
      <section>
        <h2 class="text-xs font-semibold uppercase tracking-widest text-surface-z4 mb-3">ACP Registry</h2>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 px-4 py-4 flex items-center justify-between gap-4">
          <div>
            <p class="text-sm font-medium text-surface-z8">AI coding platforms</p>
            <p class="text-xs text-surface-z4 mt-0.5">Configure MCP for each installed editor</p>
          </div>
          <a href="/acp"
            class="inline-flex items-center gap-1.5 rounded-lg border border-surface-z3 px-3 py-1.5 text-xs font-medium text-surface-z6 transition-colors hover:bg-surface-z3/60 shrink-0">
            <span class="i-solar-cpu-bold-duotone text-sm"></span>
            Open Registry
          </a>
        </div>
      </section>

      <!-- About -->
      <section>
        <h2 class="text-xs font-semibold uppercase tracking-widest text-surface-z4 mb-3">About</h2>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 divide-y divide-surface-z2">
          <div class="flex items-center justify-between px-4 py-3">
            <p class="text-sm font-medium text-surface-z8">Sensei</p>
            <span class="text-xs text-surface-z4">v{version}</span>
          </div>
          <div class="px-4 py-3">
            <p class="text-xs text-surface-z4 leading-relaxed">
              Sensei tracks your AI coding sessions, builds a knowledge graph of your codebase,
              and helps you work smarter with every session.
            </p>
          </div>
        </div>
      </section>

    </div>
  </div>
</div>
