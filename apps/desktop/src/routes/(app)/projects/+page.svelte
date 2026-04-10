<script lang="ts">
  import { Toggle } from '@rokkit/ui';
  import { invalidateAll } from '$app/navigation';
  import RepoList from '$lib/RepoList.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // Selection — individual project OR variant group
  let selectedId = $state<string | null>(null);
  let selectedGroupKey = $state<string | null>(null);
  let selectedCardId = $state<string | null>(null);
  let activeTab = $state<'cards' | 'graph' | 'sessions' | 'libraries'>('cards');
  let cardPhaseFilter = $state<string>('all');

  // Navigator state
  let search = $state('');
  let groupMode = $state<'variants' | 'clients'>('variants');
  let expandedGroupKeys = $state<Set<string>>(new Set());
  let showAllOlder = $state(false);

  // Per-repo edits (reactive overrides; persisted to localStorage on save)
  type RepoEdit = { client?: string | null; categories?: string[] };
  let repoEdits = $state<Map<string, RepoEdit>>(new Map());

  function getEffective(p: typeof data.projects[0]) {
    const e = repoEdits.get(p.path);
    return e ? { ...p, ...e } : p;
  }

  function saveRepoEdit(path: string, edit: RepoEdit) {
    const m = new Map(repoEdits);
    m.set(path, { ...(m.get(path) ?? {}), ...edit });
    repoEdits = m;
    try {
      const raw = localStorage.getItem('sensei:projects_raw');
      if (!raw) return;
      const all = JSON.parse(raw);
      const idx = all.findIndex((r: { path: string }) => r.path === path);
      if (idx !== -1) { all[idx] = { ...all[idx], ...edit }; localStorage.setItem('sensei:projects_raw', JSON.stringify(all)); }
    } catch {}
  }

  // Inline edit form state
  let editingRepoPath = $state<string | null>(null);
  let editDraftClient = $state('');
  let editDraftCats = $state<string[]>([]);

  function startEdit(p: typeof data.projects[0]) {
    const ep = getEffective(p);
    editingRepoPath = ep.path;
    editDraftClient = ep.client ?? '';
    editDraftCats = (ep.categories ?? []).filter((c: string) => c !== 'unknown');
  }

  function cancelEdit() { editingRepoPath = null; }

  function commitEdit(path: string) {
    saveRepoEdit(path, { client: editDraftClient.trim() || null, categories: editDraftCats });
    editingRepoPath = null;
  }

  // Prompt
  let prompt = $state('');

  // Add project panel
  let showAdd = $state(false);
  let addMode = $state<'choose' | 'scan' | 'new'>('choose');

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
    stale: 'bg-warning-z2 text-warning-z7', archived: 'bg-surface-z3 text-surface-z5',
    abandoned: 'bg-error-z2 text-error-z7', unknown: 'bg-surface-z3 text-surface-z5',
  };
  let scanRoot = $state('');
  let scanning = $state(false);
  let scanned = $state<ScannedRepo[]>([]);
  let scanSelected = $state<Set<string>>(new Set());

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
  let scanClientTags = $state(new Map<string, string>());

  const SENSEI_PORT = () => parseInt(localStorage.getItem('sensei:port') ?? '7744', 10);

  async function startIndexing() {
    const toImport = scanned.filter(r => scanSelected.has(r.path)).map(r => ({
      ...r,
      client: scanClientTags.get(r.path) ?? null,
    }));
    if (toImport.length > 0) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const existing = JSON.parse(localStorage.getItem('sensei:projects_raw') ?? '[]');
        const existingPaths = new Set(existing.map((r: { path: string }) => r.path));
        const newRepos = toImport.filter(r => !existingPaths.has(r.path));

        // Resolve real repoIds from .sensei/config.yaml, fall back to UUID.
        const withIds = await Promise.all(newRepos.map(async repo => {
          const repoId: string = await invoke<string | null>('get_repo_id', { path: repo.path })
            .catch(() => null) ?? crypto.randomUUID();
          return { ...repo, repoId };
        }));

        const merged = [...existing, ...withIds];
        localStorage.setItem('sensei:projects_raw', JSON.stringify(merged));

        const port = SENSEI_PORT();
        for (const repo of withIds) {
          // Register immediately so it shows up in projects.json even if indexing fails.
          fetch(`http://127.0.0.1:${port}/api/projects`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ repoId: repo.repoId, name: repo.name, path: repo.path }),
          }).catch(() => {});
          // Trigger indexing (fire-and-forget).
          fetch(`http://127.0.0.1:${port}/api/index`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ repoId: repo.repoId, repoPath: repo.path }),
          }).catch(() => {});
        }
      } catch {}
    }
    addPhase = 'done';
    invalidateAll();
  }

  function closeAdd() {
    showAdd = false; addMode = 'choose';
    scanRoot = ''; scanned = []; scanSelected = new Set(); scanClientTags = new Map();
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

  // ── Nav entries: groups and solos sorted by recency ──
  type NavEntry =
    | { type: 'group'; key: string; items: typeof data.projects; minDays: number | null; categories: string[] }
    | { type: 'solo';  item: (typeof data.projects)[0]; days: number | null };

  const navEntries = $derived.by<NavEntry[]>(() => {
    const groupMap = new Map<string, typeof data.projects>();
    const solos: (typeof data.projects)[0][] = [];
    for (const p of data.projects) {
      const ep = getEffective(p);
      const g = groupMode === 'clients'
        ? (ep.client?.trim() || null)
        : resolvedGroup(p);
      if (g) { const a = groupMap.get(g) ?? []; a.push(ep); groupMap.set(g, a); }
      else { solos.push(ep); }
    }
    const entries: NavEntry[] = [];
    const pendingSolos = [...solos];
    const minSize = groupMode === 'clients' ? 1 : 2;
    for (const [key, items] of groupMap) {
      if (items.length >= minSize) {
        const days = items.map(i => i.last_commit_days).filter((d): d is number => d != null);
        const cats = [...new Set(items.flatMap(i => (i.categories ?? []).filter((c: string) => c !== 'unknown')))] as string[];
        entries.push({ type: 'group', key, items, minDays: days.length ? Math.min(...days) : null, categories: cats });
      } else { pendingSolos.push(...items); }
    }
    for (const item of pendingSolos) {
      entries.push({ type: 'solo', item, days: item.last_commit_days ?? null });
    }
    entries.sort((a, b) => {
      const ad = a.type === 'group' ? a.minDays : a.days;
      const bd = b.type === 'group' ? b.minDays : b.days;
      if (ad == null && bd == null) return 0;
      if (ad == null) return 1;
      if (bd == null) return -1;
      return ad - bd;
    });
    return entries;
  });

  const RECENT_COUNT = 5;
  const recentEntries = $derived(navEntries.slice(0, RECENT_COUNT));
  const olderEntries  = $derived(navEntries.slice(RECENT_COUNT));

  // Search-filtered view — flat list, groups show only matching members
  type FilteredEntry =
    | { type: 'group'; key: string; items: typeof data.projects; matchedItems: typeof data.projects; minDays: number | null; categories: string[]; groupMatches: boolean }
    | { type: 'solo'; item: (typeof data.projects)[0]; days: number | null };

  const filteredEntries = $derived.by<FilteredEntry[]>(() => {
    const q = search.trim().toLowerCase();
    if (!q) return [];
    const result: FilteredEntry[] = [];
    for (const entry of navEntries) {
      if (entry.type === 'group') {
        const groupMatches = entry.key.toLowerCase().includes(q);
        const matchedItems = entry.items.filter((p: { name: string; description?: string }) =>
          p.name.toLowerCase().includes(q) || (p.description ?? '').toLowerCase().includes(q)
        );
        if (groupMatches || matchedItems.length > 0) {
          result.push({ ...entry, matchedItems: groupMatches ? entry.items : matchedItems, groupMatches });
        }
      } else {
        const { item } = entry;
        if (item.name.toLowerCase().includes(q) || (item.description ?? '').toLowerCase().includes(q)) {
          result.push(entry);
        }
      }
    }
    return result;
  });

  const isSearching = $derived(search.trim().length > 0);

  function toggleGroup(key: string) {
    const s = new Set(expandedGroupKeys);
    s.has(key) ? s.delete(key) : s.add(key);
    expandedGroupKeys = s;
  }

  function selectGroup(key: string) {
    selectedGroupKey = key;
    selectedId = null;
    selectedCardId = null;
    activeTab = 'cards';
  }

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
    selectedGroupKey = null;
    selectedCardId = null;
    cardPhaseFilter = 'all';
    activeTab = 'cards';
    showAllLibs = false;
  }

  // ── Library detection & index state ────────────────────────────────────────
  type DetectedDep = { name: string; file_count: number; total_files: number; usage_pct: number };
  type LibOptState = 'auto' | 'in' | 'out';
  type IndexStatus = 'idle' | 'queued' | 'running' | 'done' | 'failed';

  const LIB_AUTO_THRESHOLD = 30;   // ≥30% → auto opt-in
  const LIB_SUGGEST_MIN   = 10;   // 10–29% → suggested

  let libData    = $state(new Map<string, DetectedDep[]>());
  let libLoading = $state(new Set<string>());
  let showAllLibs = $state(false);

  let libOpts = $state<Record<string, Record<string, LibOptState>>>(
    (() => { try { return JSON.parse(localStorage.getItem('sensei:lib_opts') ?? '{}'); } catch { return {}; } })()
  );
  let indexStates = $state<Record<string, IndexStatus>>(
    (() => { try { return JSON.parse(localStorage.getItem('sensei:index_states') ?? '{}'); } catch { return {}; } })()
  );

  function getLibOpt(repoPath: string, lib: string): LibOptState {
    return libOpts[repoPath]?.[lib] ?? 'auto';
  }
  function setLibOpt(repoPath: string, lib: string, s: LibOptState) {
    libOpts = { ...libOpts, [repoPath]: { ...(libOpts[repoPath] ?? {}), [lib]: s } };
    localStorage.setItem('sensei:lib_opts', JSON.stringify(libOpts));
  }
  function isOptedIn(repoPath: string, dep: DetectedDep): boolean {
    const s = getLibOpt(repoPath, dep.name);
    if (s === 'in')  return true;
    if (s === 'out') return false;
    return dep.usage_pct >= LIB_AUTO_THRESHOLD;
  }
  async function triggerIndex(repoPath: string) {
    indexStates = { ...indexStates, [repoPath]: 'running' };
    localStorage.setItem('sensei:index_states', JSON.stringify(indexStates));
    try {
      const res = await fetch('http://localhost:7744/api/index', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ repoPath }),
      });
      if (!res.ok || !res.body) throw new Error(`HTTP ${res.status}`);
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = '';
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        for (const line of buf.split('\n')) {
          if (!line.trim()) continue;
          try {
            const msg = JSON.parse(line) as { status: string; message?: string };
            if (msg.status === 'done') {
              indexStates = { ...indexStates, [repoPath]: 'done' };
              localStorage.setItem('sensei:index_states', JSON.stringify(indexStates));
            } else if (msg.status === 'error') {
              indexStates = { ...indexStates, [repoPath]: 'failed' };
              localStorage.setItem('sensei:index_states', JSON.stringify(indexStates));
            }
          } catch { /* skip malformed line */ }
        }
        buf = buf.includes('\n') ? buf.split('\n').at(-1) ?? '' : buf;
      }
    } catch {
      indexStates = { ...indexStates, [repoPath]: 'failed' };
      localStorage.setItem('sensei:index_states', JSON.stringify(indexStates));
    }
  }
  async function loadDeps(path: string) {
    if (libData.has(path) || libLoading.has(path)) return;
    libLoading = new Set([...libLoading, path]);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const deps = await invoke<DetectedDep[]>('detect_dependencies', { path });
      libData = new Map([...libData, [path, deps]]);
    } catch { /* not in Tauri context */ }
    finally {
      const s = new Set(libLoading); s.delete(path); libLoading = s;
    }
  }
  $effect(() => { if (project?.path) loadDeps(project.path); });
