<script lang="ts">
  import { onMount } from 'svelte';
  import RepoList from '$lib/RepoList.svelte';
  import { createSolution, setActiveSolutionId, getActiveSolutionId, inferRepoRole, loadSolutions } from '$lib/solutions.svelte.js';
  import type { SolutionRepo } from '$lib/types.js';

  type Step = 'welcome' | 'acps' | 'folders' | 'repos' | 'groups' | 'done';
  let step = $state<Step>('welcome');

  // ── ACPs (Agent Coding Platforms) ─────────────────────────────────────────
  const acps = [
    { id: 'claude-desktop', label: 'Claude Desktop', icon: 'i-solar-cpu-bold-duotone',         desc: 'Anthropic desktop app' },
    { id: 'claude-code',    label: 'Claude Code',    icon: 'i-solar-code-square-bold-duotone', desc: 'CLI + VS Code extension' },
    { id: 'cursor',         label: 'Cursor',         icon: 'i-solar-cursor-bold-duotone',      desc: 'AI-first code editor' },
    { id: 'windsurf',       label: 'Windsurf',       icon: 'i-solar-wind-bold-duotone',        desc: 'Codeium editor' },
    { id: 'zed',            label: 'Zed',            icon: 'i-solar-bolt-bold-duotone',        desc: 'High-performance code editor' },
    { id: 'kiro',           label: 'Kiro',           icon: 'i-solar-programming-bold-duotone', desc: 'AWS AI IDE' },
    { id: 'opencode',       label: 'OpenCode',       icon: 'i-solar-terminal-bold-duotone',    desc: 'Terminal AI coding by SST' },
  ];

  let detected    = $state<string[]>([]);
  let selected    = $state<Set<string>>(new Set());
  let configuring = $state(false);
  let configured  = $state<string[]>([]);
  let configError = $state<string | null>(null);

  async function loadDetected() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      detected = await invoke<string[]>('detect_acps');
      selected = new Set(detected);
    } catch { /* browser preview */ }
  }

  function toggle(id: string) {
    const s = new Set(selected);
    s.has(id) ? s.delete(id) : s.add(id);
    selected = s;
  }

  async function configureMcp() {
    if (selected.size === 0) { step = 'folders'; return; }
    configuring = true; configError = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      configured = await invoke<string[]>('configure_mcp', { acps: [...selected] });
      await new Promise(r => setTimeout(r, 500));
      step = 'folders';
    } catch (e) {
      // Tauri not available (browser preview) — skip MCP config and continue
      if (String(e).includes('invoke') || String(e).includes('__TAURI__')) {
        step = 'folders';
      } else {
        configError = String(e);
      }
    }
    finally { configuring = false; }
  }

  // ── Repo model ────────────────────────────────────────────────────────────────
  type Repo = {
    name: string;
    path: string;
    remote: string | null;
    description: string | null;
    categories: string[];
    status: 'active' | 'recent' | 'stale' | 'archived' | 'abandoned' | 'unknown';
    last_commit_days: number | null;
    tech_stack: string[];
    commit_count: number;
    duplicate_of: string | null;
    variant_group: string | null;
  };


  // ── Scan folders ──────────────────────────────────────────────────────────────
  let scanRoots    = $state<string[]>([]);
  let rootInput    = $state('~/Developer');
  let scanning     = $state(false);
  let discovered   = $state<Repo[]>([]);
  let selectedRepos = $state<Set<string>>(new Set());
  function addRoot(path: string) {
    const t = path.trim();
    if (t && !scanRoots.includes(t)) scanRoots = [...scanRoots, t];
  }

  function removeRoot(root: string) {
    scanRoots = scanRoots.filter(r => r !== root);
  }

  async function pickAndAddFolder() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const picked = await invoke<string | null>('pick_folder');
      if (picked) addRoot(picked);
    } catch { /* browser preview */ }
  }

  async function scanAll() {
    if (scanRoots.length === 0) return;
    scanning = true;
    discovered = [];
    selectedRepos = new Set();
    variantOverrides = new Map();

    try {
      // Try Tauri first (native desktop)
      const { invoke } = await import('@tauri-apps/api/core');
      const results = await Promise.all(
        scanRoots.map(root => invoke<Repo[]>('analyze_folder', { root }))
      );
      const seen = new Set<string>();
      for (const repos of results) {
        for (const r of repos) {
          if (!seen.has(r.path)) { seen.add(r.path); discovered.push(r); }
        }
      }
    } catch {
      // Browser fallback — use daemon API
      const { senseiApi } = await import('$lib/api.js');
      const { getPort } = await import('$lib/appstate.svelte.js');
      const api = senseiApi(getPort());
      for (const root of scanRoots) {
        const scanned = await api.scanFolder(root);
        for (const r of scanned) {
          discovered.push({
            name: r.name, path: r.path, remote: null, description: null,
            categories: [], status: 'active' as any, last_commit_days: null,
            tech_stack: r.stack ?? [], commit_count: 0,
            duplicate_of: null, variant_group: null,
          });
        }
      }
    }

    // Auto-select active/recent repos
    selectedRepos = new Set(
      discovered
        .filter(r => !r.duplicate_of)
        .map(r => r.path)
    );

    // Register all with daemon
    const { senseiApi } = await import('$lib/api.js');
    const { getPort } = await import('$lib/appstate.svelte.js');
    const api = senseiApi(getPort());
    for (const r of discovered) {
      await api.registerProject(r.name, r.name, r.path);
    }

    scanning = false;
  }

  const selectedCount = $derived(selectedRepos.size);

  // Used in groups step rows
  const STATUS_CLS: Record<string, string> = {
    active: 'bg-success-z2 text-success-z7', recent: 'bg-primary-z2 text-primary-z7',
    stale: 'bg-warning-z2 text-warning-z7', archived: 'bg-surface-z3 text-surface-z5',
    abandoned: 'bg-error-z2 text-error-z7', unknown: 'bg-surface-z3 text-surface-z5',
  };
  const CAT_CLS: Record<string, string> = {
    app: 'bg-primary-z2 text-primary-z6', library: 'bg-info-z2 text-info-z6',
    tool: 'bg-warning-z2 text-warning-z7', idea: 'bg-surface-z3 text-surface-z5',
    unknown: 'bg-surface-z2 text-surface-z4',
  };
  function relPath(fullPath: string): string {
    const parts = fullPath.split('/');
    return parts.length >= 2 ? parts.slice(-2).join('/') : fullPath;
  }

  // ── Variant group management ──────────────────────────────────────────────────
  let clientTags = $state<Map<string, string>>(new Map());

  let variantOverrides = $state<Map<string, string | null>>(new Map());

  function effectiveGroup(r: Repo): string | null {
    return variantOverrides.has(r.path) ? (variantOverrides.get(r.path) ?? null) : r.variant_group;
  }

  function removeFromGroup(e: MouseEvent, path: string) {
    e.stopPropagation();
    variantOverrides = new Map(variantOverrides).set(path, null);
  }

  function disbandGroup(repos: Repo[]) {
    const m = new Map(variantOverrides);
    for (const r of repos) m.set(r.path, null);
    variantOverrides = m;
  }

  function nameStem(name: string): string {
    const n = name.toLowerCase();
    for (const s of ['-backup','-prototype','-v1','-v2','-v3','-v4','-v5',
                     '-old','-new','-bak','-copy','-alt','-next',
                     '-poc','-demo','-test','-idea','-draft',
                     '_backup','_old','_bak','_copy','_new']) {
      if (n.endsWith(s) && n.length - s.length >= 3) return n.slice(0, -s.length);
    }
    const t = n.replace(/\d+$/, '').replace(/[-_]$/, '');
    return t.length >= 3 && t.length < n.length ? t : n;
  }

  function variantClusters(repos: Repo[]): Map<string, Repo[]> {
    const map = new Map<string, Repo[]>();
    for (const r of repos) {
      const g = effectiveGroup(r);
      if (g) { const a = map.get(g) ?? []; a.push(r); map.set(g, a); }
    }
    for (const [k, v] of map) { if (v.length < 2) map.delete(k); }
    return map;
  }

  // Clusters among currently selected repos (used in groups step)
  const selectedClusters = $derived.by(() =>
    variantClusters(discovered.filter(r => selectedRepos.has(r.path)))
  );

  // Groups step: repos the user has ticked for manual merging
  let mergeSelected = $state<Set<string>>(new Set());

  function toggleMerge(path: string) {
    const s = new Set(mergeSelected);
    s.has(path) ? s.delete(path) : s.add(path);
    mergeSelected = s;
  }

  function mergeIntoGroup() {
    if (mergeSelected.size < 2) return;
    const names = [...mergeSelected].map(p => discovered.find(d => d.path === p)?.name ?? '');
    const key = 'manual:' + names.sort((a, b) => a.length - b.length)[0];
    const m = new Map(variantOverrides);
    for (const p of mergeSelected) m.set(p, key);
    variantOverrides = m;
    mergeSelected = new Set();
  }

  const ungroupedSelected = $derived(
    discovered.filter(r => selectedRepos.has(r.path) && !effectiveGroup(r))
  );

  const selectedDuplicates = $derived(
    discovered.filter(r => selectedRepos.has(r.path) && !!r.duplicate_of)
  );

  // ── Flow control ──────────────────────────────────────────────────────────────
  function continueToGroups() {
    if (selectedClusters.size > 0) { mergeSelected = new Set(); step = 'groups'; }
    else startIndexing();
  }

  async function startIndexing() {
    const toImport = discovered.filter(r => selectedRepos.has(r.path)).map(r => ({
      ...r,
      client: clientTags.get(r.path) ?? null,
    }));

    if (toImport.length > 0) {
      // Resolve repoIds
      let withIds: (typeof toImport[0] & { repoId: string })[];
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        withIds = await Promise.all(toImport.map(async repo => {
          const repoId: string = await invoke<string | null>('get_repo_id', { path: repo.path })
            .catch(() => null) ?? repo.name;
          return { ...repo, repoId };
        }));
      } catch {
        withIds = toImport.map(repo => ({ ...repo, repoId: repo.name }));
      }

      // Register with daemon API
      const { senseiApi } = await import('$lib/api.js');
      const { getPort } = await import('$lib/appstate.svelte.js');
      const api = senseiApi(getPort());
      for (const repo of withIds) {
        await api.registerProject(repo.repoId, repo.name, repo.path);
      }

      // Store withIds for createSolutionsFromScan
      _importedRepos = withIds;
    }

    createSolutionsFromScan();
    step = 'done';
  }

  let _importedRepos: Array<{ path: string; repoId: string; name: string }> = [];

  function createSolutionsFromScan() {
    const idMap: Record<string, string> = {};
    for (const r of _importedRepos) idMap[r.path] = r.repoId;

    function repoId(path: string): string { return idMap[path] ?? path.split('/').at(-1) ?? path; }

    const selected = discovered.filter(r => selectedRepos.has(r.path));
    const clusters = variantClusters(selected);

    // Group clusters → solutions
    const grouped = new Set<string>();
    for (const [groupName, members] of clusters) {
      if (members.length < 2) continue;
      const repos: SolutionRepo[] = members.map(r => ({
        repoId: repoId(r.path),
        path: r.path,
        role: inferRepoRole(r as any),
        label: r.name,
      }));
      const allIdeas = members.every(r => r.categories?.includes('idea'));
      const sol = createSolution(
        groupName.replace(/^manual:/, '').replace(/[-_]/g, ' ').replace(/\b\w/g, c => c.toUpperCase()),
        repos,
        { category: allIdeas ? 'idea' : 'active', client: clientTags.get(members[0].path) },
      );
      if (!allIdeas) setActiveSolutionId(sol.id);
      for (const m of members) grouped.add(m.path);
    }

    // Ungrouped repos → individual solutions
    for (const r of selected) {
      if (grouped.has(r.path)) continue;
      const isIdea = r.categories?.includes('idea');
      const sol = createSolution(r.name, [{
        repoId: repoId(r.path),
        path: r.path,
        role: inferRepoRole(r as any),
        label: r.name,
      }], {
        category: isIdea ? 'idea' : 'active',
        client: clientTags.get(r.path),
      });
      if (!isIdea && !getActiveSolutionId()) setActiveSolutionId(sol.id);
    }
  }

  async function finish() {
    const { setSetupComplete, getActiveSolutionId: getActive } = await import('$lib/appstate.svelte.js');
    await setSetupComplete();
    const activeSolution = getActive();
    window.location.replace(activeSolution ? `/s/${activeSolution}` : '/all');
  }

  onMount(() => { loadDetected(); });
