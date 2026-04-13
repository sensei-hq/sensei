<script lang="ts">
  import { page } from '$app/stores';
  import { getSolutionById } from '$lib/solutions.svelte.js';

  let solution = $derived(getSolutionById($page.params.id as string));

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

  // Track which skills are installed per repo (would check .claude/skills/ via Tauri)
  // For now, show catalog with install instructions
</script>

{#if solution}
  <div class="flex-1 overflow-y-auto px-6 py-5 space-y-5">

    <div class="flex items-center justify-between">
      <h2 class="text-lg font-semibold text-surface-z8">Skills</h2>
      <span class="text-xs text-surface-z4">{solution.repos.length} repos in solution</span>
    </div>

    <p class="text-xs text-surface-z4">
      Skills are prompt-based enhancements installed into each repo's <code class="bg-surface-z3 px-1 rounded">.claude/skills/</code> directory.
      They guide AI coding assistants to follow best practices for your codebase.
    </p>

    <!-- Per-repo skill status -->
    <div class="space-y-4">
      {#each solution.repos as repo}
        <div class="rounded-lg border border-surface-z3 bg-surface-z2/50">
          <div class="px-4 py-3 border-b border-surface-z2">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-surface-z7">{repo.label ?? repo.path.split('/').at(-1)}</span>
              <span class="text-[10px] text-surface-z4">{repo.role}</span>
            </div>
            <p class="text-[10px] text-surface-z3 font-mono truncate">{repo.path}</p>
          </div>
          <div class="divide-y divide-surface-z2">
            {#each SKILL_CATALOG as skill}
              <div class="flex items-center justify-between px-4 py-2.5">
                <div class="flex items-center gap-2 flex-1 min-w-0">
                  <span class="text-xs text-surface-z7">{skill.name}</span>
                  {#if skill.recommended}
                    <span class="rounded px-1 py-0.5 text-[9px] bg-success-z2 text-success-z7 shrink-0">rec</span>
                  {/if}
                  <span class="text-[10px] text-surface-z4 truncate">{skill.description}</span>
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

    <!-- Bulk actions -->
    <div class="flex gap-2">
      <button class="rounded-lg bg-primary-z3 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z4">
        Install Recommended (All Repos)
      </button>
      <button class="rounded-lg bg-surface-z3 px-3 py-1.5 text-xs font-medium text-surface-z6 hover:bg-surface-z4">
        Install All Skills (All Repos)
      </button>
    </div>

  </div>
{/if}
