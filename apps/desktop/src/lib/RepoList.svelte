<script lang="ts">

  export type Repo = {
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

  const STATUS_CLS: Record<string, string> = {
    active:    'bg-success-z2 text-success-z7',
    recent:    'bg-primary-z2 text-primary-z7',
    stale:     'bg-warning-z2 text-warning-z7',
    archived:  'bg-surface-z3 text-surface-z5',
    abandoned: 'bg-error-z2 text-error-z7',
    unknown:   'bg-surface-z3 text-surface-z5',
  };

  const CAT_CLS: Record<string, string> = {
    app:     'bg-primary-z2 text-primary-z6',
    library: 'bg-info-z2 text-info-z6',
    tool:    'bg-warning-z2 text-warning-z7',
    idea:    'bg-surface-z3 text-surface-z5',
    unknown: 'bg-surface-z2 text-surface-z4',
  };

  const STATUS_PRIORITY: Record<string, number> = {
    active: 0, recent: 1, stale: 2, archived: 3, unknown: 4, abandoned: 5,
  };

  let {
    repos,
    selected = $bindable(new Set<string>()),
    clientTags = $bindable(new Map<string, string>()),
  }: {
    repos: Repo[];
    selected?: Set<string>;
    clientTags?: Map<string, string>;
  } = $props();

  let search = $state('');
  let editingTag = $state<string | null>(null);
  let tagInput = $state('');

  const filtered = $derived.by(() => {
    const q = search.toLowerCase();
    return [...repos]
      .sort((a, b) => {
        const d = (STATUS_PRIORITY[a.status] ?? 4) - (STATUS_PRIORITY[b.status] ?? 4);
        return d !== 0 ? d : a.name.localeCompare(b.name);
      })
      .filter(r => !q ||
        r.name.toLowerCase().includes(q) ||
        r.path.toLowerCase().includes(q) ||
        (r.description ?? '').toLowerCase().includes(q)
      );
  });

  function relPath(p: string) {
    const parts = p.split('/');
    return parts.length >= 2 ? parts.slice(-2).join('/') : p;
  }

  function toggle(path: string) {
    const s = new Set(selected);
    s.has(path) ? s.delete(path) : s.add(path);
    selected = s;
  }

  function selectActive() {
    selected = new Set(repos.filter(r => !r.duplicate_of && (r.status === 'active' || r.status === 'recent')).map(r => r.path));
  }

  function selectHealthy() {
    selected = new Set(repos.filter(r => !r.duplicate_of).map(r => r.path));
  }

  function setClientTag(path: string, tag: string) {
    const m = new Map(clientTags);
    if (tag.trim()) m.set(path, tag.trim()); else m.delete(path);
    clientTags = m;
    editingTag = null;
  }
</script>

<!-- Quick selectors -->
<div class="flex items-center justify-between text-xs text-surface-z5">
  <span>{repos.length} repos · {selected.size} selected</span>
  <div class="flex gap-3">
    <button onclick={selectActive} class="text-primary-z6 hover:text-primary-z7">Active &amp; recent</button>
    <button onclick={selectHealthy} class="text-surface-z5 hover:text-surface-z7">All healthy</button>
    <button onclick={() => selected = new Set()} class="text-surface-z4 hover:text-surface-z6">Clear</button>
  </div>
</div>

<!-- Search -->
<div class="relative">
  <span class="absolute left-3 top-1/2 -translate-y-1/2 i-solar-magnifer-bold-duotone text-xs text-surface-z4 pointer-events-none"></span>
  <input
    bind:value={search}
    placeholder="Filter by name, path, or description…"
    class="w-full rounded-lg border border-surface-z3 bg-surface-z2 py-1.5 pl-8 pr-3 text-sm text-surface-z7 outline-none placeholder:text-surface-z3 focus:border-primary-z4"
  />
</div>

<!-- Repo list -->
<div class="flex-1 min-h-0 overflow-y-auto rounded-xl border border-surface-z2 divide-y divide-surface-z2">
  {#if filtered.length === 0}
    <div class="flex flex-col items-center gap-2 py-10 text-center text-sm text-surface-z4">
      <span class="i-solar-folder-open-bold-duotone text-2xl text-surface-z3"></span>
      <p>{repos.length === 0 ? 'No repos found' : 'No matches'}</p>
    </div>
  {:else}
    {#each filtered as repo}
      {@const sel = selected.has(repo.path)}
      {@const tag = clientTags.get(repo.path)}
      {@const editing = editingTag === repo.path}
      <div class="flex w-full items-center group hover:bg-surface-z2/50 transition-colors {repo.duplicate_of ? 'opacity-55' : ''}">
        <button onclick={() => toggle(repo.path)} class="flex flex-1 min-w-0 items-start gap-3 px-4 py-3 text-left">
          <div class="mt-0.5 shrink-0 h-4 w-4 rounded border-2 flex items-center justify-center
                      {sel ? 'border-primary-z6 bg-primary-z6' : 'border-surface-z4 bg-transparent'}">
            {#if sel}<span class="text-[9px] font-bold text-white leading-none">✓</span>{/if}
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-1.5 flex-wrap">
              <span class="text-sm font-medium text-surface-z8">{repo.name}</span>
              {#each (repo.categories ?? []).filter(c => c !== 'unknown') as cat}
                <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium {CAT_CLS[cat] ?? 'bg-surface-z3 text-surface-z5'}">{cat}</span>
              {/each}
              <span class="text-[10px] rounded-full px-1.5 py-0.5 {STATUS_CLS[repo.status] ?? STATUS_CLS.unknown}">
                {repo.status}{repo.last_commit_days != null ? ` · ${repo.last_commit_days}d` : ''}
              </span>
              {#each repo.tech_stack.slice(0, 2) as tech}
                <span class="text-[10px] bg-surface-z3 text-surface-z5 px-1.5 py-0.5 rounded">{tech}</span>
              {/each}
            </div>
            <p class="text-[10px] font-mono text-surface-z3 mt-0.5">{relPath(repo.path)}</p>
            {#if repo.duplicate_of}
              <p class="text-[11px] text-warning-z6 mt-0.5">Likely copy of {repo.duplicate_of.split('/').pop()}</p>
            {:else if repo.description}
              <p class="text-xs text-surface-z4 mt-0.5 line-clamp-1">{repo.description}</p>
            {/if}
          </div>
        </button>
        <div class="shrink-0 pr-4">
          {#if editing}
            <input
              bind:value={tagInput}
              onclick={(e) => e.stopPropagation()}
              onblur={() => setClientTag(repo.path, tagInput)}
              onkeydown={(e) => {
                if (e.key === 'Enter') setClientTag(repo.path, tagInput);
                if (e.key === 'Escape') editingTag = null;
              }}
              placeholder="client"
              class="w-20 rounded-lg border border-primary-z4 bg-surface-z1 px-2 py-0.5 text-[11px] text-surface-z7 outline-none font-mono"
            />
          {:else if tag}
            <button
              onclick={(e) => { e.stopPropagation(); editingTag = repo.path; tagInput = tag; }}
              class="flex items-center gap-1 rounded-full bg-surface-z3 px-2 py-0.5 text-[10px] text-surface-z6 hover:bg-surface-z4 transition-colors">
              <span class="i-solar-tag-bold-duotone text-xs"></span>{tag}
            </button>
          {:else}
            <button
              onclick={(e) => { e.stopPropagation(); editingTag = repo.path; tagInput = ''; }}
              class="opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1 rounded-full border border-surface-z3 px-2 py-0.5 text-[10px] text-surface-z4 hover:border-primary-z4 hover:text-primary-z6">
              <span class="i-solar-tag-bold-duotone text-xs"></span>tag
            </button>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>