</script>

<div class="drag-region flex h-screen flex-col bg-surface-z1 select-none overflow-hidden">
  <!-- Traffic light clearance -->
  <div class="absolute top-0 left-0 right-0 h-10 pointer-events-none shrink-0"></div>

  <!-- ══ WELCOME ══════════════════════════════════════════════════════════════ -->
  {#if step === 'welcome'}
    <div class="no-drag flex flex-1 flex-col items-center justify-center gap-6 px-8 text-center">
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-primary-z6 text-2xl font-bold text-white">⬡</div>
      <div class="max-w-xs">
        <h1 class="text-2xl font-bold text-surface-z9">Welcome to Sensei</h1>
        <p class="mt-2 text-sm text-surface-z5 leading-relaxed">
          Track sessions, build a knowledge graph of your codebase, and work smarter with every AI coding session.
        </p>
      </div>
      <div class="flex flex-col gap-2 w-full max-w-xs text-sm text-surface-z5">
        {#each ['Connect your ACPs via MCP', 'Import project folders to index', 'Sessions · symbols · graph · libraries'] as item}
          <div class="flex items-center gap-2.5 rounded-lg bg-surface-z2 px-4 py-2.5">
            <span class="h-1.5 w-1.5 rounded-full bg-primary-z5 shrink-0"></span>
            {item}
          </div>
        {/each}
      </div>
      <button onclick={() => step = 'acps'}
        class="w-full max-w-xs rounded-xl bg-primary-z6 px-6 py-3 text-sm font-semibold text-white transition-colors hover:bg-primary-z7">
        Get started →
      </button>
    </div>

  <!-- ══ COORDINATORS ══════════════════════════════════════════════════════════ -->
  {:else if step === 'acps'}
    <div class="no-drag flex flex-1 flex-col items-center justify-center gap-5 px-8">
      <div class="w-full max-w-md">
        <p class="text-xs text-surface-z4 mb-1">Step 1 of 4</p>
        <h2 class="text-xl font-bold text-surface-z9">Connect your ACPs</h2>
        <p class="mt-1.5 text-sm text-surface-z5">Sensei will configure MCP for the ACPs you select.</p>
      </div>
      <div class="w-full max-w-md space-y-2">
        {#each acps as c}
          {@const isDet = detected.includes(c.id)}
          {@const isSel = selected.has(c.id)}
          <button onclick={() => toggle(c.id)}
            class="flex w-full items-center gap-3 rounded-xl border px-4 py-3 text-left transition-all
                   {isSel ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3 bg-surface-z2 hover:border-surface-z4'}">
            <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg {isSel ? 'bg-primary-z3' : 'bg-surface-z3'}">
              <span class="text-lg {c.icon} {isSel ? 'text-primary-z7' : 'text-surface-z5'}"></span>
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium text-surface-z8">{c.label}</span>
                {#if isDet}<span class="text-[10px] rounded-full bg-success-z2 px-1.5 py-0.5 text-success-z7">Detected</span>{/if}
              </div>
              <p class="text-xs text-surface-z4">{c.desc}</p>
            </div>
            <div class="shrink-0 h-4 w-4 rounded border-2 flex items-center justify-center
                        {isSel ? 'border-primary-z6 bg-primary-z6' : 'border-surface-z4'}">
              {#if isSel}<span class="text-[10px] font-bold text-white leading-none">✓</span>{/if}
            </div>
          </button>
        {/each}
      </div>
      {#if configError}
        <p class="w-full max-w-md text-xs text-error-z6 rounded-lg bg-error-z1 px-3 py-2">{configError}</p>
      {/if}
      <div class="flex gap-2 w-full max-w-md">
        <button onclick={() => step = 'welcome'}
          class="flex-1 rounded-xl border border-surface-z3 py-2.5 text-sm text-surface-z5 transition-colors hover:bg-surface-z2">
          Back
        </button>
        <button onclick={configureMcp} disabled={configuring}
          class="flex-1 rounded-xl py-2.5 text-sm font-semibold transition-colors
                 {configuring ? 'bg-surface-z3 text-surface-z5' : 'bg-primary-z6 text-white hover:bg-primary-z7'}">
          {#if configuring}
            <span class="inline-flex items-center gap-2">
              <span class="i-solar-refresh-bold-duotone animate-spin text-sm"></span>Configuring…
            </span>
          {:else if selected.size === 0}Skip →
          {:else}Configure {selected.size} tool{selected.size > 1 ? 's' : ''} →{/if}
        </button>
      </div>
    </div>

  <!-- ══ IMPORT ═══════════════════════════════════════════════════════════════ -->
  <!-- ══ FOLDERS ══════════════════════════════════════════════════════════════ -->
  {:else if step === 'folders'}
    <div class="no-drag flex flex-1 flex-col items-center justify-center gap-5 px-8 pt-10">
      <div class="w-full max-w-md">
        <p class="text-xs text-surface-z4 mb-1">Step 2 of 4 — Folders</p>
        <h2 class="text-xl font-bold text-surface-z9">Add folders to scan</h2>
        <p class="mt-1.5 text-sm text-surface-z5">
          Sensei recursively finds every git repo under each folder you add.
        </p>
      </div>

      <!-- Added folders -->
      <div class="w-full max-w-md space-y-1.5">
        {#each scanRoots as root}
          <div class="flex items-center gap-2 rounded-xl border border-surface-z3 bg-surface-z2 px-3.5 py-2.5 group">
            <span class="i-solar-folder-bold-duotone text-sm text-primary-z6 shrink-0"></span>
            <span class="flex-1 min-w-0 truncate font-mono text-sm text-surface-z7" title={root}>{root}</span>
            <button onclick={() => removeRoot(root)}
              class="shrink-0 opacity-0 group-hover:opacity-100 transition-opacity text-surface-z4 hover:text-error-z6 text-sm">✕</button>
          </div>
        {:else}
          <div class="rounded-xl border border-dashed border-surface-z3 px-4 py-5 text-center text-sm text-surface-z4">
            No folders added yet — add one below
          </div>
        {/each}
      </div>

      <!-- Add folder row -->
      <div class="flex gap-2 w-full max-w-md">
        <input
          bind:value={rootInput}
          onkeydown={(e) => { if (e.key === 'Enter') { addRoot(rootInput); rootInput = ''; } }}
          placeholder="~/Developer"
          class="min-w-0 flex-1 rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-2.5 font-mono text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4"
        />
        <button onclick={() => { addRoot(rootInput); rootInput = ''; }}
          class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-2.5 text-sm text-surface-z5 hover:bg-surface-z3 transition-colors font-medium">
          Add
        </button>
        <button onclick={pickAndAddFolder} title="Browse"
          class="rounded-xl border border-surface-z3 bg-surface-z2 px-3.5 text-surface-z5 hover:bg-surface-z3 transition-colors">
          <span class="i-solar-folder-open-bold-duotone text-base"></span>
        </button>
      </div>

      {#if configured.length > 0}
        <div class="w-full max-w-md flex items-center gap-2 rounded-lg bg-success-z1 border border-success-z3 px-3 py-2 text-xs text-success-z7">
          <span class="i-solar-check-circle-bold-duotone text-sm shrink-0"></span>
          MCP configured for {configured.map(id => acps.find(c => c.id === id)?.label).join(', ')}
        </div>
      {/if}

      <div class="flex gap-2 w-full max-w-md">
        <button onclick={() => step = 'acps'}
          class="flex-1 rounded-xl border border-surface-z3 py-2.5 text-sm text-surface-z5 transition-colors hover:bg-surface-z2">
          Back
        </button>
        <button
          onclick={async () => { await scanAll(); await startIndexing(); step = 'done'; }}
          disabled={scanRoots.length === 0 || scanning}
          class="flex-1 rounded-xl py-2.5 text-sm font-semibold transition-colors
                 {scanRoots.length > 0 && !scanning ? 'bg-primary-z6 text-white hover:bg-primary-z7' : 'bg-surface-z3 text-surface-z4 cursor-not-allowed'}">
          {#if scanning}
            <span class="inline-flex items-center justify-center gap-2">
              <span class="i-solar-refresh-bold-duotone animate-spin text-sm"></span>Scanning…
            </span>
          {:else}
            Scan {scanRoots.length} folder{scanRoots.length !== 1 ? 's' : ''} →
          {/if}
        </button>
      </div>
      <button onclick={finish} class="text-xs text-surface-z4 hover:text-surface-z6 transition-colors">Skip setup</button>
    </div>

  <!-- ══ REPOS ═════════════════════════════════════════════════════════════════ -->
  {:else if step === 'repos'}
    <div class="no-drag flex flex-1 min-h-0 flex-col pt-10">

      <!-- Header -->
      <div class="flex items-center gap-3 border-b border-surface-z3 px-5 py-3 shrink-0">
        <div class="flex-1">
          <p class="text-xs text-surface-z4 mb-0.5">Step 3 of 4</p>
          <h2 class="text-base font-bold text-surface-z9">Select projects to import</h2>
        </div>
      </div>

      <!-- Repo list -->
      <div class="flex flex-col flex-1 min-h-0 gap-2 px-5 py-3">
        {#if discovered.length === 0}
          <div class="flex flex-col items-center gap-2 py-16 text-center text-sm text-surface-z4">
            <span class="i-solar-folder-open-bold-duotone text-3xl text-surface-z3"></span>
            <p>No repos found — go back and check your folders</p>
          </div>
        {:else}
          <RepoList repos={discovered} bind:selected={selectedRepos} bind:clientTags />
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center gap-3 border-t border-surface-z3 px-5 py-3 shrink-0">
        <button onclick={() => step = 'folders'} class="text-sm text-surface-z4 hover:text-surface-z6 transition-colors">← Back</button>
        <button onclick={finish} class="text-xs text-surface-z4 hover:text-surface-z6 transition-colors ml-2">Skip for now</button>
        <div class="flex-1"></div>
        <button
          onclick={continueToGroups}
          disabled={selectedCount === 0 && discovered.length > 0}
          class="rounded-xl px-6 py-2.5 text-sm font-semibold transition-colors
                 {selectedCount > 0 || discovered.length === 0
                   ? 'bg-primary-z6 text-white hover:bg-primary-z7'
                   : 'bg-surface-z3 text-surface-z4 cursor-not-allowed'}">
          {selectedCount > 0 ? `Continue with ${selectedCount} →` : 'Continue →'}
        </button>
      </div>
    </div>

  <!-- ══ GROUPS ════════════════════════════════════════════════════════════════ -->
  {:else if step === 'groups'}
    <div class="no-drag flex flex-1 min-h-0 flex-col pt-10">

      <!-- Header -->
      <div class="flex items-center gap-3 border-b border-surface-z3 px-6 py-4 shrink-0">
        <div class="flex-1">
          <h2 class="text-base font-bold text-surface-z9">Review variant groups</h2>
          <p class="text-xs text-surface-z4 mt-0.5">
            {selectedClusters.size} group{selectedClusters.size !== 1 ? 's' : ''} detected among your {selectedCount} selected projects.
            Ungroup or merge as needed before importing.
          </p>
        </div>
        {#if mergeSelected.size >= 2}
          <button onclick={mergeIntoGroup}
            class="flex items-center gap-1.5 rounded-xl bg-info-z2 px-3 py-2 text-xs font-semibold text-info-z7 hover:bg-info-z3 transition-colors">
            <span class="i-solar-layers-minimalistic-bold-duotone text-sm"></span>
            Merge {mergeSelected.size} selected
          </button>
        {/if}
      </div>

      <!-- Cluster list -->
      <div class="flex-1 min-h-0 overflow-y-auto px-6 py-4 space-y-4">
        {#each [...selectedClusters.entries()] as [stem, clusterRepos]}
          {@const displayName = stem.startsWith('manual:') ? stem.slice(7) : stem}
          {@const isManual = stem.startsWith('manual:')}
          {@const stems = clusterRepos.map(r => nameStem(r.name))}
          <div class="rounded-xl border border-info-z3 bg-info-z1/20 overflow-hidden">
            <!-- Cluster header -->
            <div class="flex items-center gap-2 px-4 py-2.5 border-b border-info-z2/40 bg-info-z1/40">
              <span class="i-solar-layers-minimalistic-bold-duotone text-sm text-info-z6 shrink-0"></span>
              <div class="flex-1 min-w-0">
                <span class="text-sm font-semibold text-surface-z8">{displayName}</span>
                <span class="ml-2 text-xs text-surface-z4">
                  {clusterRepos.length} variants ·
                  {#if isManual}manually grouped
                  {:else if stems.every(s => s === stems[0])}name stem "{stems[0]}"
                  {:else}concept match{/if}
                </span>
              </div>
              <button onclick={() => disbandGroup(clusterRepos)}
                class="text-xs text-surface-z4 hover:text-error-z6 transition-colors shrink-0">
                Ungroup all
              </button>
            </div>
            <!-- Repos in cluster -->
            {#each clusterRepos as repo}
              {@const isNewest = clusterRepos.every(r => r.path === repo.path || (repo.last_commit_days ?? 9999) <= (r.last_commit_days ?? 9999))}
              {@const inMerge = mergeSelected.has(repo.path)}
              <div class="flex items-center group border-b border-info-z2/20 last:border-0">
                <!-- Merge-select checkbox -->
                <button onclick={() => toggleMerge(repo.path)}
                  class="shrink-0 ml-4 h-4 w-4 rounded border-2 flex items-center justify-center transition-colors
                         {inMerge ? 'border-info-z6 bg-info-z6' : 'border-surface-z3 bg-transparent'}">
                  {#if inMerge}<span class="text-[9px] font-bold text-white leading-none">✓</span>{/if}
                </button>
                <!-- Repo info -->
                <div class="flex flex-1 items-start gap-3 px-4 py-3 min-w-0">
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-1.5 flex-wrap">
                      <span class="text-sm font-medium text-surface-z8">{repo.name}</span>
                      {#if isNewest}<span class="text-[10px] rounded-full bg-success-z2 text-success-z7 px-1.5 py-0.5">newest</span>{/if}
                      <span class="text-[10px] rounded-full px-1.5 py-0.5 {STATUS_CLS[repo.status] ?? STATUS_CLS.unknown}">
                        {repo.status}{repo.last_commit_days != null ? ` · ${repo.last_commit_days}d` : ''}
                      </span>
                      {#each (repo.categories ?? []) as cat}
                        <span class="text-[10px] px-1.5 py-0.5 rounded-full {CAT_CLS[cat] ?? CAT_CLS.unknown}">{cat}</span>
                      {/each}
                    </div>
                    <p class="text-[10px] font-mono text-surface-z4 mt-0.5">{relPath(repo.path)}</p>
                  </div>
                </div>
                <!-- Remove from group button (hover) -->
                <button onclick={(e) => removeFromGroup(e, repo.path)}
                  title="Remove from group"
                  class="mr-4 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity rounded px-1.5 py-0.5 text-[10px] text-surface-z4 hover:bg-error-z2 hover:text-error-z6">
                  ✕ remove
                </button>
              </div>
            {/each}
          </div>
        {/each}

        <!-- Duplicates among selected repos — review before importing -->
        {#if selectedDuplicates.length > 0}
          <div class="rounded-xl border border-warning-z3 overflow-hidden">
            <div class="px-4 py-2.5 border-b border-warning-z2 bg-warning-z1/40">
              <span class="i-solar-copy-bold-duotone text-xs text-warning-z6 mr-1"></span>
              <span class="text-xs font-semibold text-warning-z7 uppercase tracking-wide">Likely duplicates — {selectedDuplicates.length}</span>
              <span class="ml-2 text-[10px] text-surface-z4">Consider deselecting these to avoid importing redundant copies</span>
            </div>
            {#each selectedDuplicates as repo}
              <div class="flex items-center gap-3 px-4 py-2.5 border-b border-warning-z2/30 last:border-0">
                <button onclick={() => { selectedRepos = new Set([...selectedRepos].filter(p => p !== repo.path)); }}
                  class="text-[10px] rounded-full bg-error-z2 text-error-z6 px-2 py-0.5 hover:bg-error-z3 transition-colors shrink-0">Deselect</button>
                <span class="text-sm text-surface-z7">{repo.name}</span>
                <span class="text-[10px] font-mono text-surface-z3">{relPath(repo.path)}</span>
                <span class="text-[11px] text-warning-z6">copy of {repo.duplicate_of!.split('/').pop()}</span>
              </div>
            {/each}
          </div>
        {/if}

        <!-- Ungrouped selected repos — for manual merging -->
        {#if ungroupedSelected.length > 0}
          <div class="rounded-xl border border-surface-z3 overflow-hidden">
            <div class="px-4 py-2.5 border-b border-surface-z2 bg-surface-z2/60">
              <span class="text-xs font-semibold text-surface-z5 uppercase tracking-wide">Standalone — {ungroupedSelected.length} projects</span>
              <span class="ml-2 text-[10px] text-surface-z4">Select 2+ and hit "Merge selected" to group them</span>
            </div>
            {#each ungroupedSelected as repo}
              {@const inMerge = mergeSelected.has(repo.path)}
              <div class="flex items-center border-b border-surface-z2/50 last:border-0">
                <button onclick={() => toggleMerge(repo.path)}
                  class="shrink-0 ml-4 h-4 w-4 rounded border-2 flex items-center justify-center transition-colors
                         {inMerge ? 'border-info-z6 bg-info-z6' : 'border-surface-z3 bg-transparent'}">
                  {#if inMerge}<span class="text-[9px] font-bold text-white leading-none">✓</span>{/if}
                </button>
                <div class="flex flex-1 items-center gap-2 px-4 py-2.5 min-w-0">
                  <span class="text-sm text-surface-z7">{repo.name}</span>
                  <span class="text-[10px] font-mono text-surface-z3">{relPath(repo.path)}</span>
                  {#each (repo.categories ?? []) as cat}
                    <span class="text-[10px] px-1.5 py-0.5 rounded-full {CAT_CLS[cat] ?? CAT_CLS.unknown}">{cat}</span>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center gap-3 border-t border-surface-z3 px-6 py-3 shrink-0">
        <button onclick={() => step = 'repos'} class="text-sm text-surface-z4 hover:text-surface-z6 transition-colors">← Back</button>
        <div class="flex-1"></div>
        <button onclick={startIndexing}
          class="rounded-xl bg-primary-z6 px-6 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-primary-z7">
          Import {selectedCount} project{selectedCount !== 1 ? 's' : ''} →
        </button>
      </div>
    </div>

  <!-- ══ DONE ══════════════════════════════════════════════════════════════════ -->
  {:else}
    <div class="no-drag flex flex-1 flex-col items-center justify-center gap-6 px-8 text-center">
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-success-z2 text-2xl">✓</div>
      <div class="max-w-xs">
        <h2 class="text-xl font-bold text-surface-z9">Ready to go</h2>
        <p class="mt-1.5 text-sm text-surface-z5">
          {#if selectedCount > 0}
            {selectedCount} project{selectedCount !== 1 ? 's' : ''} imported. Indexing happens in the background as you work.
          {:else}
            You can add projects any time from the Projects page.
          {/if}
        </p>
      </div>
      <button onclick={finish}
        class="w-full max-w-xs rounded-xl bg-primary-z6 px-6 py-3 text-sm font-semibold text-white transition-colors hover:bg-primary-z7">
        Open Sensei →
      </button>
    </div>
  {/if}

  <!-- Step dots (import + groups only) -->
  {#if step === 'acps' || step === 'folders' || step === 'repos' || step === 'groups'}
    <div class="absolute bottom-5 left-1/2 -translate-x-1/2 flex gap-1.5 pointer-events-none">
      {#each ['acps', 'folders', 'repos', 'groups'] as s}
        <div class="h-1.5 rounded-full transition-all {step === s ? 'w-4 bg-primary-z6' : 'w-1.5 bg-surface-z3'}"></div>
      {/each}
    </div>
  {/if}

</div>
