<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // View modes
  type ViewMode = 'split' | 'board';
  let viewMode = $state<ViewMode>('split');

  // Selection
  let selectedId = $state<string | null>(null);
  $effect(() => { if (selectedId === null && data.projects[0]) selectedId = data.projects[0].id; });
  let selectedCardId = $state<string | null>(null);
  let activeTab = $state<'cards' | 'graph' | 'sessions'>('cards');

  // Filters
  let search = $state('');
  let kindFilter = $state<'all' | 'repo' | 'idea'>('all');
  let cardPhaseFilter = $state<string>('all');

  // Prompt
  let prompt = $state('');

  // Import panel
  let showImport = $state(false);
  let importPath = $state('');
  let importName = $state('');
  let importPhase = $state<'idle' | 'indexing' | 'done'>('idle');
  let importProgress = $state(0);
  let importStep = $state(0);
  const importSteps = ['Scanning files', 'Extracting symbols', 'Building graph', 'Detecting libraries'];
  const importResults = ['3,241 files', '8,432 symbols', '142 nodes · 5 communities', '6 libraries'];
  let importInterval: ReturnType<typeof setInterval> | null = null;

  function onImportPath(e: Event) {
    const v = (e.target as HTMLInputElement).value;
    importPath = v;
    importName = v.split('/').filter(Boolean).pop() ?? '';
  }

  function startImport() {
    importPhase = 'indexing';
    importProgress = 0;
    importStep = 0;
    importInterval = setInterval(() => {
      importProgress = Math.min(100, importProgress + 2 + Math.random() * 4);
      importStep = Math.min(3, Math.floor(importProgress / 26));
      if (importProgress >= 100) {
        clearInterval(importInterval!);
        setTimeout(() => { importPhase = 'done'; }, 300);
      }
    }, 80);
  }

  function closeImport() {
    if (importInterval) clearInterval(importInterval);
    showImport = false;
    importPath = '';
    importName = '';
    importPhase = 'idle';
    importProgress = 0;
  }

  const maturityLabel = ['Seed', 'Exploring', 'Developing', 'Maturing', 'Established', 'Mature'];
  const maturityBg    = ['bg-surface-z3', 'bg-info-z5', 'bg-warning-z5', 'bg-secondary-z5', 'bg-success-z5', 'bg-primary-z6'];

  const kindIcon: Record<string, string> = {
    decision:    'i-solar-shield-check-bold-duotone',
    requirement: 'i-solar-checklist-bold-duotone',
    task:        'i-solar-sledgehammer-bold-duotone',
    note:        'i-solar-note-bold-duotone',
    question:    'i-solar-question-circle-bold-duotone',
    finding:     'i-solar-telescope-bold-duotone',
  };
  const kindColor: Record<string, string> = {
    decision:    'text-primary-z6',
    requirement: 'text-success-z6',
    task:        'text-warning-z6',
    note:        'text-info-z6',
    question:    'text-danger-z6',
    finding:     'text-secondary-z6',
  };
  const tagBg: Record<string, string> = {
    decision:    'bg-primary-z2 text-primary-z7',
    requirement: 'bg-success-z2 text-success-z7',
    task:        'bg-warning-z2 text-warning-z7',
    note:        'bg-info-z2 text-info-z7',
    question:    'bg-danger-z2 text-danger-z7',
    finding:     'bg-secondary-z2 text-secondary-z7',
  };

  let filteredProjects = $derived(
    data.projects.filter((p: { kind: string; name: string; description: string }) => {
      if (kindFilter !== 'all' && p.kind !== kindFilter) return false;
      if (search && !p.name.toLowerCase().includes(search.toLowerCase()) &&
          !p.description.toLowerCase().includes(search.toLowerCase())) return false;
      return true;
    })
  );

  let project = $derived(data.projects.find((p: { id: string }) => p.id === selectedId));

  let filteredCards = $derived(
    project?.cards.filter((c: { phase?: string }) =>
      cardPhaseFilter === 'all' || c.phase === cardPhaseFilter
    ) ?? []
  );

  let selectedCard = $derived(project?.cards.find((c: { id: string }) => c.id === selectedCardId));

  // Phases for card filter
  let phaseNames = $derived(project?.phases.map((p: { name: string }) => p.name) ?? []);

  function selectProject(id: string) {
    selectedId = id;
    selectedCardId = null;
    cardPhaseFilter = 'all';
    activeTab = 'cards';
  }
