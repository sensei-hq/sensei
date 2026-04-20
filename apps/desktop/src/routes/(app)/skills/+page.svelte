<script lang="ts">
  // TODO: wire to real catalog API
  const skills = [
    { name: 'codebase-indexing', description: 'Index an unfamiliar codebase', enabled: true },
    { name: 'reverse-engineering', description: 'Reverse-engineer codebases into docs', enabled: true },
    { name: 'test-gen', description: 'Generate test suites from code', enabled: true },
    { name: 'refactor', description: 'Safe refactoring with tests', enabled: false },
    { name: 'extract-docs', description: 'Extract docs from code', enabled: false },
    { name: 'building-app-mockups', description: 'Build interactive mockups', enabled: false },
    { name: 'identify-unknown-libs', description: 'Detect unknown libraries', enabled: true },
    { name: 'analyze', description: 'Deep codebase analysis', enabled: true },
    { name: 'ui-dummy-data', description: 'Types-first UI development pattern', enabled: false },
  ];

  const plugins = [
    { name: 'sensei-mcp', description: 'Graph-powered code intelligence', status: 'running' },
    { name: 'playwright-mcp', description: 'Browser automation', status: 'available' },
    { name: 'firebase-mcp', description: 'Firebase management', status: 'not installed' },
  ];

  function pluginStatusCls(s: string): string {
    if (s === 'running') return 'bg-success-z2 text-success-z7';
    if (s === 'available') return 'bg-info-z2 text-info-z7';
    return 'bg-surface-z3 text-surface-z5';
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <h2 class="text-lg font-semibold text-surface-z8">Skills & Plugins</h2>

  <!-- Skills -->
  <div class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Skills ({skills.filter(s => s.enabled).length} enabled)</p>
    {#each skills as skill (skill.name)}
      <div class="flex items-center gap-3 rounded-lg bg-surface-z2 px-4 py-2.5">
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium text-surface-z7">{skill.name}</p>
          <p class="text-[10px] text-surface-z4">{skill.description}</p>
        </div>
        <button class="rounded px-2 py-0.5 text-[10px] font-medium {skill.enabled ? 'bg-success-z2 text-success-z7' : 'bg-surface-z3 text-surface-z5'}">
          {skill.enabled ? 'enabled' : 'disabled'}
        </button>
      </div>
    {/each}
  </div>

  <!-- Plugins -->
  <div class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Plugins</p>
    {#each plugins as plugin (plugin.name)}
      <div class="flex items-center gap-3 rounded-lg bg-surface-z2 px-4 py-2.5">
        <span class="text-lg i-solar-cpu-bold-duotone text-surface-z5"></span>
        <div class="flex-1 min-w-0">
          <p class="text-sm font-medium text-surface-z7">{plugin.name}</p>
          <p class="text-[10px] text-surface-z4">{plugin.description}</p>
        </div>
        <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {pluginStatusCls(plugin.status)}">{plugin.status}</span>
      </div>
    {/each}
  </div>

</div>
