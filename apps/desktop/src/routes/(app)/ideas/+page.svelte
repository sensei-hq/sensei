<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  let selectedId = $state<string | null>(null);
  let prompt = $state('');

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

  let selectedIdea = $derived(data.ideas.find((i: { id: string }) => i.id === selectedId));
</script>

<div class="flex h-full min-h-0">

  <!-- Ideas grid -->
  <div class="flex flex-1 flex-col min-w-0 overflow-hidden">
    <div class="flex items-center justify-between border-b border-surface-z0/50 px-4 py-2 shrink-0">
      <h1 class="text-sm font-semibold text-surface-z8">Ideas</h1>
      <button class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 py-1.5 text-xs text-surface-z7 transition-colors hover:bg-surface-z3">
        <span class="i-solar-add-circle-bold-duotone text-sm"></span>
        New idea
      </button>
    </div>

    <div class="flex-1 overflow-y-auto px-4 py-4">
      <div class="grid grid-cols-2 gap-3 xl:grid-cols-3">
        {#each data.ideas as idea (idea.id)}
          <button
            onclick={() => selectedId = selectedId === idea.id ? null : idea.id}
            class="rounded-2xl border text-left transition-all
                   {selectedId === idea.id
                     ? 'border-primary-z4 bg-primary-z1'
                     : 'border-surface-z3/60 bg-surface-z2/50 hover:border-surface-z4 hover:bg-surface-z2'}"
          >
            <div class="px-4 py-4">
              <div class="flex items-start justify-between mb-2">
                <div class="flex items-center gap-2">
                  <span class="i-solar-lightbulb-bold-duotone text-xl text-warning-z6"></span>
                  <span class="font-semibold text-surface-z8">{idea.name}</span>
                </div>
                <span class="rounded-full bg-surface-z3 px-2 py-0.5 text-[9px] font-medium text-surface-z5 capitalize">{idea.status}</span>
              </div>

              <p class="text-xs text-surface-z5 line-clamp-2 mb-3">{idea.description}</p>

              <!-- Maturity -->
              <div class="flex items-center gap-2 mb-2">
                <div class="flex flex-1 gap-0.5">
                  {#each Array(5) as _, i}
                    <div class="h-1 flex-1 rounded-full {i < idea.maturity ? maturityBg[idea.maturity] : 'bg-surface-z3'}"></div>
                  {/each}
                </div>
                <span class="text-[10px] text-surface-z5">{maturityLabel[idea.maturity]}</span>
              </div>

              <!-- Tags -->
              <div class="flex flex-wrap gap-1">
                {#each idea.tags as tag}
                  <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">{tag}</span>
                {/each}
              </div>

              <div class="mt-2 flex items-center justify-between text-[10px] text-surface-z4">
                <span>{idea.cardCount} cards</span>
                <span>{idea.updatedAt}</span>
              </div>
            </div>

            <!-- Expanded card list -->
            {#if selectedId === idea.id}
              <div class="border-t border-primary-z3/50 px-4 py-3 space-y-2">
                {#each idea.cards as card (card.id)}
                  <div class="flex items-center gap-2">
                    <span class="text-sm shrink-0 {kindIcon[card.kind] ?? 'i-solar-document-bold-duotone'} text-surface-z5"></span>
                    <span class="flex-1 text-xs text-surface-z7 line-clamp-1">{card.title}</span>
                    <span class="shrink-0 text-[9px] rounded-full bg-surface-z3 px-1.5 py-0.5 text-surface-z5">{card.status}</span>
                  </div>
                {/each}
                <button class="mt-1 flex items-center gap-1.5 text-xs text-primary-z6 hover:text-primary-z7">
                  <span class="i-solar-arrow-right-up-bold-duotone text-xs"></span>
                  Graduate to repo
                </button>
              </div>
            {/if}
          </button>
        {/each}

        <!-- New idea placeholder -->
        <button class="rounded-2xl border-2 border-dashed border-surface-z3 p-8 text-center transition-colors hover:border-primary-z4 hover:bg-primary-z1/30">
          <span class="i-solar-add-circle-bold-duotone text-2xl text-surface-z4 block mx-auto mb-2"></span>
          <p class="text-sm text-surface-z5">Capture a new idea</p>
          <p class="mt-1 text-xs text-surface-z4">Start with a name and description</p>
        </button>
      </div>
    </div>

    <!-- Prompt bar -->
    <div class="border-t border-surface-z0/50 bg-surface-z2/60 px-4 py-2.5 backdrop-blur-sm shrink-0">
      <div class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-3 py-2 focus-within:border-primary-z4 transition-all">
        <span class="i-solar-magic-stick-3-bold-duotone text-sm text-primary-z6 shrink-0"></span>
        <input
          bind:value={prompt}
          placeholder={selectedIdea ? `Explore ${selectedIdea.name}…` : 'Describe an idea or ask "What should I build next?"'}
          class="flex-1 bg-transparent text-sm text-surface-z7 outline-none placeholder:text-surface-z4"
        />
        <kbd class="rounded border border-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z4">⏎</kbd>
      </div>
    </div>
  </div>

</div>
