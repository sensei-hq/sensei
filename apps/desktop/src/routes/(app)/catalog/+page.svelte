<script lang="ts">
  type CatalogTab = 'skills' | 'plugins' | 'commands' | 'hooks';
  let activeTab = $state<CatalogTab>('skills');
  let search = $state('');

  const SKILLS = [
    { name: 'zero-errors-policy', description: 'Zero lint/test errors at all times — before and after every change', for: ['all'] },
    { name: 'managing-project-sessions', description: 'Structured session protocol with context snapshots', for: ['all'] },
    { name: 'pattern-based-development', description: 'Check PATTERNS.md before implementing — follow established recipes', for: ['all'] },
    { name: 'detecting-doc-drift', description: 'Flag design docs that have drifted from the code', for: ['api', 'backend', 'library'] },
    { name: 'identifying-patterns', description: 'Discover and document recurring structural patterns', for: ['all'] },
    { name: 'decomposing-broad-tasks', description: 'Break large requests into focused subtasks before starting', for: ['api', 'backend'] },
    { name: 'managing-context', description: 'Trim and refocus context when switching tasks', for: ['all'] },
    { name: 'running-agentic-sessions', description: 'Protocols for running long autonomous sessions safely', for: ['all'] },
    { name: 'compressing-content', description: 'Reduce token usage by compressing code representations', for: ['all'] },
    { name: 'indexing-codebase', description: 'Index an unfamiliar codebase for efficient navigation', for: ['all'] },
  ];

  const PLUGINS = [
    { name: 'sensei-mcp', description: 'Graph-powered code intelligence — search symbols, trace calls, query architecture', for: ['all'] },
    { name: 'playwright-mcp', description: 'Browser automation — navigate, click, screenshot, evaluate JS', for: ['ui', 'frontend', 'e2e'] },
    { name: 'firebase-mcp', description: 'Firebase project management — apps, rules, environment', for: ['api', 'backend', 'mobile'] },
    { name: 'cocoindex-mcp', description: 'Semantic code search across codebase', for: ['all'] },
  ];

  const COMMANDS = [
    { name: '/commit', description: 'Create structured git commits with conventional format', for: ['all'] },
    { name: '/review-pr', description: 'Review pull request for bugs, style, security issues', for: ['all'] },
    { name: '/benchmark', description: 'Run benchmark comparing bare vs skills vs indexed', for: ['all'] },
    { name: '/init', description: 'Initialize sensei for a new project', for: ['all'] },
    { name: '/register', description: 'Register project with ACP (AI Coding Platforms)', for: ['all'] },
  ];

  const HOOKS = [
    { name: 'session-start', description: 'Inject skills and project context on session startup', for: ['all'] },
    { name: 'pre-tool', description: 'Capture tool inputs before execution for analytics', for: ['all'] },
    { name: 'post-tool', description: 'Record tool outputs and exit codes after execution', for: ['all'] },
  ];

  const KIND_CLS: Record<string, string> = {
    skills: 'bg-primary-z2 text-primary-z7',
    plugins: 'bg-accent-z2 text-accent-z7',
    commands: 'bg-info-z2 text-info-z7',
    hooks: 'bg-warning-z2 text-warning-z7',
  };

  let activeItems = $derived(() => {
    const items = activeTab === 'skills' ? SKILLS
      : activeTab === 'plugins' ? PLUGINS
      : activeTab === 'commands' ? COMMANDS
      : HOOKS;
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(i => i.name.toLowerCase().includes(q) || i.description.toLowerCase().includes(q));
  });
</script>

<div class="flex-1 overflow-y-auto px-6 py-5 space-y-5">

  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold text-surface-z8">Catalog</h2>
    <input
      type="text"
      bind:value={search}
      placeholder="Search..."
      class="rounded-md border border-surface-z3 bg-surface-z1 px-2 py-1 text-xs text-surface-z7 outline-none focus:border-primary-z4 w-40"
    />
  </div>

  <!-- Tab bar -->
  <div class="flex gap-1">
    {#each [['skills', 'Skills', SKILLS.length], ['plugins', 'Plugins', PLUGINS.length], ['commands', 'Commands', COMMANDS.length], ['hooks', 'Hooks', HOOKS.length]] as [id, label, count]}
      <button
        onclick={() => activeTab = id as CatalogTab}
        class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors
               {activeTab === id ? KIND_CLS[id] : 'bg-surface-z2 text-surface-z5 hover:bg-surface-z3'}"
      >
        {label}
        <span class="text-[10px] opacity-60">{count}</span>
      </button>
    {/each}
  </div>

  <!-- Items list -->
  <div class="rounded-2xl border border-surface-z3 bg-surface-z2/50 divide-y divide-surface-z2">
    {#each activeItems() as item}
      <div class="flex items-center justify-between px-4 py-3">
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <p class="text-sm font-medium text-surface-z7">{item.name}</p>
            {#each item.for.filter(f => f !== 'all').slice(0, 3) as tag}
              <span class="rounded px-1 py-0.5 text-[9px] bg-surface-z3 text-surface-z5">{tag}</span>
            {/each}
          </div>
          <p class="text-xs text-surface-z4 mt-0.5">{item.description}</p>
        </div>
      </div>
    {/each}
    {#if activeItems().length === 0}
      <div class="px-4 py-6 text-center text-xs text-surface-z4">No items matching "{search}"</div>
    {/if}
  </div>

  <p class="text-[10px] text-surface-z3">
    Skills are installed per-repo. Plugins are MCP servers configured via ACP Registry. Commands are slash commands available in Claude Code. Hooks run automatically on session events.
  </p>

</div>