</script>

<div class="flex h-full min-h-0 flex-col">

  <!-- Top bar -->
  <div class="flex items-center justify-between border-b border-surface-z0/50 px-4 py-2 shrink-0">
    <h1 class="text-sm font-semibold text-surface-z8">Projects</h1>
    <div class="flex items-center gap-1.5">
      <!-- View toggle -->
      <div class="flex items-center rounded-lg border border-surface-z3 bg-surface-z2 p-0.5">
        <button
          onclick={() => viewMode = 'split'}
          title="Split view"
          class="rounded-md p-1.5 transition-colors {viewMode === 'split' ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
        >
          <span class="i-solar-sidebar-minimalistic-bold-duotone text-sm"></span>
        </button>
        <button
          onclick={() => viewMode = 'board'}
          title="Board view"
          class="rounded-md p-1.5 transition-colors {viewMode === 'board' ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
        >
          <span class="i-solar-widget-3-bold-duotone text-sm"></span>
        </button>
      </div>
      <button
        onclick={() => { showImport = true; importPhase = 'idle'; }}
        class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 py-1.5 text-xs text-surface-z7 transition-colors hover:bg-surface-z3"
      >
        <span class="i-solar-add-circle-bold-duotone text-sm"></span>
        Import project
      </button>
    </div>
  </div>

  <!-- Content switches on view mode -->
  {#if viewMode === 'split'}
    <!-- ══ SPLIT VIEW ════════════════════════════════════════════════ -->
    <div class="flex flex-1 min-h-0 overflow-hidden">

      <!-- Project list panel -->
      <div class="flex w-64 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">
        <div class="px-3 py-2.5 space-y-2">
          <div class="relative">
            <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-xs i-solar-magnifer-bold-duotone text-surface-z4"></span>
            <input
              bind:value={search}
              placeholder="Search…"
              class="w-full rounded-lg border border-surface-z3 bg-surface-z2 py-1.5 pl-7 pr-3 text-xs outline-none placeholder:text-surface-z4 focus:border-primary-z5"
            />
          </div>
          <div class="flex gap-0.5">
            {#each (['all', 'repo', 'idea'] as const) as f}
              <button
                onclick={() => kindFilter = f}
                class="flex-1 rounded-md py-1 text-[10px] font-medium transition-colors capitalize
                       {kindFilter === f ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
              >{f === 'all' ? 'All' : f === 'repo' ? 'Repos' : 'Ideas'}</button>
            {/each}
          </div>
        </div>

        <div class="flex-1 overflow-y-auto px-1.5 pb-3 space-y-0.5">
          {#each filteredProjects as p (p.id)}
            <button
              onclick={() => selectProject(p.id)}
              class="w-full rounded-xl px-3 py-2.5 text-left transition-colors
                     {selectedId === p.id ? 'bg-primary-z2' : 'hover:bg-surface-z2/80'}"
            >
              <div class="flex items-center gap-1.5">
                <span class="text-sm shrink-0 {p.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
                <span class="truncate text-sm font-medium text-surface-z8">{p.name}</span>
                {#if p.language}
                  <span class="ml-auto shrink-0 text-[10px] text-surface-z4">{p.language}</span>
                {/if}
              </div>
              <p class="mt-0.5 truncate text-[11px] text-surface-z4">{p.description}</p>
              <div class="mt-2 flex items-center gap-2">
                <div class="flex flex-1 gap-0.5">
                  {#each Array(5) as _, i}
                    <div class="h-1 flex-1 rounded-full {i < p.maturity ? maturityBg[p.maturity] : 'bg-surface-z3'}"></div>
                  {/each}
                </div>
                <span class="text-[10px] text-surface-z4">{p.lastActivity}</span>
              </div>
            </button>
          {/each}
        </div>
      </div>

      <!-- Detail panel -->
      {#if project}
        <div class="flex flex-1 min-w-0 flex-col overflow-hidden">

          <!-- Project header -->
          <div class="border-b border-surface-z0/50 px-5 py-3 shrink-0">
            <div class="flex items-start justify-between">
              <div>
                <div class="flex items-center gap-2">
                  <span class="text-base {project.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
                  <h2 class="text-base font-semibold text-surface-z9">{project.name}</h2>
                  <span class="rounded-full bg-surface-z3 px-2 py-0.5 text-[10px] font-medium text-surface-z6">{maturityLabel[project.maturity]}</span>
                </div>
                {#if project.path}
                  <p class="mt-0.5 font-mono text-[11px] text-surface-z4">{project.path}</p>
                {/if}
              </div>
              <button class="rounded-lg border border-surface-z3 px-2.5 py-1.5 text-xs font-medium text-surface-z7 transition-colors hover:bg-surface-z3">
                Open in Claude Code
              </button>
            </div>

            <!-- Stats strip -->
            <div class="mt-3 flex items-center gap-4 text-xs text-surface-z5">
              <span><span class="font-semibold text-surface-z8">{project.sessionCount}</span> sessions</span>
              <span><span class="font-semibold text-surface-z8">{project.cardCount}</span> cards</span>
              <span><span class="font-semibold text-surface-z8">{project.symbolCount?.toLocaleString()}</span> symbols</span>
              {#if project.ftrScore != null}
                <span class="rounded-full bg-success-z2 px-2 py-0.5 text-[10px] font-semibold text-success-z7">
                  FTR {Math.round(project.ftrScore * 100)}%
                </span>
              {/if}
            </div>

            <!-- Phase pipeline -->
            <div class="mt-3 flex gap-1">
              {#each project.phases as phase}
                <div class="flex flex-1 flex-col items-center gap-1">
                  <div class="h-1 w-full rounded-full overflow-hidden bg-surface-z3">
                    {#if phase.done || phase.active}
                      <div class="h-full rounded-full {phase.done ? 'bg-success-z5' : 'bg-primary-z6'}"></div>
                    {/if}
                  </div>
                  <span class="text-[9px] {phase.active ? 'font-bold text-primary-z7' : phase.done ? 'text-success-z5' : 'text-surface-z3'}">
                    {phase.name.slice(0,4)}
                  </span>
                </div>
              {/each}
            </div>

            <!-- Tab bar -->
            <div class="mt-3 flex gap-1">
              {#each (['cards', 'graph', 'sessions'] as const) as tab}
                <button
                  onclick={() => activeTab = tab}
                  class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors capitalize
                         {activeTab === tab ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
                >{tab}</button>
              {/each}
            </div>
          </div>

          <!-- Tab content -->
          {#if activeTab === 'cards'}
            <div class="flex flex-1 min-h-0 overflow-hidden">
              <!-- Card list -->
              <div class="flex w-72 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">
                <div class="px-3 py-2 shrink-0">
                  <div class="flex gap-0.5 flex-wrap">
                    <button
                      onclick={() => cardPhaseFilter = 'all'}
                      class="rounded-md px-2 py-0.5 text-[10px] font-medium transition-colors
                             {cardPhaseFilter === 'all' ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
                    >All</button>
                    {#each phaseNames as p}
                      <button
                        onclick={() => cardPhaseFilter = p}
                        class="rounded-md px-2 py-0.5 text-[10px] font-medium transition-colors
                               {cardPhaseFilter === p ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
                      >{p.slice(0,4)}</button>
                    {/each}
                  </div>
                </div>
                <div class="flex-1 overflow-y-auto pb-3">
                  {#each project.cards as card (card.id)}
                    <button
                      onclick={() => selectedCardId = card.id}
                      class="flex w-full items-start gap-2.5 border-b border-surface-z0/30 px-3 py-2.5 text-left transition-colors
                             {selectedCardId === card.id ? 'bg-primary-z1' : 'hover:bg-surface-z2/50'}"
                    >
                      <span class="mt-0.5 text-sm shrink-0 {kindIcon[card.kind] ?? ''} {kindColor[card.kind] ?? ''}"></span>
                      <div class="min-w-0 flex-1">
                        <p class="text-xs leading-snug text-surface-z8 line-clamp-2">{card.title}</p>
                        <p class="mt-0.5 text-[10px] text-surface-z4 capitalize">{card.kind} · {card.status}</p>
                      </div>
                    </button>
                  {/each}
                </div>
              </div>

              <!-- Card detail -->
              {#if selectedCard}
                <div class="flex flex-1 min-w-0 flex-col overflow-hidden">
                  <div class="flex-1 overflow-y-auto px-5 py-4">
                    <div class="flex items-center gap-2 mb-3">
                      <span class="text-lg {kindIcon[selectedCard.kind] ?? ''} {kindColor[selectedCard.kind] ?? ''}"></span>
                      <span class="text-xs text-surface-z5 capitalize">{selectedCard.kind}</span>
                      <span class="ml-auto text-xs text-surface-z4 capitalize">{selectedCard.status}</span>
                    </div>
                    <h3 class="text-sm font-semibold leading-snug text-surface-z9 mb-3">{selectedCard.title}</h3>
                    <p class="mt-3 text-sm leading-relaxed text-surface-z6">{selectedCard.body}</p>
                    {#if selectedCard.linkedSymbols > 0}
                      <div class="mt-5">
                        <p class="mb-2 text-xs font-semibold uppercase tracking-wide text-surface-z4">Linked symbols</p>
                        <div class="space-y-1.5">
                          {#each project.godNodes.slice(0, selectedCard.linkedSymbols) as node}
                            <div class="flex items-center gap-2 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-2">
                              <span class="i-solar-code-circle-bold-duotone text-sm text-primary-z5"></span>
                              <span class="font-mono text-xs text-surface-z7">{node.name}</span>
                              <span class="ml-auto text-[10px] text-surface-z4">{node.community}</span>
                            </div>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  </div>
                </div>
              {:else}
                <div class="flex flex-1 items-center justify-center">
                  <div class="text-center">
                    <span class="i-solar-document-add-bold-duotone text-3xl text-surface-z3 block mx-auto mb-2"></span>
                    <p class="text-xs text-surface-z5">Select a card</p>
                  </div>
                </div>
              {/if}
            </div>

          {:else if activeTab === 'graph'}
            <div class="flex-1 overflow-y-auto px-5 py-4">
              {#if project.communities.length > 0}
                <div class="grid grid-cols-2 gap-4">
                  <div>
                    <p class="mb-3 text-xs text-surface-z4">Communities ({project.communities.length})</p>
                    <div class="space-y-2">
                      {#each project.communities as c (c.id)}
                        <div class="flex items-center gap-2.5 rounded-xl border border-surface-z3/50 bg-surface-z2/50 px-3 py-2.5">
                          <div class="h-2.5 w-2.5 rounded-full shrink-0 {c.color}"></div>
                          <span class="flex-1 text-sm text-surface-z7">{c.label}</span>
                          <span class="text-xs text-surface-z4">{c.symbolCount} sym</span>
                        </div>
                      {/each}
                    </div>
                  </div>
                  <div>
                    <p class="mb-3 text-xs text-surface-z4">God nodes</p>
                    <div class="space-y-2">
                      {#each project.godNodes as node (node.name)}
                        <div class="rounded-xl border border-surface-z3/50 bg-surface-z2/50 px-3 py-2.5">
                          <div class="flex items-center gap-2">
                            <span class="i-solar-star-bold-duotone text-sm text-warning-z6"></span>
                            <span class="font-mono text-sm text-surface-z8">{node.name}</span>
                            <span class="ml-auto text-xs text-surface-z4">deg {node.degree}</span>
                          </div>
                          <p class="mt-0.5 text-xs text-surface-z5">{node.community}</p>
                        </div>
                      {/each}
                    </div>
                  </div>
                </div>
                {#if project.rationale.length > 0}
                  <div class="mt-5">
                    <p class="mb-3 text-xs text-surface-z4">Rationale nodes</p>
                    <div class="space-y-2">
                      {#each project.rationale as r (r.file)}
                        <div class="rounded-xl border border-surface-z3/50 bg-surface-z2/50 px-3 py-2.5">
                          <div class="flex items-center gap-2 mb-1">
                            <span class="rounded bg-info-z2 px-1.5 py-0.5 text-[9px] font-bold text-info-z7">{r.tag}</span>
                            <span class="font-mono text-[10px] text-surface-z4 truncate">{r.file.split('/').pop()}</span>
                          </div>
                          <p class="text-xs leading-relaxed text-surface-z6">{r.text}</p>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}
              {:else}
                <div class="flex h-full items-center justify-center">
                  <div class="text-center">
                    <span class="i-solar-graph-up-bold-duotone text-3xl text-surface-z3 block mx-auto mb-2"></span>
                    <p class="text-sm text-surface-z5">No graph data yet</p>
                    <p class="mt-1 text-xs text-surface-z4">Index this repo to generate the graph</p>
                  </div>
                </div>
              {/if}
            </div>

          {:else}
            <!-- Sessions tab -->
            <div class="flex-1 overflow-y-auto">
              {#if !project.sessions?.length}
                <div class="flex h-full items-center justify-center">
                  <div class="text-center">
                    <span class="i-solar-history-bold-duotone text-3xl text-surface-z3 block mx-auto mb-2"></span>
                    <p class="text-sm text-surface-z5">No sessions yet</p>
                  </div>
                </div>
              {:else}
                {#each project.sessions as s (s.id)}
                  <div class="flex items-center gap-4 border-b border-surface-z0/30 px-5 py-3">
                    <div class="min-w-0 flex-1">
                      <p class="truncate text-sm text-surface-z8">{s.task}</p>
                      <p class="mt-0.5 text-xs text-surface-z4">{s.when} · {s.turns} turns</p>
                    </div>
                    <span class="flex items-center gap-1 text-xs text-surface-z5">
                      <span class="h-1.5 w-1.5 rounded-full {s.status === 'completed' ? 'bg-success-z5' : 'bg-primary-z6 animate-pulse'}"></span>
                      {s.status}
                    </span>
                    <span class="w-14 text-right text-xs font-mono {s.ftr === null ? 'text-surface-z4' : s.ftr >= 0.8 ? 'text-success-z6' : s.ftr >= 0.5 ? 'text-warning-z6' : 'text-error-z6'}">
                      {s.ftr !== null ? `${Math.round(s.ftr * 100)}%` : '—'}
                    </span>
                    <span class="w-14 text-right text-xs text-surface-z5">${s.cost.toFixed(2)}</span>
                  </div>
                {/each}
              {/if}
            </div>
          {/if}

          <!-- Prompt bar -->
          <div class="border-t border-surface-z0/50 bg-surface-z2/60 px-4 py-2.5 backdrop-blur-sm shrink-0">
            <div class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-3 py-2 focus-within:border-primary-z4 focus-within:ring-1 focus-within:ring-primary-z4/30 transition-all">
              <span class="i-solar-magic-stick-3-bold-duotone text-sm text-primary-z6 shrink-0"></span>
              <input
                bind:value={prompt}
                placeholder="Ask about {project.name}… or type / for commands"
                class="flex-1 bg-transparent text-sm text-surface-z7 outline-none placeholder:text-surface-z4"
              />
              <kbd class="rounded border border-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z4">⏎</kbd>
            </div>
            <div class="mt-1.5 flex gap-3 px-1">
              {#each ['/gap-analysis', '/analyze-repo', '/decision-log', '/token-estimate'] as cmd}
                <button class="text-[10px] text-surface-z4 hover:text-primary-z6">{cmd}</button>
              {/each}
            </div>
          </div>
        </div>
      {:else}
        <div class="flex flex-1 items-center justify-center">
          <div class="text-center space-y-3">
            <span class="i-solar-planets-bold-duotone text-3xl text-surface-z3 block mx-auto"></span>
            <p class="text-sm text-surface-z5">No projects yet</p>
            <button
              onclick={() => { showImport = true; importPhase = 'idle'; }}
              class="rounded-lg bg-primary-z6 px-4 py-2 text-xs font-semibold text-white hover:bg-primary-z7 transition-colors"
            >Import your first project</button>
          </div>
        </div>
      {/if}
    </div>

  {:else}
    <!-- ══ BOARD VIEW ════════════════════════════════════════════════ -->
    <div class="flex-1 overflow-y-auto px-5 py-4">
      <div class="grid grid-cols-2 gap-3 xl:grid-cols-3">
        {#each filteredProjects as p (p.id)}
          <button
            onclick={() => { selectProject(p.id); viewMode = 'split'; }}
            class="rounded-2xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-4 text-left transition-all hover:border-surface-z4 hover:bg-surface-z2"
          >
            <div class="flex items-start justify-between mb-2">
              <div class="flex items-center gap-2">
                <span class="text-lg {p.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
                <span class="font-semibold text-surface-z8">{p.name}</span>
              </div>
              {#if p.language}
                <span class="text-[10px] text-surface-z4 rounded bg-surface-z3 px-1.5 py-0.5">{p.language}</span>
              {/if}
            </div>

            <p class="text-xs text-surface-z5 line-clamp-2 mb-3">{p.description}</p>

            <!-- Maturity bar -->
            <div class="flex items-center gap-2 mb-2">
              <div class="flex flex-1 gap-0.5">
                {#each Array(5) as _, i}
                  <div class="h-1.5 flex-1 rounded-full {i < p.maturity ? maturityBg[p.maturity] : 'bg-surface-z3'}"></div>
                {/each}
              </div>
              <span class="text-[10px] text-surface-z5">{maturityLabel[p.maturity]}</span>
            </div>

            <!-- Phase pipeline mini -->
            <div class="flex gap-1 mb-3">
              {#each p.phases as phase}
                <div class="flex-1 text-center">
                  <div class="h-0.5 w-full rounded-full {phase.done ? 'bg-success-z5' : phase.active ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
                </div>
              {/each}
            </div>

            <div class="flex items-center justify-between text-[10px] text-surface-z4">
              <span>{p.sessionCount} sessions · {p.cardCount} cards</span>
              {#if p.ftrScore != null}
                <span class="rounded-full bg-success-z2 px-1.5 py-0.5 text-[10px] font-semibold text-success-z7">FTR {Math.round(p.ftrScore * 100)}%</span>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/if}

</div>

<!-- ══ IMPORT PANEL ══════════════════════════════════════════════════ -->
{#if showImport}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-40 bg-surface-z9/20 backdrop-blur-sm"
    onclick={closeImport}
    onkeydown={(e) => e.key === 'Escape' && closeImport()}
  ></div>

  <!-- Panel -->
  <div class="fixed right-0 top-0 bottom-0 z-50 flex w-96 flex-col border-l border-surface-z3 bg-surface-z1 shadow-2xl">
    <div class="flex items-center justify-between border-b border-surface-z0/50 px-5 py-4 shrink-0">
      <h2 class="text-sm font-semibold text-surface-z8">Import project</h2>
      <button onclick={closeImport} class="text-surface-z4 hover:text-surface-z7 transition-colors">
        <span class="i-solar-close-circle-bold-duotone text-lg"></span>
      </button>
    </div>

    <div class="flex-1 overflow-y-auto px-5 py-5">

      {#if importPhase === 'idle'}
        <div class="space-y-4">
          <p class="text-sm text-surface-z5">
            Choose a repository folder. Sensei will scan files, extract symbols, build a graph, and detect libraries.
          </p>

          <div>
            <label for="import-path" class="mb-1.5 block text-xs text-surface-z5">Folder path</label>
            <div class="flex gap-2">
              <input
                id="import-path"
              value={importPath}
              oninput={onImportPath}
                placeholder="~/Developer/my-project"
                class="min-w-0 flex-1 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-2 font-mono text-xs text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4"
              />
              <button
                onclick={() => { importPath = '~/Developer/my-project'; importName = 'my-project'; }}
                aria-label="Browse folders"
              class="rounded-lg border border-surface-z3 bg-surface-z2 px-3 text-surface-z5 hover:bg-surface-z3 transition-colors"
              title="Browse"
            >
                <span class="i-solar-folder-open-bold-duotone text-base"></span>
              </button>
            </div>
          </div>

          {#if importName}
            <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3 flex items-center gap-3">
              <span class="i-solar-code-square-bold-duotone text-xl text-primary-z5"></span>
              <div>
                <p class="text-sm font-medium text-surface-z8">{importName}</p>
                <p class="text-xs text-surface-z4">{importPath}</p>
              </div>
            </div>
          {/if}

          <button
            onclick={startImport}
            disabled={!importPath}
            class="w-full rounded-xl py-2.5 text-sm font-semibold transition-colors
                   {importPath ? 'bg-primary-z6 text-white hover:bg-primary-z7' : 'bg-surface-z3 text-surface-z4 cursor-not-allowed'}"
          >Start import</button>
        </div>

      {:else if importPhase === 'indexing'}
        <div class="space-y-5">
          <div>
            <p class="text-sm font-medium text-surface-z8 mb-1">Indexing {importName}…</p>
            <div class="h-1 w-full rounded-full bg-surface-z3 overflow-hidden">
              <div class="h-full rounded-full bg-primary-z6 transition-all duration-150" style="width: {importProgress}%"></div>
            </div>
          </div>
          <div class="space-y-3">
            {#each importSteps as s, i}
              {@const done = i < importStep || importProgress >= 100}
              {@const active = i === importStep && importProgress < 100}
              <div class="flex items-center gap-3 text-sm {done ? 'text-surface-z6' : active ? 'text-surface-z7' : 'text-surface-z3'}">
                {#if done}
                  <span class="i-solar-check-circle-bold-duotone text-success-z5 text-base shrink-0"></span>
                {:else if active}
                  <span class="i-solar-refresh-bold-duotone text-primary-z6 text-base shrink-0 animate-spin"></span>
                {:else}
                  <span class="h-4 w-4 rounded-full border border-surface-z3 shrink-0"></span>
                {/if}
                <span class="flex-1">{s}</span>
                {#if done}<span class="text-xs text-surface-z4">{importResults[i]}</span>{/if}
              </div>
            {/each}
          </div>
        </div>

      {:else}
        <!-- Done -->
        <div class="space-y-5">
          <div class="text-center py-4">
            <div class="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-success-z2 text-xl mb-3">✓</div>
            <p class="text-sm font-semibold text-surface-z8">{importName} imported</p>
            <p class="text-xs text-surface-z5 mt-1">Ready to explore</p>
          </div>
          <div class="grid grid-cols-2 gap-2">
            {#each [['8,432', 'Symbols'], ['5', 'Communities'], ['6', 'Libraries'], ['3,241', 'Files']] as [v, l]}
              <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-3 py-2.5 text-center">
                <p class="text-lg font-semibold text-surface-z8">{v}</p>
                <p class="text-xs text-surface-z4">{l}</p>
              </div>
            {/each}
          </div>
          <button onclick={closeImport} class="w-full rounded-xl bg-primary-z6 py-2.5 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors">
            View project
          </button>
        </div>
      {/if}

    </div>
  </div>
{/if}
