<script lang="ts">
  import { Toggle } from '@rokkit/ui';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  const viewOptions = [
    { value: 'split', icon: 'i-solar-sidebar-minimalistic-bold-duotone', label: 'Split', description: 'Split view' },
    { value: 'board', icon: 'i-solar-widget-3-bold-duotone', label: 'Board', description: 'Board view' },
  ];

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

  // Add project panel
  let showAdd = $state(false);
  let addMode = $state<'choose' | 'scan' | 'new'>('choose');

  // Manage groups panel
  let showGroups = $state(false);

  // Variant overrides from localStorage
  function loadVariantOverrides(): Record<string, string | null> {
    try {
      const raw = localStorage.getItem('sensei:variant_overrides');
      return raw ? JSON.parse(raw) : {};
    } catch { return {}; }
  }
  function saveVariantOverrides(overrides: Record<string, string | null>) {
    try { localStorage.setItem('sensei:variant_overrides', JSON.stringify(overrides)); } catch {}
  }
  let variantOverrides = $state<Record<string, string | null>>(loadVariantOverrides());

  function removeFromGroup(projectId: string) {
    const o = { ...variantOverrides, [projectId]: null };
    variantOverrides = o;
    saveVariantOverrides(o);
  }
  function ungroupAll(groupKey: string) {
    const members = data.projects.filter((p: { variant_group: string | null }) => resolvedGroup(p) === groupKey);
    const o = { ...variantOverrides };
    for (const m of members) o[m.id] = null;
    variantOverrides = o;
    saveVariantOverrides(o);
  }
  function resolvedGroup(p: { id: string; variant_group: string | null }): string | null {
    if (p.id in variantOverrides) return variantOverrides[p.id];
    return p.variant_group;
  }

  // ── Scan mode ──
  type ScannedRepo = {
    name: string; path: string; remote: string | null; description: string | null;
    categories: string[];
    status: 'active' | 'recent' | 'stale' | 'archived' | 'abandoned' | 'unknown';
    last_commit_days: number | null; tech_stack: string[]; commit_count: number;
    duplicate_of: string | null; variant_group: string | null;
  };
  const SCAN_STATUS_CLS: Record<string, string> = {
    active: 'bg-success-z2 text-success-z7', recent: 'bg-primary-z2 text-primary-z7',
    stale: 'bg-warning-z2 text-warning-z7',  archived: 'bg-surface-z3 text-surface-z5',
    abandoned: 'bg-error-z2 text-error-z7',  unknown: 'bg-surface-z3 text-surface-z5',
  };
  const SCAN_CAT_META: Record<string, { label: string; icon: string }> = {
    app:     { label: 'Apps',               icon: 'i-solar-monitor-smartphone-bold-duotone' },
    library: { label: 'Libraries',          icon: 'i-solar-box-bold-duotone' },
    tool:    { label: 'Tools & CLIs',       icon: 'i-solar-settings-bold-duotone' },
    idea:    { label: 'Ideas & Prototypes', icon: 'i-solar-lightbulb-bolt-bold-duotone' },
  };
  let scanRoot = $state('');
  let scanning = $state(false);
  let scanned = $state<ScannedRepo[]>([]);
  let scanSelected = $state<Set<string>>(new Set());

  function scanVariantClusters(repos: ScannedRepo[]): Map<string, ScannedRepo[]> {
    const map = new Map<string, ScannedRepo[]>();
    for (const r of repos) {
      if (r.variant_group) {
        const g = map.get(r.variant_group) ?? [];
        g.push(r);
        map.set(r.variant_group, g);
      }
    }
    for (const [k, v] of map) { if (v.length < 2) map.delete(k); }
    return map;
  }

  const scanGroups = $derived.by(() => {
    const cats: Record<string, ScannedRepo[]> = { app: [], library: [], tool: [], idea: [] };
    const attn: ScannedRepo[] = [];
    for (const r of scanned) {
      if (r.duplicate_of || r.status === 'abandoned') { attn.push(r); }
      else {
        const buckets = r.categories?.length ? r.categories.filter(c => c in SCAN_CAT_META) : [];
        if (buckets.length === 0) { /* skip ungrouped */ }
        else { for (const cat of buckets) { if (cats[cat]) cats[cat].push(r); } }
      }
    }
    const out = Object.entries(SCAN_CAT_META)
      .filter(([k]) => (cats[k]?.length ?? 0) > 0)
      .map(([k, m]) => ({ key: k, ...m, repos: cats[k], variants: scanVariantClusters(cats[k]) }));
    if (attn.length > 0) out.push({ key: 'attention', label: 'Needs attention', icon: 'i-solar-danger-triangle-bold-duotone', repos: attn, variants: new Map<string, ScannedRepo[]>() });
    return out;
  });

  function isScanGroupFull(repos: ScannedRepo[]) { return repos.every(r => scanSelected.has(r.path)); }
  function toggleScanGroup(repos: ScannedRepo[]) {
    const s = new Set(scanSelected);
    if (isScanGroupFull(repos)) { repos.forEach(r => s.delete(r.path)); }
    else { repos.forEach(r => s.add(r.path)); }
    scanSelected = s;
  }
  function toggleScanRepo(path: string) {
    const s = new Set(scanSelected);
    s.has(path) ? s.delete(path) : s.add(path);
    scanSelected = s;
  }

  async function doScan() {
    if (!scanRoot) return;
    scanning = true; scanned = []; scanSelected = new Set();
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const found = await invoke<ScannedRepo[]>('analyze_folder', { root: scanRoot });
      scanned = found;
      scanSelected = new Set(found.filter(r => r.status === 'active' || r.status === 'recent').map(r => r.path));
    } catch {
      // Tauri not available — scan returns nothing
    } finally { scanning = false; }
  }

  // ── New project mode ──
  const domains = [
    { id: 'personal',    label: 'Personal',    icon: 'i-solar-user-bold-duotone',    desc: 'Side project or open source' },
    { id: 'work',        label: 'Work',        icon: 'i-solar-buildings-bold-duotone', desc: 'Client or employer project' },
  ] as const;
  const projectTypes: Record<string, { label: string; desc: string; icon: string }[]> = {
    personal: [
      { label: 'Greenfield',   desc: 'New project from scratch',        icon: 'i-solar-leaf-bold-duotone' },
      { label: 'Enhancement',  desc: 'Add features to existing code',   icon: 'i-solar-layers-bold-duotone' },
      { label: 'Research',     desc: 'Exploration or learning spike',   icon: 'i-solar-telescope-bold-duotone' },
    ],
    work: [
      { label: 'Implementation', desc: 'Build or extend a system',      icon: 'i-solar-sledgehammer-bold-duotone' },
      { label: 'Analysis',       desc: 'Understand an existing system', icon: 'i-solar-magnifer-bold-duotone' },
      { label: 'Proposal',       desc: 'Requirements or estimation',    icon: 'i-solar-document-text-bold-duotone' },
    ],
  };
  let newDomain = $state<'personal' | 'work'>('personal');
  let newType = $state('');
  let newName = $state('');
  let newRoot = $state('');
  $effect(() => { void newDomain; newType = ''; }); // reset type when domain changes

  // ── Shared indexing ──
  let addPhase = $state<'idle' | 'done'>('idle');

  function startIndexing() {
    addPhase = 'done';
  }

  function closeAdd() {
    showAdd = false; addMode = 'choose';
    scanRoot = ''; scanned = []; scanSelected = new Set();
    newName = ''; newRoot = ''; newType = '';
    addPhase = 'idle';
  }

  // Keep old name for HTML button references
  function openAdd() { showAdd = true; addMode = 'choose'; }

  const maturityLabel = ['Seed', 'Exploring', 'Developing', 'Maturing', 'Established', 'Mature'];
  const maturityBg    = ['bg-surface-z3', 'bg-info-z5', 'bg-warning-z5', 'bg-secondary-z5', 'bg-success-z5', 'bg-primary-z6'];

  const CATEGORY_ICON: Record<string, string> = {
    app:     'i-solar-monitor-smartphone-bold-duotone',
    library: 'i-solar-box-bold-duotone',
    tool:    'i-solar-settings-bold-duotone',
    idea:    'i-solar-lightbulb-bolt-bold-duotone',
  };

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

  // Group filteredProjects by variant_group (with overrides applied)
  type GroupSection = { type: 'group'; key: string; name: string; items: typeof data.projects };
  type SoloSection  = { type: 'solo';  item: (typeof data.projects)[0] };
  type Section = GroupSection | SoloSection;

  const groupedProjects = $derived.by<Section[]>(() => {
    const groupMap = new Map<string, typeof data.projects>();
    const solos: (typeof data.projects)[0][] = [];

    for (const p of filteredProjects) {
      const g = resolvedGroup(p);
      if (g) {
        const arr = groupMap.get(g) ?? [];
        arr.push(p);
        groupMap.set(g, arr);
      } else {
        solos.push(p);
      }
    }

    // Only treat as a group if 2+ members; otherwise fall through to solos
    const sections: Section[] = [];
    const pendingSolos = [...solos];

    for (const [key, items] of groupMap) {
      if (items.length >= 2) {
        sections.push({ type: 'group', key, name: key, items });
      } else {
        pendingSolos.push(...items);
      }
    }

    // Merge and re-sort solos by original order
    const originalOrder = filteredProjects.map((p: { id: string }) => p.id);
    pendingSolos.sort((a, b) => originalOrder.indexOf(a.id) - originalOrder.indexOf(b.id));
    for (const item of pendingSolos) sections.push({ type: 'solo', item });

    // Sort sections: groups first (preserving their first-item order), then solos
    sections.sort((a, b) => {
      const aIdx = a.type === 'group'
        ? originalOrder.indexOf(a.items[0].id)
        : originalOrder.indexOf(a.item.id);
      const bIdx = b.type === 'group'
        ? originalOrder.indexOf(b.items[0].id)
        : originalOrder.indexOf(b.item.id);
      return aIdx - bIdx;
    });

    return sections;
  });

  // All distinct groups across all projects (for Manage groups panel)
  const allGroups = $derived.by(() => {
    const map = new Map<string, typeof data.projects>();
    for (const p of data.projects) {
      const g = resolvedGroup(p);
      if (g) {
        const arr = map.get(g) ?? [];
        arr.push(p);
        map.set(g, arr);
      }
    }
    return [...map.entries()].filter(([, items]) => items.length >= 2).map(([key, items]) => ({ key, items }));
  });

  let project = $derived(data.projects.find((p: { id: string }) => p.id === selectedId));

  let filteredCards = $derived(
    project?.cards.filter((c: { phase?: string }) =>
      cardPhaseFilter === 'all' || c.phase === cardPhaseFilter
    ) ?? []
  );

  let selectedCard = $derived(project?.cards.find((c: { id: string }) => c.id === selectedCardId));

  // Phases for card filter
  let phaseNames = $derived(project?.phases.map((p: { name: string }) => p.name) ?? []);

  // Whether the selected project has indexed data yet
  let hasData = $derived(
    (project?.cards?.length ?? 0) > 0 || (project?.sessions?.length ?? 0) > 0
  );
  let hasPhaseProgress = $derived(
    project?.phases?.some((p: { done: boolean; active: boolean }) => p.done || p.active) ?? false
  );

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
      <Toggle bind:value={viewMode} options={viewOptions} showLabels={false} size="sm" />
      <button
        onclick={() => showGroups = !showGroups}
        title="Manage groups"
        class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 py-1.5 text-xs transition-colors
               {showGroups ? 'border-primary-z4 bg-primary-z1 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
      >
        <span class="i-solar-layers-minimalistic-bold-duotone text-sm"></span>
        Manage groups
      </button>
      <button
        onclick={openAdd}
        class="flex items-center gap-1.5 rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 py-1.5 text-xs text-surface-z7 transition-colors hover:bg-surface-z3"
      >
        <span class="i-solar-add-circle-bold-duotone text-sm"></span>
        Add project
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
          {#each groupedProjects as section}
            {#if section.type === 'group'}
              <!-- Group header chip -->
              <div class="mt-2 mb-0.5 flex items-center gap-1.5 px-2">
                <span class="i-solar-layers-minimalistic-bold-duotone text-xs text-info-z6 shrink-0"></span>
                <span class="flex-1 truncate text-[10px] font-semibold uppercase tracking-wide text-info-z6">{section.name}</span>
                <span class="text-[10px] text-surface-z4">{section.items.length}</span>
              </div>
              <!-- Group items — slightly indented -->
              {#each section.items as p (p.id)}
                <button
                  onclick={() => selectProject(p.id)}
                  class="w-full rounded-xl pl-5 pr-3 py-2.5 text-left transition-colors
                         {selectedId === p.id ? 'bg-primary-z2' : 'hover:bg-surface-z2/80'}"
                >
                  <div class="flex items-center gap-1.5">
                    <span class="text-sm shrink-0 {p.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
                    <span class="truncate text-sm font-medium text-surface-z8">{p.name}</span>
                    {#if p.scanStatus}
                      <span class="ml-auto shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-medium capitalize {SCAN_STATUS_CLS[p.scanStatus] ?? SCAN_STATUS_CLS.unknown}">{p.scanStatus}</span>
                    {:else if p.language}
                      <span class="ml-auto shrink-0 text-[10px] text-surface-z4">{p.language}</span>
                    {/if}
                  </div>
                  <p class="mt-0.5 truncate text-[11px] text-surface-z4">{p.description}</p>
                  <div class="mt-1.5 flex items-center gap-1.5 flex-wrap">
                    {#if p.tech_stack?.length > 1}
                      {#each p.tech_stack.slice(0, 3) as t}
                        <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">{t}</span>
                      {/each}
                    {:else if p.maturity > 0}
                      <div class="flex flex-1 gap-0.5">
                        {#each Array(5) as _, i}
                          <div class="h-1 flex-1 rounded-full {i < p.maturity ? maturityBg[p.maturity] : 'bg-surface-z3'}"></div>
                        {/each}
                      </div>
                    {/if}
                    <span class="ml-auto text-[10px] text-surface-z4">{p.lastActivity}</span>
                  </div>
                </button>
              {/each}
            {:else}
              <!-- Solo (ungrouped) project -->
              <button
                onclick={() => selectProject(section.item.id)}
                class="w-full rounded-xl px-3 py-2.5 text-left transition-colors
                       {selectedId === section.item.id ? 'bg-primary-z2' : 'hover:bg-surface-z2/80'}"
              >
                <div class="flex items-center gap-1.5">
                  <span class="text-sm shrink-0 {section.item.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
                  <span class="truncate text-sm font-medium text-surface-z8">{section.item.name}</span>
                  {#if section.item.scanStatus}
                    <span class="ml-auto shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-medium capitalize {SCAN_STATUS_CLS[section.item.scanStatus] ?? SCAN_STATUS_CLS.unknown}">{section.item.scanStatus}</span>
                  {:else if section.item.language}
                    <span class="ml-auto shrink-0 text-[10px] text-surface-z4">{section.item.language}</span>
                  {/if}
                </div>
                <p class="mt-0.5 truncate text-[11px] text-surface-z4">{section.item.description}</p>
                <div class="mt-1.5 flex items-center gap-1.5 flex-wrap">
                  {#if section.item.tech_stack?.length > 1}
                    {#each section.item.tech_stack.slice(0, 3) as t}
                      <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">{t}</span>
                    {/each}
                  {:else if section.item.maturity > 0}
                    <div class="flex flex-1 gap-0.5">
                      {#each Array(5) as _, i}
                        <div class="h-1 flex-1 rounded-full {i < section.item.maturity ? maturityBg[section.item.maturity] : 'bg-surface-z3'}"></div>
                      {/each}
                    </div>
                  {/if}
                  <span class="ml-auto text-[10px] text-surface-z4">{section.item.lastActivity}</span>
                </div>
              </button>
            {/if}
          {/each}
        </div>
      </div>

      <!-- Manage groups panel (slides in alongside list) -->
      {#if showGroups}
        <div class="flex w-64 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden bg-surface-z1">
          <div class="flex items-center justify-between px-3 py-2.5 border-b border-surface-z0/50 shrink-0">
            <span class="text-xs font-semibold text-surface-z7">Variant groups</span>
            <button onclick={() => showGroups = false} title="Close" class="text-surface-z4 hover:text-surface-z7 transition-colors">
              <span class="i-solar-close-circle-bold-duotone text-base"></span>
            </button>
          </div>
          <div class="flex-1 overflow-y-auto px-2 py-2 space-y-3">
            {#if allGroups.length === 0}
              <div class="flex flex-col items-center justify-center py-8 text-center gap-2">
                <span class="i-solar-layers-minimalistic-bold-duotone text-2xl text-surface-z3"></span>
                <p class="text-xs text-surface-z4">No variant groups detected</p>
              </div>
            {:else}
              {#each allGroups as group}
                <div class="rounded-xl border border-surface-z3 bg-surface-z2/50 overflow-hidden">
                  <div class="flex items-center gap-1.5 px-3 py-2 border-b border-surface-z2">
                    <span class="i-solar-layers-minimalistic-bold-duotone text-xs text-info-z6 shrink-0"></span>
                    <span class="flex-1 truncate text-xs font-semibold text-surface-z7">{group.key}</span>
                    <button
                      onclick={() => ungroupAll(group.key)}
                      class="text-[10px] text-surface-z4 hover:text-error-z6 transition-colors"
                      title="Ungroup all"
                    >Ungroup</button>
                  </div>
                  <div class="divide-y divide-surface-z0/20">
                    {#each group.items as p}
                      <div class="flex items-center gap-2 px-3 py-2">
                        <span class="text-xs shrink-0 {p.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z5' : 'i-solar-code-square-bold-duotone text-primary-z5'}"></span>
                        <span class="flex-1 truncate text-xs text-surface-z7">{p.name}</span>
                        <button
                          onclick={() => removeFromGroup(p.id)}
                          class="text-[10px] text-surface-z4 hover:text-error-z6 transition-colors shrink-0"
                          title="Remove from group"
                        >
                          <span class="i-solar-close-square-bold-duotone text-xs"></span>
                        </button>
                      </div>
                    {/each}
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}

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

            <!-- Stats strip: scan metadata for new projects, session stats for tracked ones -->
            <div class="mt-3 flex items-center gap-3 flex-wrap text-xs text-surface-z5">
              {#if project.scanStatus}
                <span class="rounded-full px-2 py-0.5 text-[10px] font-medium capitalize {SCAN_STATUS_CLS[project.scanStatus] ?? SCAN_STATUS_CLS.unknown}">{project.scanStatus}</span>
                {#if project.category && project.category !== 'unknown'}
                  <span class="flex items-center gap-1">
                    <span class="text-sm {CATEGORY_ICON[project.category] ?? ''} text-surface-z4"></span>
                    <span class="capitalize">{project.category}</span>
                  </span>
                {/if}
                {#if project.commit_count > 0}
                  <span><span class="font-semibold text-surface-z8">{project.commit_count}</span> commits</span>
                {/if}
                {#if project.tech_stack?.length}
                  <div class="flex gap-1">
                    {#each project.tech_stack.slice(0, 4) as t}
                      <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">{t}</span>
                    {/each}
                  </div>
                {/if}
              {:else}
                {#if project.sessionCount > 0}
                  <span><span class="font-semibold text-surface-z8">{project.sessionCount}</span> sessions</span>
                {/if}
                {#if project.cardCount > 0}
                  <span><span class="font-semibold text-surface-z8">{project.cardCount}</span> cards</span>
                {/if}
                {#if project.symbolCount > 0}
                  <span><span class="font-semibold text-surface-z8">{project.symbolCount?.toLocaleString()}</span> symbols</span>
                {/if}
                {#if project.ftrScore > 0}
                  <span class="rounded-full bg-success-z2 px-2 py-0.5 text-[10px] font-semibold text-success-z7">
                    FTR {Math.round(project.ftrScore * 100)}%
                  </span>
                {/if}
              {/if}
            </div>

            <!-- Phase pipeline: only show if any work has been tracked -->
            {#if hasPhaseProgress}
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
            {/if}

            <!-- Tab bar: only shown when project has indexed data -->
            {#if hasData}
              <div class="mt-3 flex gap-1">
                {#each (['cards', 'graph', 'sessions'] as const) as tab}
                  <button
                    onclick={() => activeTab = tab}
                    class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors capitalize
                           {activeTab === tab ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
                  >{tab}</button>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Tab content -->
          {#if !hasData}
            <!-- No data yet — show project overview with scan metadata -->
            <div class="flex-1 overflow-y-auto px-5 py-5 space-y-5">
              {#if project.description && project.description !== project.name}
                <p class="text-sm leading-relaxed text-surface-z6">{project.description}</p>
              {/if}
              {#if project.remote}
                <div class="flex items-center gap-2 rounded-lg bg-surface-z2 px-3 py-2 text-xs font-mono text-surface-z5">
                  <span class="i-solar-link-bold-duotone text-sm shrink-0"></span>
                  <span class="truncate">{project.remote}</span>
                </div>
              {/if}
              {#if resolvedGroup(project)}
                <div class="flex items-start gap-2 rounded-lg bg-info-z1 border border-info-z2/50 px-3 py-2.5 text-xs text-info-z6">
                  <span class="i-solar-layers-bold-duotone text-sm shrink-0 mt-0.5"></span>
                  <span>Part of the <strong>{resolvedGroup(project)}</strong> concept cluster — consider consolidating related variants.</span>
                </div>
              {/if}
              <div class="rounded-xl border border-surface-z3 bg-surface-z2/50 px-5 py-4 text-center space-y-3">
                <span class="i-solar-graph-up-bold-duotone text-2xl text-surface-z3 block mx-auto"></span>
                <p class="text-sm font-medium text-surface-z7">Not yet indexed</p>
                <p class="text-xs text-surface-z4">Start a session in Claude Code to build the knowledge graph, track decisions, and capture session history.</p>
                <button class="rounded-lg bg-primary-z6 px-4 py-2 text-xs font-semibold text-white hover:bg-primary-z7 transition-colors">
                  Open in Claude Code
                </button>
              </div>
            </div>
          {:else if activeTab === 'cards'}
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
              onclick={openAdd}
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

            {#if p.scanStatus}
              <!-- Scan-derived metadata -->
              <div class="flex items-center gap-1.5 mb-3 flex-wrap">
                <span class="rounded-full px-1.5 py-0.5 text-[9px] font-medium capitalize {SCAN_STATUS_CLS[p.scanStatus] ?? SCAN_STATUS_CLS.unknown}">{p.scanStatus}</span>
                {#each (p.tech_stack ?? []).slice(0, 3) as t}
                  <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">{t}</span>
                {/each}
              </div>
              <div class="flex items-center justify-between text-[10px] text-surface-z4">
                <span>{p.commit_count ?? 0} commits · {p.lastActivity}</span>
                {#if p.category && p.category !== 'unknown'}
                  <span class="capitalize text-surface-z4">{p.category}</span>
                {/if}
              </div>
            {:else}
              <!-- Maturity bar for tracked projects -->
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
                {#if p.ftrScore > 0}
                  <span class="rounded-full bg-success-z2 px-1.5 py-0.5 text-[10px] font-semibold text-success-z7">FTR {Math.round(p.ftrScore * 100)}%</span>
                {/if}
              </div>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}

</div>

<!-- ══ ADD PROJECT PANEL ══════════════════════════════════════════════ -->
{#if showAdd}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-40 bg-surface-z9/20 backdrop-blur-sm"
       onclick={closeAdd} onkeydown={(e) => e.key === 'Escape' && closeAdd()}></div>

  <div class="fixed right-0 top-0 bottom-0 z-50 flex w-[440px] flex-col border-l border-surface-z3 bg-surface-z1 shadow-2xl">

    <!-- Header -->
    <div class="flex items-center justify-between border-b border-surface-z0/50 px-5 py-4 shrink-0">
      <div class="flex items-center gap-2">
        {#if addMode !== 'choose'}
          <button onclick={() => { addMode = 'choose'; scanned = []; }} aria-label="Back" class="text-surface-z4 hover:text-surface-z7 transition-colors mr-1">
            <span class="i-solar-arrow-left-bold-duotone text-base"></span>
          </button>
        {/if}
        <h2 class="text-sm font-semibold text-surface-z8">
          {addMode === 'choose' ? 'Add project' : addMode === 'scan' ? 'Import from folder' : 'New project'}
        </h2>
      </div>
      <button onclick={closeAdd} aria-label="Close panel" class="text-surface-z4 hover:text-surface-z7 transition-colors">
        <span class="i-solar-close-circle-bold-duotone text-lg"></span>
      </button>
    </div>

    <div class="flex-1 overflow-y-auto px-5 py-5">

      <!-- ── Choose mode ── -->
      {#if addMode === 'choose'}
        <p class="text-sm text-surface-z5 mb-5">How would you like to add a project?</p>
        <div class="space-y-3">
          <button onclick={() => addMode = 'scan'}
            class="flex w-full items-start gap-4 rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-4 text-left hover:border-primary-z4 hover:bg-primary-z1 transition-all">
            <span class="i-solar-folder-with-files-bold-duotone text-xl text-primary-z5 mt-0.5 shrink-0"></span>
            <div>
              <p class="text-sm font-semibold text-surface-z8">Import existing repos</p>
              <p class="text-xs text-surface-z5 mt-1">Scan a folder, classify all git repos, detect duplicates, and select what to import.</p>
            </div>
            <span class="i-solar-arrow-right-bold-duotone text-sm text-surface-z4 mt-1 shrink-0"></span>
          </button>

          <button onclick={() => addMode = 'new'}
            class="flex w-full items-start gap-4 rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-4 text-left hover:border-primary-z4 hover:bg-primary-z1 transition-all">
            <span class="i-solar-add-square-bold-duotone text-xl text-primary-z5 mt-0.5 shrink-0"></span>
            <div>
              <p class="text-sm font-semibold text-surface-z8">Start a new project</p>
              <p class="text-xs text-surface-z5 mt-1">Choose domain and type — Sensei creates the project context and kicks off indexing.</p>
            </div>
            <span class="i-solar-arrow-right-bold-duotone text-sm text-surface-z4 mt-1 shrink-0"></span>
          </button>
        </div>

      <!-- ── Scan mode ── -->
      {:else if addMode === 'scan'}

        {#if addPhase === 'idle'}
          <!-- Folder input -->
          <div class="space-y-4">
            <div class="flex gap-2">
              <input
                value={scanRoot}
                oninput={(e) => { scanRoot = (e.target as HTMLInputElement).value; scanned = []; }}
                placeholder="~/Developer"
                class="min-w-0 flex-1 rounded-xl border border-surface-z3 bg-surface-z2 px-3 py-2.5 font-mono text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4"
              />
              <button onclick={() => { scanRoot = '~/Developer'; doScan(); }} aria-label="Browse folder"
                class="rounded-xl border border-surface-z3 bg-surface-z2 px-3 text-surface-z5 hover:bg-surface-z3 transition-colors">
                <span class="i-solar-folder-open-bold-duotone text-base"></span>
              </button>
              <button onclick={doScan} disabled={!scanRoot || scanning} aria-label="Scan"
                class="rounded-xl border border-surface-z3 bg-surface-z2 px-3 text-surface-z5 hover:bg-surface-z3 transition-colors disabled:opacity-40">
                <span class="{scanning ? 'i-solar-refresh-bold-duotone animate-spin' : 'i-solar-magnifer-bold-duotone'} text-base"></span>
              </button>
            </div>

            {#if scanning}
              <div class="flex items-center gap-2 text-sm text-surface-z5 py-4 justify-center">
                <span class="i-solar-refresh-bold-duotone animate-spin text-primary-z6"></span>
                Scanning…
              </div>

            {:else if scanned.length > 0}
              <!-- Summary -->
              <div class="flex items-center justify-between text-xs text-surface-z5">
                <span>{scanned.length} repos · {scanSelected.size} selected</span>
                <div class="flex gap-3">
                  <button onclick={() => scanSelected = new Set(scanned.filter(r => !r.duplicate_of).map(r => r.path))} class="text-primary-z6 hover:text-primary-z7">Select healthy</button>
                  <button onclick={() => scanSelected = new Set()} class="text-surface-z4 hover:text-surface-z6">Clear</button>
                </div>
              </div>

              <!-- Groups -->
              <div class="space-y-3 max-h-96 overflow-y-auto pr-1">
                {#each scanGroups as group}
                  {@const allSel = isScanGroupFull(group.repos)}
                  {@const standalones = group.repos.filter(r => !r.variant_group)}
                  {@const clusterEntries = [...group.variants.entries()]}
                  <div>
                    <button onclick={() => toggleScanGroup(group.repos)}
                      class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left hover:bg-surface-z2 transition-colors">
                      <span class="{group.icon} text-sm {group.key === 'attention' ? 'text-warning-z6' : 'text-surface-z5'}"></span>
                      <span class="flex-1 text-xs font-semibold text-surface-z6 uppercase tracking-wide">{group.label}</span>
                      <span class="text-[10px] text-surface-z4">{group.repos.length}</span>
                      <span class="text-[10px] text-primary-z6">{allSel ? 'Deselect' : 'Select'} all</span>
                    </button>
                    <div class="ml-2 rounded-xl border border-surface-z2 divide-y divide-surface-z0/20 overflow-hidden">
                      <!-- Standalone repos -->
                      {#each standalones as repo}
                        {@const sel = scanSelected.has(repo.path)}
                        <button onclick={() => toggleScanRepo(repo.path)}
                          class="flex w-full items-start gap-3 px-3 py-2.5 text-left transition-colors hover:bg-surface-z2/60 {sel ? 'bg-primary-z1/40' : 'bg-surface-z1'}">
                          <div class="mt-0.5 shrink-0 h-4 w-4 rounded border-2 flex items-center justify-center
                                      {sel ? 'border-primary-z6 bg-primary-z6' : 'border-surface-z4 bg-transparent'}">
                            {#if sel}<span class="text-[9px] font-bold text-white leading-none">✓</span>{/if}
                          </div>
                          <div class="min-w-0 flex-1">
                            <div class="flex items-center gap-1.5 flex-wrap">
                              <span class="text-sm font-medium text-surface-z8">{repo.name}</span>
                              <span class="text-[10px] rounded-full px-1.5 py-0.5 {SCAN_STATUS_CLS[repo.status] ?? SCAN_STATUS_CLS.unknown}">
                                {repo.status}{repo.last_commit_days ? ` · ${repo.last_commit_days}d` : ''}
                              </span>
                              {#each repo.tech_stack.slice(0,2) as t}
                                <span class="text-[10px] bg-surface-z3 text-surface-z5 px-1.5 py-0.5 rounded">{t}</span>
                              {/each}
                            </div>
                            {#if repo.duplicate_of}
                              <p class="text-[11px] text-warning-z6 mt-0.5">Likely copy of {repo.duplicate_of.split('/').pop()}</p>
                            {:else if repo.description}
                              <p class="text-xs text-surface-z4 mt-0.5 line-clamp-1">{repo.description}</p>
                            {/if}
                          </div>
                        </button>
                      {/each}
                      <!-- Variant clusters -->
                      {#each clusterEntries as [stem, clusterRepos]}
                        {@const allClusterSel = clusterRepos.every(r => scanSelected.has(r.path))}
                        <div class="border-l-2 border-info-z4 bg-info-z1/20">
                          <div class="flex items-center gap-2 px-3 py-1.5 border-b border-info-z2/40">
                            <span class="i-solar-layers-minimalistic-bold-duotone text-xs text-info-z6 shrink-0"></span>
                            <span class="flex-1 text-[11px] font-semibold text-info-z7">{clusterRepos.length} variants — "{stem}"</span>
                            <button onclick={() => {
                              const s = new Set(scanSelected);
                              if (allClusterSel) { clusterRepos.forEach(r => s.delete(r.path)); }
                              else { clusterRepos.forEach(r => s.add(r.path)); }
                              scanSelected = s;
                            }} class="text-[10px] text-primary-z6 hover:text-primary-z7">{allClusterSel ? 'Deselect' : 'Select'} all</button>
                          </div>
                          {#each clusterRepos as repo}
                            {@const sel = scanSelected.has(repo.path)}
                            {@const isNewest = clusterRepos.every(r => r.path === repo.path || (repo.last_commit_days ?? 9999) <= (r.last_commit_days ?? 9999))}
                            <button onclick={() => toggleScanRepo(repo.path)}
                              class="flex w-full items-start gap-3 px-3 py-2.5 text-left transition-colors hover:bg-info-z1/40 {sel ? 'bg-primary-z1/30' : ''}">
                              <div class="mt-0.5 shrink-0 h-4 w-4 rounded border-2 flex items-center justify-center
                                          {sel ? 'border-primary-z6 bg-primary-z6' : 'border-surface-z4 bg-transparent'}">
                                {#if sel}<span class="text-[9px] font-bold text-white leading-none">✓</span>{/if}
                              </div>
                              <div class="min-w-0 flex-1">
                                <div class="flex items-center gap-1.5 flex-wrap">
                                  <span class="text-sm font-medium text-surface-z8">{repo.name}</span>
                                  {#if isNewest}<span class="text-[10px] rounded-full bg-success-z2 text-success-z7 px-1.5 py-0.5">most recent</span>{/if}
                                  <span class="text-[10px] rounded-full px-1.5 py-0.5 {SCAN_STATUS_CLS[repo.status] ?? SCAN_STATUS_CLS.unknown}">
                                    {repo.status}{repo.last_commit_days ? ` · ${repo.last_commit_days}d` : ''}
                                  </span>
                                </div>
                                {#if repo.description}<p class="text-xs text-surface-z4 mt-0.5 line-clamp-1">{repo.description}</p>{/if}
                              </div>
                            </button>
                          {/each}
                          <div class="flex items-start gap-2 px-3 py-2 bg-info-z1/30 border-t border-info-z2/30 text-[11px] text-info-z6">
                            <span class="i-solar-lightbulb-bolt-bold-duotone text-xs shrink-0 mt-0.5"></span>
                            <span>Different approaches to the same concept. After importing, pick the strongest direction and archive the others.</span>
                          </div>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/each}
              </div>

              <button
                onclick={startIndexing}
                disabled={scanSelected.size === 0}
                class="w-full rounded-xl py-2.5 text-sm font-semibold transition-colors
                       {scanSelected.size > 0 ? 'bg-primary-z6 text-white hover:bg-primary-z7' : 'bg-surface-z3 text-surface-z4 cursor-not-allowed'}"
              >Import {scanSelected.size} project{scanSelected.size !== 1 ? 's' : ''} →</button>
            {/if}
          </div>

        {:else}
          <div class="space-y-5">
            <div class="text-center py-4">
              <div class="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-success-z2 text-xl mb-3">✓</div>
              <p class="text-sm font-semibold text-surface-z8">{scanSelected.size} project{scanSelected.size !== 1 ? 's' : ''} imported</p>
              <p class="text-xs text-surface-z5 mt-1">Ready to explore</p>
            </div>
            <button onclick={closeAdd} class="w-full rounded-xl bg-primary-z6 py-2.5 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors">
              View projects
            </button>
          </div>
        {/if}

      <!-- ── New project mode ── -->
      {:else if addMode === 'new'}
        {#if addPhase === 'idle'}
          <div class="space-y-5">
            <!-- Domain -->
            <div>
              <p class="text-xs font-medium text-surface-z6 mb-2">Domain</p>
              <div class="grid grid-cols-2 gap-2">
                {#each domains as d}
                  <button onclick={() => newDomain = d.id}
                    class="flex items-center gap-2.5 rounded-xl border px-3 py-2.5 text-left transition-all
                           {newDomain === d.id ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3 bg-surface-z2 hover:border-surface-z4'}">
                    <span class="{d.icon} text-base {newDomain === d.id ? 'text-primary-z6' : 'text-surface-z5'}"></span>
                    <div>
                      <p class="text-xs font-semibold {newDomain === d.id ? 'text-primary-z8' : 'text-surface-z7'}">{d.label}</p>
                      <p class="text-[10px] text-surface-z4">{d.desc}</p>
                    </div>
                  </button>
                {/each}
              </div>
            </div>

            <!-- Project type -->
            <div>
              <p class="text-xs font-medium text-surface-z6 mb-2">Type</p>
              <div class="space-y-1.5">
                {#each (projectTypes[newDomain] ?? []) as t}
                  <button onclick={() => newType = t.label}
                    class="flex w-full items-center gap-3 rounded-xl border px-3 py-2.5 text-left transition-all
                           {newType === t.label ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3 bg-surface-z2 hover:border-surface-z4'}">
                    <span class="{t.icon} text-sm {newType === t.label ? 'text-primary-z6' : 'text-surface-z5'}"></span>
                    <div class="min-w-0 flex-1">
                      <p class="text-sm font-medium {newType === t.label ? 'text-primary-z8' : 'text-surface-z7'}">{t.label}</p>
                      <p class="text-xs text-surface-z4">{t.desc}</p>
                    </div>
                    {#if newType === t.label}
                      <span class="i-solar-check-circle-bold-duotone text-primary-z6 text-base shrink-0"></span>
                    {/if}
                  </button>
                {/each}
              </div>
            </div>

            <!-- Name + root -->
            <div class="space-y-3">
              <div>
                <label class="mb-1 block text-xs text-surface-z5" for="new-proj-name">Project name</label>
                <input id="new-proj-name" bind:value={newName} placeholder="my-project"
                  class="w-full rounded-xl border border-surface-z3 bg-surface-z2 px-3 py-2.5 text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4" />
              </div>
              <div>
                <label class="mb-1 block text-xs text-surface-z5" for="new-proj-root">Root folder <span class="text-surface-z3">(optional)</span></label>
                <div class="flex gap-2">
                  <input id="new-proj-root" bind:value={newRoot} placeholder="~/Developer/my-project"
                    class="min-w-0 flex-1 rounded-xl border border-surface-z3 bg-surface-z2 px-3 py-2.5 font-mono text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4" />
                  <button onclick={() => newRoot = '~/Developer/' + (newName || 'new-project')} aria-label="Browse"
                    class="rounded-xl border border-surface-z3 bg-surface-z2 px-3 text-surface-z5 hover:bg-surface-z3 transition-colors">
                    <span class="i-solar-folder-open-bold-duotone text-base"></span>
                  </button>
                </div>
              </div>
            </div>

            <button
              onclick={startIndexing}
              disabled={!newName || !newType}
              class="w-full rounded-xl py-2.5 text-sm font-semibold transition-colors
                     {newName && newType ? 'bg-primary-z6 text-white hover:bg-primary-z7' : 'bg-surface-z3 text-surface-z4 cursor-not-allowed'}"
            >Create project →</button>
          </div>

        {:else}
          <div class="space-y-5">
            <div class="text-center py-4">
              <div class="inline-flex h-12 w-12 items-center justify-center rounded-xl bg-success-z2 text-xl mb-3">✓</div>
              <p class="text-sm font-semibold text-surface-z8">{newName} created</p>
              <p class="text-xs text-surface-z5 mt-1">{newDomain} · {newType}</p>
            </div>
            <button onclick={closeAdd} class="w-full rounded-xl bg-primary-z6 py-2.5 text-sm font-semibold text-white hover:bg-primary-z7 transition-colors">
              Open project
            </button>
          </div>
        {/if}
      {/if}

    </div>
  </div>
{/if}