</script>

<!-- ── Snippets ──────────────────────────────────────────────────────── -->

{#snippet dayLabel(d: number | null)}
  {#if d != null}{d === 0 ? 'Today' : d < 7 ? d + 'd' : d < 30 ? Math.floor(d / 7) + 'w' : Math.floor(d / 30) + 'mo'}{/if}
{/snippet}

{#snippet editForm(p: typeof data.projects[0])}
  <div class="border-t border-surface-z2 bg-surface-z1 px-3 py-3 space-y-2.5">
    <div>
      <p class="text-[10px] font-medium text-surface-z5 mb-1">Client</p>
      <input
        bind:value={editDraftClient}
        placeholder="e.g. Acme Corp"
        class="w-full rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 py-1.5 text-xs outline-none focus:border-primary-z4 transition-colors"
      />
    </div>
    <div>
      <p class="text-[10px] font-medium text-surface-z5 mb-1.5">Tags</p>
      <div class="flex flex-wrap gap-1">
        {#each ['app', 'library', 'tool', 'idea'] as cat}
          <button
            onclick={() => { const s = new Set(editDraftCats); s.has(cat) ? s.delete(cat) : s.add(cat); editDraftCats = [...s]; }}
            class="rounded-full border px-2 py-0.5 text-[10px] transition-colors
                   {editDraftCats.includes(cat) ? 'border-primary-z4 bg-primary-z1 text-primary-z7' : 'border-surface-z3 text-surface-z5 hover:border-surface-z4'}"
          >{cat}</button>
        {/each}
      </div>
    </div>
    <div class="flex gap-2">
      <button onclick={() => commitEdit(p.path)} class="flex-1 rounded-lg bg-primary-z6 py-1.5 text-xs font-semibold text-white hover:bg-primary-z7 transition-colors">Save</button>
      <button onclick={cancelEdit} class="flex-1 rounded-lg border border-surface-z3 py-1.5 text-xs text-surface-z6 hover:bg-surface-z3 transition-colors">Cancel</button>
    </div>
  </div>
{/snippet}

{#snippet depRow(repoPath: string, dep: DetectedDep)}
  {@const state = getLibOpt(repoPath, dep.name)}
  {@const optedIn = isOptedIn(repoPath, dep)}
  <div class="flex items-center gap-2.5 py-1">
    <span class="text-xs font-mono text-surface-z7 w-36 truncate shrink-0" title={dep.name}>{dep.name}</span>
    <div class="flex-1 min-w-0 h-1 rounded-full bg-surface-z3 overflow-hidden">
      <div class="h-full rounded-full {optedIn ? 'bg-success-z5' : 'bg-surface-z4'}" style="width: {Math.min(dep.usage_pct, 100)}%"></div>
    </div>
    <span class="text-[10px] text-surface-z4 w-7 text-right shrink-0">{dep.usage_pct < 1 ? '<1' : Math.round(dep.usage_pct)}%</span>
    <span class="text-[10px] text-surface-z4 shrink-0 w-10">{dep.file_count}f</span>
    <div class="shrink-0 w-16 flex justify-end">
      {#if optedIn && state === 'auto'}
        <button onclick={() => setLibOpt(repoPath, dep.name, 'out')} title="Opt out"
          class="flex items-center gap-0.5 text-[10px] text-success-z6 hover:text-danger-z5 transition-colors">
          <span class="i-solar-check-circle-bold-duotone text-xs"></span> auto
        </button>
      {:else if state === 'in'}
        <button onclick={() => setLibOpt(repoPath, dep.name, 'auto')} title="Undo opt-in"
          class="flex items-center gap-0.5 text-[10px] text-success-z7 hover:text-surface-z5 transition-colors">
          <span class="i-solar-check-circle-bold-duotone text-xs"></span> opted in
        </button>
      {:else if state === 'out'}
        <button onclick={() => setLibOpt(repoPath, dep.name, 'auto')}
          class="text-[10px] text-surface-z3 hover:text-surface-z6 transition-colors">undo skip</button>
      {:else}
        <button onclick={() => setLibOpt(repoPath, dep.name, 'in')}
          class="rounded border border-surface-z3 px-1.5 py-0.5 text-[10px] text-surface-z5 hover:border-primary-z4 hover:text-primary-z7 transition-colors">+ opt in</button>
      {/if}
    </div>
  </div>
{/snippet}

{#snippet repoCard(p: typeof data.projects[0], selected: boolean)}
  {@const ep = getEffective(p)}
  {@const isEditing = editingRepoPath === ep.path}
  <div class="group relative rounded-xl border transition-all overflow-hidden
              {selected ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3/60 bg-surface-z2/50 hover:border-surface-z4 hover:bg-surface-z2'}">
    <button onclick={() => selectProject(p.id)} class="w-full px-3 py-2.5 text-left">
      <div class="flex items-center gap-2 pr-5">
        <span class="text-sm shrink-0 {p.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
        <span class="flex-1 truncate text-sm font-semibold {selected ? 'text-primary-z8' : 'text-surface-z8'}">{ep.name}</span>
        {#if (ep as { driftCount?: number | null }).driftCount != null}
          {@const dc = (ep as { driftCount: number }).driftCount}
          <span
            title={(ep as { driftSummary?: string }).driftSummary ?? ''}
            class="shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-medium
                   {dc === 0 ? 'bg-success-z2 text-success-z7' : 'bg-warning-z2 text-warning-z7'}"
          >{dc === 0 ? 'synced' : `${dc} drift`}</span>
        {:else if ep.scanStatus}
          <span class="shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-medium capitalize {SCAN_STATUS_CLS[ep.scanStatus] ?? SCAN_STATUS_CLS.unknown}">{ep.scanStatus}</span>
        {/if}
      </div>
      {#if ep.description && ep.description !== ep.name}
        <p class="mt-0.5 truncate text-[11px] {selected ? 'text-primary-z6' : 'text-surface-z4'}">{ep.description}</p>
      {/if}
      <div class="mt-1.5 flex items-center gap-1 flex-wrap">
        {#each (ep.tech_stack ?? []).slice(0, 2) as t}
          <span class="rounded px-1.5 py-0.5 text-[9px] {selected ? 'bg-primary-z2 text-primary-z6' : 'bg-surface-z3 text-surface-z5'}">{t}</span>
        {/each}
        {#if groupMode === 'variants' && ep.client}
          <span class="rounded-full border px-1.5 py-0.5 text-[9px] {selected ? 'border-primary-z3 text-primary-z6' : 'border-surface-z3 text-surface-z5'}">{ep.client}</span>
        {/if}
        <span class="ml-auto text-[10px] {selected ? 'text-primary-z5' : 'text-surface-z4'}">{@render dayLabel(ep.last_commit_days ?? null)}</span>
      </div>
    </button>
    <button
      onclick={(e) => { e.stopPropagation(); isEditing ? cancelEdit() : startEdit(ep); }}
      title={isEditing ? 'Cancel edit' : 'Edit tags'}
      class="absolute right-1.5 top-1.5 rounded-md p-1 opacity-0 group-hover:opacity-100 text-surface-z4 hover:text-surface-z7 hover:bg-surface-z3/60 transition-all"
    ><span class="{isEditing ? 'i-solar-close-square-bold-duotone' : 'i-solar-pen-bold-duotone'} text-xs"></span></button>
    {#if isEditing}{@render editForm(ep)}{/if}
  </div>
{/snippet}

{#snippet memberRow(p: typeof data.projects[0])}
  {@const ep = getEffective(p)}
  {@const isEditing = editingRepoPath === ep.path}
  <div class="group">
    <div class="flex items-center gap-2 transition-colors {selectedId === p.id ? 'bg-primary-z2' : 'hover:bg-surface-z2'}">
      <button onclick={() => selectProject(p.id)} class="flex flex-1 items-center gap-2 px-4 py-2 min-w-0 text-left">
        <span class="text-xs shrink-0 {p.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z5' : 'i-solar-code-square-bold-duotone text-primary-z5'}"></span>
        <span class="flex-1 truncate text-xs font-medium {selectedId === p.id ? 'text-primary-z8' : 'text-surface-z7'}">{ep.name}</span>
        {#if ep.scanStatus}
          <span class="shrink-0 rounded-full px-1.5 py-0.5 text-[9px] capitalize {SCAN_STATUS_CLS[ep.scanStatus] ?? SCAN_STATUS_CLS.unknown}">{ep.scanStatus}</span>
        {/if}
      </button>
      <button
        onclick={(e) => { e.stopPropagation(); isEditing ? cancelEdit() : startEdit(ep); }}
        title={isEditing ? 'Cancel' : 'Edit'}
        class="mr-2 shrink-0 rounded-md p-0.5 opacity-0 group-hover:opacity-100 text-surface-z4 hover:text-surface-z7 transition-all"
      ><span class="{isEditing ? 'i-solar-close-square-bold-duotone' : 'i-solar-pen-bold-duotone'} text-xs"></span></button>
    </div>
    {#if isEditing}{@render editForm(ep)}{/if}
  </div>
{/snippet}

<div class="flex h-full min-h-0">

  <!-- ══ NAV PANEL ════════════════════════════════════════════════════ -->
  <div class="flex w-72 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">

    <!-- Top bar -->
    <div class="border-b border-surface-z0/50 px-3 py-2 shrink-0 space-y-2">
      <div class="flex items-center gap-2">
        <h1 class="text-sm font-semibold text-surface-z8 mr-auto">Projects</h1>
        <Toggle
          bind:value={groupMode}
          options={[
            { value: 'variants', icon: 'i-solar-layers-minimalistic-bold-duotone', label: 'Variants', description: 'Group by variant' },
            { value: 'clients',  icon: 'i-solar-buildings-bold-duotone',            label: 'Clients',  description: 'Group by client'  },
          ]}
          showLabels={false}
          size="sm"
        />
        <button
          onclick={openAdd}
          class="flex items-center gap-1 rounded-lg border border-surface-z3 bg-surface-z2 px-2 py-1 text-xs text-surface-z6 transition-colors hover:bg-surface-z3"
        >
          <span class="i-solar-add-circle-bold-duotone text-sm"></span>
          Add
        </button>
      </div>
      <!-- Search -->
      <div class="relative">
        <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-xs i-solar-magnifer-bold-duotone text-surface-z4 pointer-events-none"></span>
        <input
          bind:value={search}
          placeholder="Search projects…"
          class="w-full rounded-lg border border-surface-z3 bg-surface-z1 py-1.5 pl-7 pr-3 text-xs outline-none placeholder:text-surface-z4 focus:border-primary-z4 transition-colors"
        />
        {#if search}
          <button
            onclick={() => search = ''}
            class="absolute right-2 top-1/2 -translate-y-1/2 text-surface-z4 hover:text-surface-z7"
            aria-label="Clear search"
          >
            <span class="i-solar-close-circle-bold-duotone text-sm"></span>
          </button>
        {/if}
      </div>
    </div>

    <!-- Nav list -->
    <div class="flex-1 overflow-y-auto">

      {#if navEntries.length === 0}
        <div class="flex flex-col items-center justify-center h-full gap-3 py-12 text-center">
          <span class="i-solar-folder-with-files-bold-duotone text-3xl text-surface-z3"></span>
          <p class="text-sm text-surface-z5">No projects yet</p>
          <button onclick={openAdd} class="rounded-lg bg-primary-z6 px-3 py-1.5 text-xs font-semibold text-white hover:bg-primary-z7 transition-colors">
            Import your first project
          </button>
        </div>

      {:else if isSearching}
        <!-- ── Search results (flat list, groups auto-expanded) ── -->
        <div class="px-2 pt-2 pb-3 space-y-1">
          {#if filteredEntries.length === 0}
            <div class="flex flex-col items-center justify-center py-10 text-center gap-2">
              <span class="i-solar-magnifer-bold-duotone text-2xl text-surface-z3"></span>
              <p class="text-xs text-surface-z5">No matches for "{search}"</p>
            </div>
          {:else}
            {#each filteredEntries as entry}
              {#if entry.type === 'group'}
                {@const groupSelected = selectedGroupKey === entry.key}
                <div class="rounded-xl border overflow-hidden transition-all
                            {groupSelected ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3/60 bg-surface-z2/50'}">
                  <!-- Group header -->
                  <button
                    onclick={() => selectGroup(entry.key)}
                    class="w-full px-3 py-2 text-left"
                  >
                    <div class="flex items-center gap-2">
                      <span class="i-solar-layers-minimalistic-bold-duotone text-sm shrink-0 {groupSelected ? 'text-primary-z6' : 'text-info-z6'}"></span>
                      <span class="flex-1 truncate text-sm font-semibold {groupSelected ? 'text-primary-z8' : 'text-surface-z8'}">{entry.key}</span>
                      {#if !entry.groupMatches}
                        <span class="text-[9px] rounded-full bg-info-z2 text-info-z7 px-1.5 py-0.5 shrink-0">{entry.matchedItems.length} of {entry.items.length}</span>
                      {:else}
                        <span class="text-[10px] text-surface-z4">{entry.items.length} repos</span>
                      {/if}
                    </div>
                  </button>
                  <!-- Always-expanded member list in search mode -->
                  <div class="border-t border-surface-z2 divide-y divide-surface-z0/30">
                    {#each entry.matchedItems as p (p.id)}
                      {@render memberRow(p)}
                    {/each}
                    {#if !entry.groupMatches && entry.matchedItems.length < entry.items.length}
                      <p class="px-4 py-1.5 text-[10px] text-surface-z4">{entry.items.length - entry.matchedItems.length} more not matching</p>
                    {/if}
                  </div>
                </div>
              {:else}
                {@render repoCard(entry.item, selectedId === entry.item.id)}
              {/if}
            {/each}
          {/if}
        </div>

      {:else}
        <!-- ── Normal view: RECENT + OLDER ── -->
        <!-- RECENT -->
        <div class="px-3 pt-3 pb-1">
          <p class="text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Recent</p>
        </div>
        <div class="px-2 pb-2 space-y-1">
          {#each recentEntries as entry}

            {#if entry.type === 'group'}
              <!-- Group card -->
              {@const expanded = expandedGroupKeys.has(entry.key)}
              {@const groupSelected = selectedGroupKey === entry.key}
              <div class="rounded-xl border transition-all overflow-hidden
                          {groupSelected ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3/60 bg-surface-z2/50 hover:border-surface-z4'}">
                <button
                  onclick={() => { toggleGroup(entry.key); if (!expanded) selectGroup(entry.key); }}
                  class="w-full px-3 py-2.5 text-left"
                >
                  <div class="flex items-center gap-2">
                    <span class="i-solar-layers-minimalistic-bold-duotone text-sm shrink-0 {groupSelected ? 'text-primary-z6' : 'text-info-z6'}"></span>
                    <span class="flex-1 truncate text-sm font-semibold {groupSelected ? 'text-primary-z8' : 'text-surface-z8'}">{entry.key}</span>
                    <span class="text-[10px] text-surface-z4">{entry.items.length} repos</span>
                    <span class="text-xs {expanded ? 'i-solar-alt-arrow-up-bold-duotone' : 'i-solar-alt-arrow-down-bold-duotone'} text-surface-z4 shrink-0"></span>
                  </div>
                  <div class="mt-1 flex items-center gap-1.5">
                    {#each entry.categories.slice(0, 3) as cat}
                      <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5 capitalize">{cat}</span>
                    {/each}
                    {#if entry.minDays != null}
                      <span class="ml-auto text-[10px] text-surface-z4">{entry.minDays === 0 ? 'Today' : entry.minDays < 7 ? entry.minDays + 'd ago' : entry.minDays < 30 ? Math.floor(entry.minDays / 7) + 'w ago' : Math.floor(entry.minDays / 30) + 'mo ago'}</span>
                    {/if}
                  </div>
                </button>
                {#if expanded}
                  <div class="border-t border-surface-z2 divide-y divide-surface-z0/30">
                    {#each entry.items as p (p.id)}
                      {@render memberRow(p)}
                    {/each}
                  </div>
                {/if}
              </div>

            {:else}
              {@render repoCard(entry.item, selectedId === entry.item.id)}
            {/if}

          {/each}
        </div>

        <!-- OLDER -->
        {#if olderEntries.length > 0}
          <div class="px-3 pt-2 pb-1.5 flex items-center justify-between">
            <p class="text-[10px] font-semibold uppercase tracking-widest text-surface-z4">Older</p>
            <button
              onclick={() => showAllOlder = !showAllOlder}
              class="flex items-center gap-1 text-[10px] text-surface-z4 hover:text-surface-z7 transition-colors"
            >
              Browse
              <span class="text-[8px] {showAllOlder ? 'i-solar-alt-arrow-up-bold-duotone' : 'i-solar-alt-arrow-down-bold-duotone'}"></span>
            </button>
          </div>

          {#if showAllOlder}
            <!-- Expanded compact list -->
            <div class="px-2 pb-3 space-y-0.5">
              {#each olderEntries as entry}
                {#if entry.type === 'group'}
                  <button
                    onclick={() => selectGroup(entry.key)}
                    class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors
                           {selectedGroupKey === entry.key ? 'bg-primary-z1 text-primary-z8' : 'hover:bg-surface-z2 text-surface-z7'}"
                  >
                    <span class="i-solar-layers-minimalistic-bold-duotone text-xs text-info-z5 shrink-0"></span>
                    <span class="flex-1 truncate text-xs font-medium">{entry.key}</span>
                    <span class="text-[9px] text-surface-z4">{entry.items.length}</span>
                    {#if entry.minDays != null}
                      <span class="text-[9px] text-surface-z4">{entry.minDays < 30 ? entry.minDays + 'd' : Math.floor(entry.minDays / 30) + 'mo'}</span>
                    {/if}
                  </button>
                {:else}
                  <button
                    onclick={() => selectProject(entry.item.id)}
                    class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left transition-colors
                           {selectedId === entry.item.id ? 'bg-primary-z1 text-primary-z8' : 'hover:bg-surface-z2 text-surface-z7'}"
                  >
                    <span class="text-xs shrink-0 {entry.item.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z5' : 'i-solar-code-square-bold-duotone text-primary-z5'}"></span>
                    <span class="flex-1 truncate text-xs font-medium">{entry.item.name}</span>
                    {#if entry.days != null}
                      <span class="text-[9px] text-surface-z4">{entry.days < 30 ? entry.days + 'd' : Math.floor(entry.days / 30) + 'mo'}</span>
                    {/if}
                  </button>
                {/if}
              {/each}
            </div>

          {:else}
            <!-- Compact chip row -->
            <div class="px-3 pb-4 flex flex-wrap gap-1.5">
              {#each olderEntries.slice(0, 10) as entry}
                {#if entry.type === 'group'}
                  <button
                    onclick={() => selectGroup(entry.key)}
                    class="flex items-center gap-1 rounded-full border px-2.5 py-1 text-[10px] transition-colors
                           {selectedGroupKey === entry.key ? 'border-primary-z4 bg-primary-z1 text-primary-z7' : 'border-surface-z3 bg-surface-z2 text-surface-z6 hover:border-surface-z4'}"
                  >
                    <span class="i-solar-layers-minimalistic-bold-duotone text-[9px] text-info-z5"></span>
                    {entry.key}
                  </button>
                {:else}
                  <button
                    onclick={() => selectProject(entry.item.id)}
                    class="rounded-full border px-2.5 py-1 text-[10px] transition-colors
                           {selectedId === entry.item.id ? 'border-primary-z4 bg-primary-z1 text-primary-z7' : 'border-surface-z3 bg-surface-z2 text-surface-z6 hover:border-surface-z4'}"
                  >{entry.item.name}</button>
                {/if}
              {/each}
              {#if olderEntries.length > 10}
                <span class="flex items-center px-1 text-[10px] text-surface-z4">+{olderEntries.length - 10} more</span>
              {/if}
            </div>
          {/if}
        {/if}

      {/if}
    </div>
  </div>

  <!-- ══ DETAIL AREA ═══════════════════════════════════════════════════ -->
  {#if selectedGroupKey && !project}
    {@const groupEntry = navEntries.find(e => e.type === 'group' && e.key === selectedGroupKey)}
    {#if groupEntry && groupEntry.type === 'group'}
      <div class="flex flex-1 min-w-0 flex-col overflow-hidden">
        <!-- Group header -->
        <div class="border-b border-surface-z0/50 px-5 py-4 shrink-0">
          <div class="flex items-center gap-2 mb-1">
            <span class="i-solar-layers-minimalistic-bold-duotone text-lg text-info-z6"></span>
            <h2 class="text-base font-semibold text-surface-z9 capitalize">{groupEntry.key}</h2>
            <span class="rounded-full bg-surface-z3 px-2 py-0.5 text-[10px] text-surface-z5">{groupEntry.items.length} variants</span>
          </div>
          <p class="text-xs text-surface-z5">These repos share a common name stem. Pick one to work on, or ask about the group as a whole.</p>
        </div>
        <!-- Member list -->
        <div class="flex-1 overflow-y-auto px-5 py-4 space-y-2">
          {#each groupEntry.items as p (p.id)}
            <button
              onclick={() => selectProject(p.id)}
              class="flex w-full items-start gap-3 rounded-xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-3 text-left transition-all hover:border-surface-z4 hover:bg-surface-z2"
            >
              <span class="mt-0.5 text-base shrink-0 {p.kind === 'idea' ? 'i-solar-lightbulb-bold-duotone text-warning-z6' : 'i-solar-code-square-bold-duotone text-primary-z6'}"></span>
              <div class="min-w-0 flex-1">
                <p class="text-sm font-semibold text-surface-z8">{p.name}</p>
                {#if p.description && p.description !== p.name}
                  <p class="mt-0.5 text-xs text-surface-z5 line-clamp-1">{p.description}</p>
                {/if}
                <div class="mt-1.5 flex items-center gap-1.5 flex-wrap">
                  {#each (p.tech_stack ?? []).slice(0, 3) as t}
                    <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z5">{t}</span>
                  {/each}
                  <span class="ml-auto text-[10px] text-surface-z4">{p.lastActivity}</span>
                </div>
              </div>
              {#if p.scanStatus}
                <span class="shrink-0 rounded-full px-2 py-0.5 text-[9px] font-medium capitalize mt-0.5 {SCAN_STATUS_CLS[p.scanStatus] ?? SCAN_STATUS_CLS.unknown}">{p.scanStatus}</span>
              {/if}
            </button>
          {/each}
        </div>
        <!-- Group prompt bar -->
        <div class="border-t border-surface-z0/50 bg-surface-z2/60 px-4 py-2.5 backdrop-blur-sm shrink-0">
          <div class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z1 px-3 py-2 focus-within:border-primary-z4 transition-all">
            <span class="i-solar-magic-stick-3-bold-duotone text-sm text-primary-z6 shrink-0"></span>
            <input
              bind:value={prompt}
              placeholder="Ask about all {groupEntry.key} variants…"
              class="flex-1 bg-transparent text-sm text-surface-z7 outline-none placeholder:text-surface-z4"
            />
            <kbd class="rounded border border-surface-z3 px-1.5 py-0.5 text-[9px] text-surface-z4">⏎</kbd>
          </div>
        </div>
      </div>
    {/if}

  {:else if project}
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

            <!-- Tab bar: always shown; cards/graph/sessions dimmed when no data -->
            <div class="mt-3 flex gap-1">
              {#each (['cards', 'graph', 'sessions', 'libraries'] as const) as tab}
                <button
                  onclick={() => { if (hasData || tab === 'libraries') activeTab = tab; }}
                  class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors capitalize
                         {activeTab === tab ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}
                         {!hasData && tab !== 'libraries' ? 'opacity-30 pointer-events-none' : ''}"
                >{tab}</button>
              {/each}
            </div>
          </div>

          <!-- Tab content -->
          {#if activeTab === 'libraries'}
            <!-- ── Libraries tab — always available ── -->
            <div class="flex-1 overflow-y-auto">
              {#if libLoading.has(project.path)}
                <div class="px-5 py-10 text-center">
                  <span class="i-solar-refresh-circle-bold-duotone text-2xl text-surface-z3 block mx-auto mb-2 animate-spin"></span>
                  <p class="text-xs text-surface-z4">Scanning imports…</p>
                </div>
              {:else}
                {@const allDeps = libData.get(project.path) ?? []}
                {@const highDeps = allDeps.filter((d: DetectedDep) => d.usage_pct >= LIB_AUTO_THRESHOLD)}
                {@const midDeps = allDeps.filter((d: DetectedDep) => d.usage_pct >= LIB_SUGGEST_MIN && d.usage_pct < LIB_AUTO_THRESHOLD)}
                {@const lowDeps = allDeps.filter((d: DetectedDep) => d.usage_pct < LIB_SUGGEST_MIN)}
                {#if allDeps.length === 0}
                  <div class="px-5 py-10 text-center space-y-2">
                    <span class="i-solar-box-bold-duotone text-2xl text-surface-z3 block mx-auto"></span>
                    <p class="text-sm font-medium text-surface-z7">No imports detected</p>
                    <p class="text-xs text-surface-z4">Start a session in Claude Code first, or ensure source files exist at this path.</p>
                  </div>
                {:else}
                  {@const idxStatus = indexStates[project.path] ?? 'idle'}
                  {@const optedCount = allDeps.filter((d: DetectedDep) => isOptedIn(project.path, d)).length}
                  <!-- Header: stats + index trigger -->
                  <div class="px-5 pt-4 pb-3 flex items-center justify-between gap-3">
                    <p class="text-xs text-surface-z5">{allDeps.length} packages · {allDeps[0]?.total_files ?? 0} source files</p>
                    {#if idxStatus === 'done'}
                      <button
                        onclick={() => triggerIndex(project.path)}
                        class="flex items-center gap-1 text-[10px] font-medium text-success-z6 hover:text-primary-z6 transition-colors shrink-0"
                        title="Re-index"
                      >
                        <span class="i-solar-check-circle-bold-duotone text-xs"></span> Indexed
                      </button>
                    {:else if idxStatus === 'running'}
                      <span class="flex items-center gap-1 text-[10px] text-primary-z6 animate-pulse shrink-0">
                        <span class="i-solar-restart-bold-duotone text-xs"></span> Indexing…
                      </span>
                    {:else if idxStatus === 'failed'}
                      <button
                        onclick={() => triggerIndex(project.path)}
                        class="flex shrink-0 items-center gap-1 rounded-lg bg-danger-z5 px-2.5 py-1 text-[10px] font-semibold text-white hover:bg-danger-z6 transition-colors"
                      ><span class="i-solar-refresh-bold-duotone text-xs"></span> Retry</button>
                    {:else}
                      <button
                        onclick={() => triggerIndex(project.path)}
                        disabled={optedCount === 0}
                        class="flex shrink-0 items-center gap-1 rounded-lg bg-primary-z6 px-2.5 py-1 text-[10px] font-semibold text-white hover:bg-primary-z7 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                      ><span class="i-solar-refresh-bold-duotone text-xs"></span> Index {optedCount}</button>
                    {/if}
                  </div>

                  <!-- Auto-indexed (≥30%) -->
                  {#if highDeps.length > 0}
                    <div class="px-5 pb-3">
                      <p class="mb-2 text-[10px] font-semibold uppercase tracking-widest text-success-z6">Auto-indexed · ≥30%</p>
                      <div class="space-y-0.5">
                        {#each highDeps as dep (dep.name)}{@render depRow(project.path, dep)}{/each}
                      </div>
                    </div>
                  {/if}

                  <!-- Suggested (10–29%) -->
                  {#if midDeps.length > 0}
                    <div class="px-5 pb-3">
                      <p class="mb-2 text-[10px] font-semibold uppercase tracking-widest text-warning-z6">Suggested · 10–29%</p>
                      <div class="space-y-0.5">
                        {#each midDeps as dep (dep.name)}{@render depRow(project.path, dep)}{/each}
                      </div>
                    </div>
                  {/if}

                  <!-- Low usage (<10%, collapsible) -->
                  {#if lowDeps.length > 0}
                    <div class="px-5 pb-4">
                      <button
                        onclick={() => showAllLibs = !showAllLibs}
                        class="flex items-center gap-1.5 text-[10px] text-surface-z4 hover:text-surface-z7 transition-colors"
                      >
                        <span class="text-[8px] {showAllLibs ? 'i-solar-alt-arrow-up-bold-duotone' : 'i-solar-alt-arrow-down-bold-duotone'}"></span>
                        {showAllLibs ? 'Hide' : 'Show'} {lowDeps.length} low-usage packages
                      </button>
                      {#if showAllLibs}
                        <div class="mt-2 space-y-0.5">
                          {#each lowDeps as dep (dep.name)}{@render depRow(project.path, dep)}{/each}
                        </div>
                      {/if}
                    </div>
                  {/if}

                  <p class="px-5 pb-5 text-[10px] text-surface-z3 leading-relaxed">
                    Opted-in libraries are indexed by the Sensei daemon and kept in sync with code changes.
                  </p>
                {/if}
              {/if}
            </div>
          {:else if !hasData}
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
      <div class="text-center space-y-2">
        <span class="i-solar-cursor-bold-duotone text-3xl text-surface-z3 block mx-auto"></span>
        <p class="text-sm text-surface-z5">Select a project or group</p>
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
          <div class="flex flex-col gap-3 h-full">
            <!-- Folder input -->
            <div class="flex gap-2 shrink-0">
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
              <RepoList repos={scanned} bind:selected={scanSelected} bind:clientTags={scanClientTags} />
              <button
                onclick={startIndexing}
                disabled={scanSelected.size === 0}
                class="shrink-0 w-full rounded-xl py-2.5 text-sm font-semibold transition-colors
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
