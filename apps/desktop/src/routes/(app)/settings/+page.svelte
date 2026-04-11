<script lang="ts">
  import { onMount } from 'svelte';
  import { getSolutions, clearAllSolutions } from '$lib/solutions.svelte.js';

  let projectCount = $state(0);
  let solutionCount = $derived(getSolutions().length);

  onMount(() => {
    try {
      const raw = localStorage.getItem('sensei:projects_raw');
      projectCount = raw ? JSON.parse(raw).length : 0;
    } catch { /* ignore */ }
  });

  function resetSetup() {
    if (!confirm('This will clear all imported projects, solutions, and reset setup. Continue?')) return;
    localStorage.removeItem('sensei:projects_raw');
    localStorage.removeItem('sensei:setup_complete');
    localStorage.removeItem('sensei:variant_overrides');
    localStorage.removeItem('sensei:index_states');
    clearAllSolutions();
    window.location.replace('/');
  }

  function clearVariantOverrides() {
    localStorage.removeItem('sensei:variant_overrides');
    window.location.reload();
  }

  const version = '0.1.0'; // TODO: inject from package.json at build time
</script>

<div class="flex h-full flex-col min-h-0">
  <div class="border-b border-surface-z0/50 px-4 py-2 shrink-0">
    <h1 class="text-sm font-semibold text-surface-z8">Settings</h1>
  </div>

  <div class="flex-1 overflow-y-auto">
    <div class="max-w-xl mx-auto px-6 py-6 space-y-8">

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
