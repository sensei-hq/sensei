<!--
  Mockup A — Three-pane workspace
  Pane 1: Project list (narrow sidebar)
  Pane 2: Phase pipeline + card list (center)
  Pane 3: Card detail + prompt (right panel)
-->
<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  let selectedCardId = $state<string | null>(null);
  let promptValue = $state('');

  const maturityColors = [
    'bg-surface-z3',
    'bg-info-z5',
    'bg-warning-z5',
    'bg-secondary-z5',
    'bg-success-z5',
    'bg-primary-z6',
  ];

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

  let selectedCard = $derived(
    data.activeProject.cards.find((c: { id: string }) => c.id === selectedCardId)
  );
</script>

<div class="flex h-full min-h-0 text-surface-z7">

  <!-- ── Pane 1: Project list ──────────────────────────────────── -->
  <div class="flex w-44 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">
    <div class="px-3 pb-2 pt-3">
      <p class="text-[9px] font-semibold uppercase tracking-widest text-surface-z4">All projects</p>
    </div>
    <div class="flex-1 overflow-y-auto px-1.5 pb-3 space-y-0.5">
      {#each data.projects as proj (proj.id)}
        <button
          class="w-full rounded-lg px-2.5 py-2 text-left transition-colors
                 {proj.id === data.activeProject.id ? 'bg-primary-z2' : 'hover:bg-surface-z2/70'}"
        >
          <div class="flex items-center gap-1.5">
            <span class="text-sm {proj.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
            <span class="min-w-0 flex-1 truncate text-xs font-medium text-surface-z8">{proj.name}</span>
            {#if proj.unread > 0}
              <span class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-primary-z6 text-[9px] font-bold text-white">{proj.unread}</span>
            {/if}
          </div>
          <div class="mt-1.5 flex gap-0.5">
            {#each Array(5) as _, i}
              <div class="h-0.5 flex-1 rounded-full {i < proj.maturity ? maturityColors[proj.maturity] : 'bg-surface-z3'}"></div>
            {/each}
          </div>
          <p class="mt-1 text-[10px] text-surface-z4">{proj.phase}</p>
        </button>
      {/each}
    </div>
  </div>

  <!-- ── Pane 2: Phase pipeline + cards ───────────────────────── -->
  <div class="flex w-72 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">

    <!-- Project header -->
    <div class="border-b border-surface-z0/50 px-4 py-3">
      <div class="flex items-center gap-2">
        <span class="i-solar-code-square-bold-duotone text-base text-primary-z6"></span>
        <span class="font-semibold text-surface-z9">{data.activeProject.name}</span>
      </div>
      <p class="mt-0.5 truncate text-xs text-surface-z4">{data.activeProject.path}</p>
    </div>

    <!-- Phase strip -->
    <div class="border-b border-surface-z0/50 px-4 py-3">
      <p class="mb-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Phases</p>
      <div class="flex gap-1">
        {#each data.activeProject.phases as phase (phase.name)}
          <div class="flex flex-1 flex-col items-center gap-1">
            <div class="h-1 w-full rounded-full relative overflow-hidden bg-surface-z3">
              {#if phase.done || phase.active}
                <div class="absolute inset-0 rounded-full {phase.done ? 'bg-success-z5' : 'bg-primary-z6'}"></div>
              {/if}
            </div>
            <span class="text-[9px] leading-none {phase.active ? 'font-bold text-primary-z7' : phase.done ? 'text-success-z6' : 'text-surface-z3'}">{phase.name.slice(0,4)}</span>
          </div>
        {/each}
      </div>
    </div>

    <!-- Cards -->
    <div class="flex-1 overflow-y-auto px-3 py-2 space-y-1">
      <div class="mb-1 flex items-center justify-between px-1">
        <p class="text-[9px] font-semibold uppercase tracking-widest text-surface-z4">
          Implementation · {data.activeProject.phases.find((p: { active?: boolean }) => p.active)?.cardCount ?? 0} cards
        </p>
        <button class="text-[10px] text-primary-z6">+ Add</button>
      </div>

      {#each data.activeProject.cards as card (card.id)}
        <button
          onclick={() => selectedCardId = card.id}
          class="w-full rounded-xl border px-3 py-2.5 text-left transition-all
                 {selectedCardId === card.id
                   ? 'border-primary-z4 bg-primary-z1'
                   : 'border-surface-z3/50 bg-surface-z2/40 hover:border-surface-z4 hover:bg-surface-z2'}"
        >
          <div class="flex items-start gap-2">
            <span class="mt-0.5 text-sm {kindIcon[card.kind] ?? 'i-solar-document-bold-duotone'} {kindColor[card.kind] ?? 'text-surface-z5'} shrink-0"></span>
            <div class="min-w-0 flex-1">
              <p class="text-xs font-medium leading-snug text-surface-z8 line-clamp-2">{card.title}</p>
              {#if card.linkedSymbols > 0}
                <span class="mt-1 inline-flex items-center gap-1 text-[10px] text-surface-z4">
                  <span class="i-solar-link-bold-duotone text-xs"></span>
                  {card.linkedSymbols} symbols
                </span>
              {/if}
            </div>
            <span class="shrink-0 rounded-md px-1.5 py-0.5 text-[9px] font-bold
              {card.status === 'done' ? 'bg-success-z2 text-success-z7' :
               card.status === 'active' ? 'bg-primary-z2 text-primary-z7' :
               'bg-surface-z2 text-surface-z5'}">
              {card.status}
            </span>
          </div>
        </button>
      {/each}
    </div>

    <!-- Sessions mini list -->
    <div class="border-t border-surface-z0/50 px-3 py-2.5">
      <p class="mb-1.5 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Recent sessions</p>
      {#each data.sessions.slice(0, 2) as s (s.id)}
        <div class="flex items-center gap-2 py-1">
          <span class="h-1.5 w-1.5 rounded-full shrink-0
            {s.status === 'completed' ? 'bg-success-z5' :
             s.status === 'in-progress' ? 'bg-primary-z6 animate-pulse' : 'bg-surface-z4'}
          "></span>
          <span class="min-w-0 flex-1 truncate text-[10px] text-surface-z6">{s.task}</span>
          <span class="shrink-0 text-[10px] text-surface-z4">{s.when}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- ── Pane 3: Card detail + prompt ─────────────────────────── -->
  <div class="flex flex-1 flex-col min-w-0 overflow-hidden">

    {#if selectedCard}
      <div class="flex-1 overflow-y-auto px-6 py-5">
        <!-- Card header -->
        <div class="mb-1 flex items-center gap-2">
          <span class="text-xl {kindIcon[selectedCard.kind] ?? 'i-solar-document-bold-duotone'} {kindColor[selectedCard.kind] ?? 'text-surface-z5'}"></span>
          <span class="rounded-md bg-surface-z3 px-2 py-0.5 text-xs font-bold text-surface-z7">{selectedCard.tag}</span>
          <span class="ml-auto rounded-full px-2 py-0.5 text-xs font-medium
            {selectedCard.status === 'done' ? 'bg-success-z2 text-success-z7' :
             selectedCard.status === 'active' ? 'bg-primary-z2 text-primary-z7' :
             'bg-surface-z2 text-surface-z5'}">
            {selectedCard.status}
          </span>
        </div>
        <h2 class="mt-2 text-base font-semibold leading-snug text-surface-z9">{selectedCard.title}</h2>
        <p class="mt-3 text-sm leading-relaxed text-surface-z6">{selectedCard.body}</p>

        {#if selectedCard.linkedSymbols > 0}
          <div class="mt-5">
            <p class="mb-2 text-xs font-semibold uppercase tracking-wide text-surface-z4">Linked symbols</p>
            <div class="space-y-1.5">
              {#each Array(selectedCard.linkedSymbols) as _, i}
                <div class="flex items-center gap-2 rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-2">
                  <span class="i-solar-code-circle-bold-duotone text-sm text-primary-z5"></span>
                  <span class="font-mono text-xs text-surface-z7">
                    {['CoordinatorAdapter', 'CoordinatorRegistry', 'setupAgent', 'AgentAdapter', 'claude-adapter'][i] ?? `symbol_${i}`}
                  </span>
                  <span class="ml-auto text-[10px] text-surface-z4">packages/engine</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Related cards -->
        <div class="mt-5">
          <p class="mb-2 text-xs font-semibold uppercase tracking-wide text-surface-z4">Related</p>
          <div class="space-y-1.5">
            {#each data.activeProject.cards.filter((c: { id: string }) => c.id !== selectedCard?.id).slice(0, 2) as rel (rel.id)}
              <button
                onclick={() => selectedCardId = rel.id}
                class="flex w-full items-center gap-2 rounded-lg border border-surface-z3/50 bg-surface-z2/50 px-3 py-2 text-left hover:border-surface-z4"
              >
                <span class="text-sm {kindIcon[rel.kind] ?? ''} {kindColor[rel.kind] ?? ''} shrink-0"></span>
                <span class="min-w-0 flex-1 truncate text-xs text-surface-z7">{rel.title}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>

    {:else}
      <div class="flex flex-1 items-center justify-center">
        <div class="text-center">
          <div class="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-2xl bg-surface-z3">
            <span class="i-solar-document-add-bold-duotone text-xl text-surface-z5"></span>
          </div>
          <p class="text-sm font-medium text-surface-z6">Select a card</p>
          <p class="mt-1 text-xs text-surface-z4">Pick a card from the pipeline to view details</p>
        </div>
      </div>
    {/if}

    <!-- Prompt bar -->
    <div class="border-t border-surface-z0/50 bg-surface-z2/60 px-4 py-3 backdrop-blur-sm">
      <div class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-3 py-2.5 focus-within:border-primary-z4 focus-within:ring-1 focus-within:ring-primary-z4/30 transition-all">
        <span class="i-solar-magic-stick-3-bold-duotone text-base text-primary-z6 shrink-0"></span>
        <input
          bind:value={promptValue}
          type="text"
          placeholder={selectedCard ? `Ask about this card… or type / for commands` : `Ask anything about ${data.activeProject.name}… type / for commands`}
          class="flex-1 bg-transparent text-sm text-surface-z7 outline-none placeholder:text-surface-z4"
        />
        <div class="flex items-center gap-1.5 shrink-0">
          <span class="text-[10px] text-surface-z4">@sensei</span>
          <kbd class="rounded border border-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z4">⏎</kbd>
        </div>
      </div>
      <div class="mt-1.5 flex items-center gap-3 px-1">
        <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/gap-analysis</button>
        <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/analyze-repo</button>
        <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/decision-log</button>
        <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/session-recap</button>
        <span class="ml-auto text-[10px] text-surface-z4">Claude Code · sensei</span>
      </div>
    </div>
  </div>

</div>
