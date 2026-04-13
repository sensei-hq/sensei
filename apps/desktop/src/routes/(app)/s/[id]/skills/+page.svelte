<script lang="ts">
  import { page } from '$app/stores';
  import { getSolutionById } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { InferredRole } from '$lib/types.js';

  let solution = $derived(getSolutionById($page.params.id as string));
  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));
  let inferredRoles = $state<InferredRole[]>([]);

  // Skills + plugins + commands catalog with project-type recommendations
  const SKILL_CATALOG = [
    { name: 'zero-errors-policy', description: 'Zero lint/test errors at all times', types: ['all'], kind: 'skill' },
    { name: 'managing-project-sessions', description: 'Structured session protocol with snapshots', types: ['all'], kind: 'skill' },
    { name: 'pattern-based-development', description: 'Follow established patterns from PATTERNS.md', types: ['all'], kind: 'skill' },
    { name: 'detecting-doc-drift', description: 'Flag design docs drifted from code', types: ['api', 'backend', 'library', 'shared-lib'], kind: 'skill' },
    { name: 'identifying-patterns', description: 'Discover and document recurring patterns', types: ['all'], kind: 'skill' },
    { name: 'decomposing-broad-tasks', description: 'Break large tasks into focused subtasks', types: ['api', 'backend'], kind: 'skill' },
    { name: 'indexing-codebase', description: 'Index an unfamiliar codebase for navigation', types: ['all'], kind: 'skill' },
  ];

  const PLUGIN_CATALOG = [
    { name: 'sensei-mcp', description: 'Graph-powered code intelligence via MCP', types: ['all'], kind: 'plugin' },
    { name: 'playwright-mcp', description: 'Browser automation for testing', types: ['ui', 'frontend'], kind: 'plugin' },
    { name: 'firebase-mcp', description: 'Firebase project management', types: ['api', 'backend', 'mobile'], kind: 'plugin' },
  ];

  const COMMAND_CATALOG = [
    { name: '/commit', description: 'Create structured git commits', types: ['all'], kind: 'command' },
    { name: '/review-pr', description: 'Review pull request changes', types: ['all'], kind: 'command' },
    { name: '/benchmark', description: 'Run benchmark on current repo', types: ['all'], kind: 'command' },
  ];

  function getRecommended(repoRole: string) {
    const all = [...SKILL_CATALOG, ...PLUGIN_CATALOG, ...COMMAND_CATALOG];
    return all.filter(s => s.types.includes('all') || s.types.includes(repoRole));
  }

  function getRoleForRepo(repoId: string): string {
    const inferred = inferredRoles.find(r => r.repo_id === repoId);
    if (inferred && inferred.confidence > 0.5) return inferred.role;
    const repoInSolution = solution?.repos.find(r => r.repoId === repoId);
    return repoInSolution?.role ?? 'unknown';
  }

  const KIND_CLS: Record<string, string> = {
    skill: 'bg-primary-z2 text-primary-z7',
    plugin: 'bg-accent-z2 text-accent-z7',
    command: 'bg-info-z2 text-info-z7',
  };

  import { onMount } from 'svelte';
  onMount(async () => {
    if (solution) {
      const api = senseiApi(port);
      inferredRoles = await api.getSolutionRoles(solution.id);
    }
  });
</script>

{#if solution}
  <div class="flex-1 overflow-y-auto px-6 py-5 space-y-5">

    <div class="flex items-center justify-between">
      <h2 class="text-lg font-semibold text-surface-z8">Skills, Plugins & Commands</h2>
      <span class="text-xs text-surface-z4">{solution.repos.length} repos in solution</span>
    </div>

    <p class="text-xs text-surface-z4">
      Recommended based on each repo's detected role. Skills go in <code class="bg-surface-z3 px-1 rounded">.claude/skills/</code>,
      plugins are MCP servers, commands are slash commands.
    </p>

    <!-- Per-repo recommendations -->
    <div class="space-y-4">
      {#each solution.repos as repo}
        {@const role = getRoleForRepo(repo.repoId)}
        {@const recommended = getRecommended(role)}
        <div class="rounded-lg border border-surface-z3 bg-surface-z2/50">
          <div class="px-4 py-3 border-b border-surface-z2">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-surface-z7">{repo.label ?? repo.path.split('/').at(-1)}</span>
              <span class="rounded px-1.5 py-0.5 text-[10px] bg-surface-z3 text-surface-z5">{role}</span>
              <span class="text-[10px] text-surface-z3">{recommended.length} recommended</span>
            </div>
          </div>
          <div class="divide-y divide-surface-z2">
            {#each recommended as item}
              <div class="flex items-center justify-between px-4 py-2.5">
                <div class="flex items-center gap-2 flex-1 min-w-0">
                  <span class="rounded px-1 py-0.5 text-[9px] font-medium {KIND_CLS[item.kind]}">{item.kind}</span>
                  <span class="text-xs text-surface-z7">{item.name}</span>
                  <span class="text-[10px] text-surface-z4 truncate">{item.description}</span>
                </div>
                <button
                  class="rounded-md px-2 py-1 text-[10px] font-medium bg-primary-z2 text-primary-z7 hover:bg-primary-z3 shrink-0"
                >
                  Install
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>

  </div>
{/if}
