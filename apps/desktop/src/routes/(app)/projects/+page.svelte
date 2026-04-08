<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  const maturityLabels = ['Seed', 'Exploring', 'Developing', 'Maturing', 'Established', 'Mature'];
  const maturityColors = [
    'bg-surface-z3 text-surface-z5',
    'bg-info-z2 text-info-z7',
    'bg-warning-z2 text-warning-z7',
    'bg-secondary-z2 text-secondary-z7',
    'bg-success-z2 text-success-z7',
    'bg-primary-z2 text-primary-z7',
  ];

  let selected = $state<string | null>(null);
  let filter = $state<'all' | 'repo' | 'idea'>('all');
  let search = $state('');

  let filtered = $derived(
    data.projects.filter((p: { kind: string; name: string; description: string }) => {
      if (filter !== 'all' && p.kind !== filter) return false;
      if (search && !p.name.toLowerCase().includes(search.toLowerCase()) &&
          !p.description.toLowerCase().includes(search.toLowerCase())) return false;
      return true;
    })
  );

  let selectedProject = $derived(data.projects.find((p: { id: string }) => p.id === selected));
</script>

<div class="flex h-full min-h-0">

  <!-- Project list -->
  <div class="flex w-72 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">

    <!-- Header + search -->
    <div class="px-4 pb-3 pt-4">
      <div class="mb-3 flex items-center justify-between">
        <h1 class="text-sm font-semibold text-surface-z8">Projects</h1>
        <button aria-label="New project" class="flex h-6 w-6 items-center justify-center rounded-md text-surface-z5 transition-colors hover:bg-surface-z3 hover:text-surface-z8">
          <span class="i-solar-add-circle-bold-duotone text-base"></span>
        </button>
      </div>

      <!-- Search -->
      <div class="relative">
        <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-sm i-solar-magnifer-bold-duotone text-surface-z4"></span>
        <input
          bind:value={search}
          type="text"
          placeholder="Search projects…"
          class="w-full rounded-lg border border-surface-z3 bg-surface-z2 py-1.5 pl-8 pr-3 text-xs text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z5 focus:ring-1 focus:ring-primary-z5/30"
        />
      </div>

      <!-- Filter tabs -->
      <div class="mt-2.5 flex gap-1">
        {#each (['all', 'repo', 'idea'] as const) as f}
          <button
            onclick={() => filter = f}
            class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors capitalize
                   {filter === f ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
          >
            {f === 'all' ? 'All' : f === 'repo' ? 'Repos' : 'Ideas'}
          </button>
        {/each}
      </div>
    </div>

    <!-- List -->
    <div class="flex-1 overflow-y-auto px-2 pb-4 space-y-0.5">
      {#each filtered as project (project.id)}
        <button
          onclick={() => selected = project.id}
          class="w-full rounded-xl px-3 py-2.5 text-left transition-colors
                 {selected === project.id ? 'bg-primary-z2' : 'hover:bg-surface-z2/80'}"
        >
          <div class="flex items-start justify-between gap-2">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <span class="text-sm
                  {project.kind === 'idea'
                    ? 'i-solar-lightbulb-bold-duotone text-warning-z6'
                    : 'i-solar-code-square-bold-duotone text-primary-z6'}
                "></span>
                <span class="truncate text-sm font-medium text-surface-z8">{project.name}</span>
              </div>
              <p class="mt-0.5 truncate text-xs text-surface-z4">{project.description}</p>
            </div>
          </div>

          <!-- Maturity bar -->
          <div class="mt-2 flex items-center gap-2">
            <div class="flex flex-1 gap-0.5">
              {#each Array(5) as _, i}
                <div class="h-1 flex-1 rounded-full {i < project.maturity ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
              {/each}
            </div>
            <span class="text-[10px] {maturityColors[project.maturity]} rounded-full px-1.5 py-0.5 font-medium">
              {maturityLabels[project.maturity]}
            </span>
          </div>

          <div class="mt-1.5 flex items-center gap-3 text-[10px] text-surface-z4">
            {#if project.language}
              <span>{project.language}</span>
            {/if}
            <span>{project.activePhase}</span>
            <span class="ml-auto">{project.lastActivity}</span>
          </div>
        </button>
      {/each}
    </div>
  </div>

  <!-- Detail panel -->
  <div class="flex flex-1 flex-col min-w-0 overflow-hidden">
    {#if selectedProject}
      <div class="flex-1 overflow-y-auto px-6 py-5">

        <!-- Header -->
        <div class="mb-6 flex items-start justify-between">
          <div>
            <div class="flex items-center gap-2">
              <span class="text-xl
                {selectedProject.kind === 'idea'
                  ? 'i-solar-lightbulb-bold-duotone text-warning-z6'
                  : 'i-solar-code-square-bold-duotone text-primary-z6'}
              "></span>
              <h2 class="text-lg font-semibold text-surface-z9">{selectedProject.name}</h2>
            </div>
            {#if selectedProject.path}
              <p class="mt-1 font-mono text-xs text-surface-z4">{selectedProject.path}</p>
            {/if}
            <p class="mt-2 text-sm text-surface-z6">{selectedProject.description}</p>
          </div>
          <button class="rounded-lg border border-surface-z3 px-3 py-1.5 text-xs font-medium text-surface-z7 transition-colors hover:bg-surface-z3">
            Open in Claude Code
          </button>
        </div>

        <!-- Stats row -->
        <div class="mb-6 grid grid-cols-4 gap-3">
          {#each [
            { label: 'Sessions',    value: selectedProject.sessionCount },
            { label: 'Cards',       value: selectedProject.cardCount },
            { label: 'Symbols',     value: selectedProject.symbolCount?.toLocaleString() ?? '—' },
            { label: 'FTR Score',   value: selectedProject.ftrScore != null ? `${Math.round(selectedProject.ftrScore * 100)}%` : '—' },
          ] as stat}
            <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3">
              <p class="text-xs text-surface-z4">{stat.label}</p>
              <p class="mt-0.5 text-lg font-semibold text-surface-z8">{stat.value}</p>
            </div>
          {/each}
        </div>

        <!-- Phase pipeline -->
        <div class="mb-6">
          <p class="mb-3 text-xs font-semibold uppercase tracking-wide text-surface-z4">Phase pipeline</p>
          <div class="flex gap-1">
            {#each ['Requirements', 'Analysis', 'Design', 'Implementation', 'Review'] as phase}
              <div class="flex flex-1 flex-col items-center gap-1.5">
                <div class="h-1.5 w-full rounded-full {phase === selectedProject.activePhase ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
                <span class="text-[10px] {phase === selectedProject.activePhase ? 'font-semibold text-primary-z7' : 'text-surface-z4'}">{phase}</span>
              </div>
            {/each}
          </div>
        </div>

        <!-- Cards placeholder -->
        <div>
          <div class="mb-3 flex items-center justify-between">
            <p class="text-xs font-semibold uppercase tracking-wide text-surface-z4">Recent cards</p>
            <button class="text-xs text-primary-z6 hover:text-primary-z7">View all</button>
          </div>
          {#if selectedProject.cardCount > 0}
            <div class="space-y-2">
              {#each [
                { kind: 'decision', title: 'Use SQLite as default — Postgres optional', tag: 'DECISION' },
                { kind: 'requirement', title: 'Repo discovery scans ~/Developer and ~/Work', tag: 'REQ' },
                { kind: 'note', title: 'Coordinator adapter abstracts Claude/opencode/Copilot', tag: 'NOTE' },
              ] as card}
                <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3">
                  <div class="flex items-start gap-2">
                    <span class="mt-0.5 rounded-md bg-primary-z2 px-1.5 py-0.5 text-[9px] font-bold text-primary-z7">{card.tag}</span>
                    <p class="text-sm text-surface-z7">{card.title}</p>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class="rounded-xl border border-dashed border-surface-z3 px-4 py-8 text-center">
              <p class="text-sm text-surface-z5">No cards yet</p>
              <p class="mt-1 text-xs text-surface-z4">Add requirements, decisions, and notes to build context</p>
            </div>
          {/if}
        </div>

      </div>

      <!-- Prompt bar -->
      <div class="border-t border-surface-z0/50 bg-surface-z2/50 px-4 py-3 backdrop-blur-sm">
        <div class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z2 px-3 py-2">
          <span class="i-solar-magic-stick-3-bold-duotone text-base text-primary-z6"></span>
          <input
            type="text"
            placeholder="Ask about {selectedProject.name}… (⌘K to focus)"
            class="flex-1 bg-transparent text-sm text-surface-z7 outline-none placeholder:text-surface-z4"
          />
          <kbd class="rounded border border-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z4">⏎</kbd>
        </div>
      </div>

    {:else}
      <!-- Empty state -->
      <div class="flex flex-1 items-center justify-center">
        <div class="text-center">
          <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-surface-z3">
            <span class="i-solar-planets-bold-duotone text-2xl text-surface-z5"></span>
          </div>
          <p class="text-sm font-medium text-surface-z6">Select a project</p>
          <p class="mt-1 text-xs text-surface-z4">Choose a project from the list to view details</p>
        </div>
      </div>
    {/if}
  </div>

</div>
