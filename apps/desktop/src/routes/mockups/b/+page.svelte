<!--
  Mockup B — Card workspace + graph intelligence
  Left: Cards in a masonry-style grid (main area)
  Right: Graph intelligence panel (communities, god nodes, rationale)
  Bottom: Persistent prompt bar
-->
<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  let activeFilter = $state<string>('all');
  let promptValue = $state('');
  let inspectNode = $state<string | null>(null);

  const phaseFilters = ['all', 'Requirements', 'Analysis', 'Design', 'Implementation'];

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

  const tagColor: Record<string, string> = {
    decision:    'bg-primary-z2 text-primary-z7',
    requirement: 'bg-success-z2 text-success-z7',
    task:        'bg-warning-z2 text-warning-z7',
    note:        'bg-info-z2 text-info-z7',
    question:    'bg-danger-z2 text-danger-z7',
    finding:     'bg-secondary-z2 text-secondary-z7',
  };

  let filteredCards = $derived(
    activeFilter === 'all'
      ? data.cards
      : data.cards.filter((c: { phase: string }) => c.phase === activeFilter)
  );
</script>

<div class="flex h-full min-h-0 flex-col">

  <!-- Top bar -->
  <div class="flex items-center justify-between border-b border-surface-z0/50 px-5 py-2.5">
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-2">
        <span class="i-solar-code-square-bold-duotone text-base text-primary-z6"></span>
        <span class="font-semibold text-surface-z9">{data.project.name}</span>
        <span class="text-surface-z4">/</span>
        <span class="text-sm text-surface-z6">{data.project.phase}</span>
      </div>

      <!-- Maturity bar -->
      <div class="flex items-center gap-1.5">
        <div class="flex gap-0.5">
          {#each Array(5) as _, i}
            <div class="h-1.5 w-4 rounded-full {i < data.project.maturity ? 'bg-primary-z6' : 'bg-surface-z3'}"></div>
          {/each}
        </div>
        <span class="text-xs text-surface-z5">Maturing</span>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <span class="rounded-full bg-success-z2 px-2.5 py-0.5 text-xs font-medium text-success-z7">
        FTR {Math.round(data.project.ftrScore * 100)}%
      </span>
      <button class="flex items-center gap-1.5 rounded-lg border border-surface-z3 px-2.5 py-1.5 text-xs text-surface-z7 transition-colors hover:bg-surface-z3">
        <span class="i-solar-add-circle-bold-duotone text-sm"></span>
        New card
      </button>
    </div>
  </div>

  <!-- Phase filter tabs -->
  <div class="flex items-center gap-1 border-b border-surface-z0/50 px-4 py-1.5">
    {#each phaseFilters as f}
      <button
        onclick={() => activeFilter = f}
        class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors capitalize
               {activeFilter === f ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
      >
        {f === 'all' ? 'All cards' : f}
      </button>
    {/each}
  </div>

  <!-- Content area -->
  <div class="flex flex-1 min-h-0 overflow-hidden">

    <!-- ── Cards grid ────────────────────────────────────────────── -->
    <div class="flex-1 overflow-y-auto px-4 py-4 min-w-0">
      <div class="columns-2 gap-3 xl:columns-3">
        {#each filteredCards as card (card.id)}
          <div class="mb-3 break-inside-avoid rounded-xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-3.5 transition-all hover:border-surface-z4 hover:bg-surface-z2">
            <!-- Card header -->
            <div class="mb-2.5 flex items-start gap-2">
              <span class="mt-0.5 text-sm {kindIcon[card.kind] ?? ''} {kindColor[card.kind] ?? ''} shrink-0"></span>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-1.5 flex-wrap">
                  <span class="rounded-md px-1.5 py-0.5 text-[9px] font-bold {tagColor[card.kind] ?? 'bg-surface-z2 text-surface-z6'}">{card.tag}</span>
                  <span class="text-[10px] text-surface-z4">{card.phase}</span>
                  <span class="ml-auto rounded-full px-2 py-0.5 text-[9px] font-medium
                    {card.status === 'done' ? 'bg-success-z2 text-success-z7' :
                     card.status === 'active' ? 'bg-primary-z2 text-primary-z7' :
                     'bg-surface-z2 text-surface-z4'}">
                    {card.status}
                  </span>
                </div>
                <p class="mt-1.5 text-sm font-medium leading-snug text-surface-z8">{card.title}</p>
              </div>
            </div>

            <p class="text-xs leading-relaxed text-surface-z5">{card.summary}</p>

            <div class="mt-3 flex items-center justify-between text-[10px] text-surface-z4">
              {#if card.linkedSymbols > 0}
                <span class="flex items-center gap-1">
                  <span class="i-solar-link-bold-duotone text-xs"></span>
                  {card.linkedSymbols} symbols
                </span>
              {:else}
                <span></span>
              {/if}
              <span>{card.updatedAt}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <!-- ── Graph intelligence panel ──────────────────────────────── -->
    <div class="flex w-64 shrink-0 flex-col border-l border-surface-z0/50 overflow-hidden">

      <div class="border-b border-surface-z0/50 px-4 py-3">
        <div class="flex items-center gap-2">
          <span class="i-solar-graph-up-bold-duotone text-sm text-primary-z6"></span>
          <span class="text-xs font-semibold text-surface-z8">Graph Intelligence</span>
        </div>
        <p class="mt-0.5 text-[10px] text-surface-z4">4,821 symbols · 5 communities</p>
      </div>

      <div class="flex-1 overflow-y-auto pb-4 space-y-5 px-4 pt-4">

        <!-- Communities -->
        <div>
          <p class="mb-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Communities</p>
          <div class="space-y-1.5">
            {#each data.project.communities as c (c.id)}
              <div class="flex items-center gap-2">
                <div class="h-2 w-2 rounded-full shrink-0 {c.color}"></div>
                <span class="min-w-0 flex-1 truncate text-xs text-surface-z7">{c.label}</span>
                <span class="text-[10px] text-surface-z4">{c.symbolCount}</span>
              </div>
            {/each}
          </div>
        </div>

        <!-- God nodes -->
        <div>
          <p class="mb-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">God nodes</p>
          <div class="space-y-1.5">
            {#each data.project.godNodes as node (node.name)}
              <button
                onclick={() => inspectNode = inspectNode === node.name ? null : node.name}
                class="w-full rounded-lg border px-2.5 py-2 text-left transition-colors
                       {inspectNode === node.name ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3/50 bg-surface-z2/40 hover:border-surface-z3'}"
              >
                <div class="flex items-center gap-2">
                  <span class="i-solar-star-bold-duotone text-xs text-warning-z6 shrink-0"></span>
                  <span class="min-w-0 flex-1 truncate font-mono text-xs text-surface-z8">{node.name}</span>
                  <span class="shrink-0 text-[10px] text-surface-z4">{node.degree}</span>
                </div>
                <p class="mt-0.5 text-[10px] text-surface-z4">{node.community}</p>
              </button>
            {/each}
          </div>
        </div>

        <!-- Rationale nodes -->
        <div>
          <p class="mb-2 text-[9px] font-semibold uppercase tracking-widest text-surface-z4">Rationale</p>
          <div class="space-y-2">
            {#each data.rationale as r (r.file)}
              <div class="rounded-lg border border-surface-z3/50 bg-surface-z2/40 px-2.5 py-2">
                <div class="flex items-center gap-1.5 mb-1">
                  <span class="rounded bg-info-z2 px-1 py-0.5 text-[9px] font-bold text-info-z7">{r.tag}</span>
                  <span class="min-w-0 flex-1 truncate font-mono text-[9px] text-surface-z4">{r.file.split(':')[0].split('/').pop()}:{r.file.split(':')[1]}</span>
                </div>
                <p class="text-[10px] leading-relaxed text-surface-z6 line-clamp-2">{r.text}</p>
              </div>
            {/each}
          </div>
        </div>

      </div>
    </div>
  </div>

  <!-- ── Prompt bar ─────────────────────────────────────────────── -->
  <div class="border-t border-surface-z0/50 bg-surface-z2/60 px-4 py-3 backdrop-blur-sm">
    <div class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-3 py-2.5 focus-within:border-primary-z4 focus-within:ring-1 focus-within:ring-primary-z4/30 transition-all">
      <span class="i-solar-magic-stick-3-bold-duotone text-base text-primary-z6 shrink-0"></span>
      <input
        bind:value={promptValue}
        type="text"
        placeholder="Ask about sensei… type / for commands (gap-analysis, trace, find-orphans…)"
        class="flex-1 bg-transparent text-sm text-surface-z7 outline-none placeholder:text-surface-z4"
      />
      <div class="flex items-center gap-1.5 shrink-0">
        <button class="rounded-md bg-surface-z3 px-2 py-1 text-[10px] text-surface-z6 hover:bg-surface-z4">@card</button>
        <button class="rounded-md bg-surface-z3 px-2 py-1 text-[10px] text-surface-z6 hover:bg-surface-z4">@symbol</button>
        <kbd class="rounded border border-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z4">⏎</kbd>
      </div>
    </div>
    <div class="mt-1.5 flex items-center gap-3 px-1">
      <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/gap-analysis</button>
      <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/trace CoordinatorAdapter</button>
      <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/find-orphans</button>
      <button class="text-[10px] text-surface-z4 hover:text-surface-z6">/decision-log</button>
      <span class="ml-auto text-[10px] text-surface-z4">Claude Code · 4,821 symbols indexed</span>
    </div>
  </div>

</div>
