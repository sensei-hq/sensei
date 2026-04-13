<script lang="ts">
  import { onMount } from 'svelte';
  import { getSolutions, clearAllSolutions } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';

  let projectCount = $state(0);
  let solutionCount = $derived(getSolutions().length);
  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));
  let sidebarMax = $state(parseInt(localStorage.getItem('sensei:sidebar_max_items') ?? '5', 10));
  let daemonHealth = $state<Record<string, unknown>>({});

  // Skills catalog (embedded — matches CLI skill-catalog.ts)
  const SKILL_CATALOG = [
    { name: 'zero-errors-policy', description: 'Zero lint/test errors at all times', recommended: true },
    { name: 'managing-project-sessions', description: 'Structured session protocol with snapshots', recommended: true },
    { name: 'pattern-based-development', description: 'Follow established patterns from PATTERNS.md', recommended: true },
    { name: 'detecting-doc-drift', description: 'Flag design docs drifted from code', recommended: true },
    { name: 'identifying-patterns', description: 'Discover and document recurring patterns', recommended: true },
    { name: 'decomposing-broad-tasks', description: 'Break large tasks into focused subtasks', recommended: false },
    { name: 'managing-context', description: 'Trim and refocus context when switching', recommended: false },
    { name: 'running-agentic-sessions', description: 'Protocols for long autonomous sessions', recommended: false },
    { name: 'compressing-content', description: 'Reduce token usage via compression', recommended: false },
    { name: 'indexing-codebase', description: 'Index an unfamiliar codebase', recommended: false },
  ];

  onMount(async () => {
    try {
      const raw = localStorage.getItem('sensei:projects_raw');
      projectCount = raw ? JSON.parse(raw).length : 0;
    } catch { /* ignore */ }

    const api = senseiApi(port);
    daemonHealth = await api.getHealth();
  });

  async function resetSetup() {
    if (!confirm('This will clear all imported projects, solutions, and reset setup. Continue?')) return;
    // Clear daemon solutions
    const api = senseiApi(port);
    try {
      const sols = await api.listSolutions();
      for (const s of sols) { await api.deleteSolution(s.id); }
    } catch { /* non-fatal */ }
    // Clear localStorage
    localStorage.removeItem('sensei:projects_raw');
    localStorage.removeItem('sensei:setup_complete');
    localStorage.removeItem('sensei:variant_overrides');
    localStorage.removeItem('sensei:index_states');
    localStorage.removeItem('sensei:sidebar_max_items');
    clearAllSolutions();
    window.location.replace('/');
  }

  function clearVariantOverrides() {
    localStorage.removeItem('sensei:variant_overrides');
    window.location.reload();
  }

  function updateSidebarMax(val: number) {
    sidebarMax = Math.max(1, Math.min(20, val));
    localStorage.setItem('sensei:sidebar_max_items', String(sidebarMax));
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

      <!-- Skills -->
      <section>
        <h2 class="text-xs font-semibold uppercase tracking-widest text-surface-z4 mb-3">Skills Catalog</h2>
        <p class="text-xs text-surface-z4 mb-3">Skills are prompts that enhance AI coding assistants. Install them per-repo in <code class="bg-surface-z3 px-1 rounded">.claude/skills/</code>.</p>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 divide-y divide-surface-z2">
          {#each SKILL_CATALOG as skill}
            <div class="flex items-center justify-between px-4 py-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <p class="text-sm font-medium text-surface-z7">{skill.name}</p>
                  {#if skill.recommended}
                    <span class="rounded px-1 py-0.5 text-[9px] bg-success-z2 text-success-z7">recommended</span>
                  {/if}
                </div>
                <p class="text-xs text-surface-z4 mt-0.5 truncate">{skill.description}</p>
              </div>
            </div>
          {/each}
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
          <div class="flex items-center justify-between px-4 py-3">
            <div>
              <p class="text-sm font-medium text-surface-z8">Clear variant overrides</p>
              <p class="text-xs text-surface-z4 mt-0.5">Reset all manual group/ungroup changes</p>
            </div>
            <button
              onclick={clearVariantOverrides}
              class="rounded-lg border border-surface-z3 px-3 py-1.5 text-xs font-medium text-surface-z6 transition-colors hover:bg-surface-z3">
              Clear
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
