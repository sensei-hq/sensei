<script lang="ts">
  import { List } from '@rokkit/ui';
  import { ThemeSwitcherToggle } from '@rokkit/app';
  import { vibe } from '@rokkit/states';
  import { themable } from '@rokkit/actions';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  // ── navigation state ────────────────────────────────────────────
  type Rail    = 'repos' | 'libraries' | 'analytics' | 'settings';
  type RepoTab = 'sessions' | 'analytics' | 'context' | 'agents' | 'drift';

  let rail         = $state<Rail>('repos');
  let wsId         = $state('personal');
  let selectedId   = $state<string | null>(data.repos[0]?.id ?? null);
  let repoTab      = $state<RepoTab>('sessions');
  let wsOpen       = $state(false);
  let userMenuOpen = $state(false);

  // ── helpers ───────────────────────────────────────────────────────
  function timeAgo(iso: string | null): string {
    if (!iso) return 'never';
    const diff = Date.now() - new Date(iso).getTime();
    const minutes = Math.floor(diff / 60000);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    return `${Math.floor(hours / 24)}d ago`;
  }

  function repoLang(stack: string[]): string {
    return stack[0] ?? '';
  }

  function repoStatus(lastIndexedAt: string | null): 'ready' | 'never' {
    return lastIndexedAt ? 'ready' : 'never';
  }

  // ── workspace + filtered data ────────────────────────────────────
  const workspace  = $derived(data.workspaces.find(w => w.id === wsId) ?? data.workspaces[0]!);
  const wsRepos    = $derived(data.repos); // all repos belong to personal workspace
  const activeRepo = $derived(data.repos.find(r => r.id === selectedId) ?? null);
  const activeLib  = $derived(data.libraries.find(l => l.id === selectedId) ?? null);

  const repoSessions  = $derived(data.taskSessions.filter(s => s.repoId === selectedId));
  const allWsSessions = $derived(data.taskSessions);

  // ── List items ────────────────────────────────────────────────────
  const repoListItems = $derived(
    wsRepos.map(r => ({
      label:        r.name,
      value:        r.id,
      lang:         repoLang(r.stack),
      sessionCount: r.sessionCount,
      indexed:      timeAgo(r.lastIndexedAt),
      status:       repoStatus(r.lastIndexedAt),
    }))
  );

  const libListItems = $derived(
    data.libraries.map(l => ({
      label:      l.name,
      value:      l.id,
      sourceType: l.sourceType,
      sections:   l.sectionCount,
      status:     l.indexStatus,
    }))
  );

  const sessionListItems = $derived(
    allWsSessions.slice(0, 20).map(s => {
      const repo = data.repos.find(r => r.id === s.repoId);
      return {
        label:    s.taskDescription,
        value:    s.id,
        repoName: repo?.name ?? '',
        when:     timeAgo(s.createdAt),
        ftr:      s.ftrScore ?? 0,
        repoId:   s.repoId,
        hasFtr:   s.ftrScore !== null,
      };
    })
  );

  const settingsListItems = [
    { label: 'Profile',       value: 'profile',       desc: 'Name, email, avatar' },
    { label: 'Workspaces',    value: 'workspaces',    desc: 'Orgs and teams' },
    { label: 'API Token',     value: 'api-token',     desc: 'Supabase credentials' },
    { label: 'Daemon',        value: 'daemon',        desc: 'Collector status' },
    { label: 'Notifications', value: 'notifications', desc: 'Alerts and emails' },
  ];

  function switchRail(r: Rail) {
    rail = r;
    if (r === 'repos')      selectedId = wsRepos[0]?.id ?? null;
    else if (r === 'libraries') selectedId = data.libraries[0]?.id ?? null;
    else                    selectedId = null;
    repoTab = 'sessions';
  }

  // ── FTR helpers ───────────────────────────────────────────────────
  function ftrBadge(ftr: number) {
    if (ftr >= 0.95) return 'bg-success-z1 text-success-z7 border-success-z3';
    if (ftr >= 0.8)  return 'bg-warning-z1 text-warning-z7 border-warning-z3';
    return 'bg-error-z1 text-error-z7 border-error-z3';
  }
  function ftrColor(ftr: number) {
    if (ftr >= 0.95) return 'text-success-z6';
    if (ftr >= 0.8)  return 'text-warning-z6';
    return 'text-error-z6';
  }

  const avgFtr = $derived(
    repoSessions.filter(s => s.ftrScore !== null).length
      ? repoSessions.filter(s => s.ftrScore !== null).reduce((a, s) => a + (s.ftrScore ?? 0), 0)
        / repoSessions.filter(s => s.ftrScore !== null).length
      : 0
  );

  // SVG paths for rail icons
  const railItems: Array<{ key: Rail; label: string; d: string }> = [
    {
      key: 'repos', label: 'Repos',
      d: 'M3 7a2 2 0 012-2h4.172a2 2 0 011.414.586L12 7h7a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V7z',
    },
    {
      key: 'libraries', label: 'Libraries',
      d: 'M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253',
    },
    {
      key: 'analytics', label: 'Analytics',
      d: 'M9 19V6m6 13V3m-3 16V10',
    },
    {
      key: 'settings', label: 'Settings',
      d: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065zM15 12a3 3 0 11-6 0 3 3 0 016 0z',
    },
  ];

  const repoTabs: Array<{ key: RepoTab; label: string }> = [
    { key: 'sessions',  label: 'Sessions'      },
    { key: 'analytics', label: 'Analytics'     },
    { key: 'context',   label: 'Context Packs' },
    { key: 'agents',    label: 'Agents'        },
    { key: 'drift',     label: 'Drift'         },
  ];
</script>

<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

<!-- Full-screen layout -->
<div class="flex h-screen w-screen overflow-hidden bg-surface-z1 text-surface-z8">

  <!-- ══ ICON RAIL ════════════════════════════════════════════════ -->
  <nav class="relative flex w-16 shrink-0 flex-col items-center border-r border-surface-z3 bg-surface-z2 pt-4 pb-3 gap-1">

    <!-- Logo mark -->
    <div class="mb-3 flex h-8 w-8 items-center justify-center rounded-lg bg-primary-z6">
      <span class="text-sm font-bold text-white tracking-tight">s</span>
    </div>

    {#each railItems as item}
      <button
        class="flex w-12 flex-col items-center gap-1.5 rounded-xl py-2.5 transition-colors
               {rail === item.key
                 ? 'bg-primary-z2 text-primary-z6'
                 : 'text-surface-z5 hover:bg-surface-z3 hover:text-surface-z7'}"
        onclick={() => switchRail(item.key)}
        title={item.label}
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"
             stroke-linecap="round" stroke-linejoin="round" class="h-5 w-5 shrink-0">
          <path d={item.d}/>
        </svg>
        <span class="text-[9px] font-medium leading-none">{item.label}</span>
      </button>
    {/each}

    <div class="flex-1"></div>

    <!-- User avatar — opens popover menu -->
    <div class="relative">
      <button
        class="flex h-8 w-8 items-center justify-center rounded-full bg-primary-z6 text-xs font-bold text-white hover:ring-2 hover:ring-primary-z4 transition-all"
        onclick={() => userMenuOpen = !userMenuOpen}
        title={data.user.name}
      >
        {data.user.initials}
      </button>

      {#if userMenuOpen}
        <div
          class="fixed inset-0 z-40"
          role="presentation"
          onclick={() => userMenuOpen = false}
        ></div>

        <div class="absolute bottom-0 left-full z-50 ml-2 w-56 rounded-xl border border-surface-z3 bg-surface-z2 shadow-xl overflow-hidden">
          <div class="px-4 py-3 border-b border-surface-z3">
            <p class="text-sm font-semibold text-surface-z8 truncate">{data.user.name}</p>
            <p class="text-xs text-surface-z5 truncate">{data.user.email}</p>
          </div>

          <div class="py-1">
            <button
              class="flex w-full items-center gap-3 px-4 py-2.5 text-sm text-surface-z7 hover:bg-surface-z3 transition-colors"
              onclick={() => { switchRail('settings'); userMenuOpen = false; }}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4 text-surface-z5">
                <path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065zM15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
              </svg>
              Settings
            </button>

            <div class="flex items-center justify-between px-4 py-2.5">
              <span class="flex items-center gap-3 text-sm text-surface-z7">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4 text-surface-z5">
                  <path d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364-6.364l-.707.707M6.343 17.657l-.707.707m12.728 0l-.707-.707M6.343 6.343l-.707-.707M12 8a4 4 0 100 8 4 4 0 000-8z"/>
                </svg>
                Theme
              </span>
              <ThemeSwitcherToggle />
            </div>

            <div class="my-1 border-t border-surface-z3"></div>

            <button
              class="flex w-full items-center gap-3 px-4 py-2.5 text-sm text-surface-z6 hover:bg-surface-z3 transition-colors"
              onclick={() => userMenuOpen = false}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4 text-surface-z5">
                <path d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
              </svg>
              Sign out
            </button>
          </div>
        </div>
      {/if}
    </div>
  </nav>

  <!-- ══ LIST PANE ════════════════════════════════════════════════ -->
  <aside class="flex w-72 shrink-0 flex-col border-r border-surface-z3 bg-surface-z2">

    <!-- Workspace switcher (repos + analytics views) -->
    {#if rail === 'repos' || rail === 'analytics'}
      <div class="relative shrink-0 border-b border-surface-z3">
        <button
          class="flex w-full items-center gap-2.5 px-4 py-3 hover:bg-surface-z3 transition-colors"
          onclick={() => wsOpen = !wsOpen}
        >
          <span class="flex-1 min-w-0 truncate text-left text-sm font-semibold text-surface-z8">{workspace.name}</span>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"
               class="h-3.5 w-3.5 shrink-0 text-surface-z4 transition-transform {wsOpen ? 'rotate-180' : ''}">
            <path d="M4 6l4 4 4-4"/>
          </svg>
        </button>

        {#if wsOpen}
          <div class="absolute inset-x-0 top-full z-50 border-b border-surface-z3 bg-surface-z2 shadow-xl">
            {#each data.workspaces as ws}
              <button
                class="flex w-full items-center gap-2.5 px-4 py-2 hover:bg-surface-z3 transition-colors"
                onclick={() => { wsId = ws.id; wsOpen = false; }}
              >
                <span class="flex-1 min-w-0 truncate text-sm text-surface-z7">{ws.name}</span>
                <span class="shrink-0 text-xs capitalize text-surface-z5">{ws.type}</span>
                {#if ws.id === wsId}
                  <svg viewBox="0 0 16 16" fill="currentColor" class="h-3.5 w-3.5 shrink-0 text-primary-z6">
                    <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/>
                  </svg>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div class="shrink-0 border-b border-surface-z3 px-4 py-3">
        <span class="text-sm font-semibold text-surface-z8 capitalize">{rail}</span>
      </div>
    {/if}

    <!-- List body -->
    <div class="flex-1 overflow-y-auto">

      <!-- ── Repos ─────────────────────────────────────────────── -->
      {#if rail === 'repos'}
        {#if wsRepos.length === 0}
          <p class="px-4 py-6 text-sm text-surface-z4">No repos indexed yet.</p>
        {:else}
          <List
            items={repoListItems}
            value={selectedId}
            onselect={(v) => { selectedId = v as string; repoTab = 'sessions'; }}
            class="py-1"
          >
            {#snippet itemContent(proxy)}
              <div class="flex w-full items-center gap-2.5 py-0.5">
                <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded bg-surface-z3 text-xs font-bold text-surface-z7">
                  {(proxy.label as string).charAt(0).toUpperCase()}
                </div>
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-1.5">
                    <span class="text-sm font-medium text-surface-z8 truncate">{proxy.label}</span>
                  </div>
                  <div class="flex items-center gap-1.5 text-xs text-surface-z5">
                    {#if proxy.get('lang')}<span>{proxy.get('lang')}</span><span class="text-surface-z3">·</span>{/if}
                    <span>{proxy.get('sessionCount')} sessions</span>
                  </div>
                </div>
                <span class="shrink-0 text-xs text-surface-z4">{proxy.get('indexed')}</span>
              </div>
            {/snippet}
          </List>
        {/if}

      <!-- ── Libraries ──────────────────────────────────────────── -->
      {:else if rail === 'libraries'}
        {#if data.libraries.length === 0}
          <p class="px-4 py-6 text-sm text-surface-z4">No libraries indexed yet.</p>
        {:else}
          <List
            items={libListItems}
            value={selectedId}
            onselect={(v) => { selectedId = v as string; }}
            class="py-1"
          >
            {#snippet itemContent(proxy)}
              <div class="flex w-full items-center gap-2.5 py-0.5">
                <span class="mt-1 h-2 w-2 shrink-0 rounded-full
                  {proxy.get('status') === 'indexed' ? 'bg-success-z5'
                    : proxy.get('status') === 'pending' ? 'bg-warning-z5'
                    : 'bg-surface-z4'}">
                </span>
                <div class="min-w-0 flex-1">
                  <p class="text-sm font-medium text-surface-z8">{proxy.label}</p>
                  <p class="text-xs text-surface-z5">{proxy.get('sections')} sections · {proxy.get('sourceType')}</p>
                </div>
                <span class="shrink-0 rounded bg-surface-z3 px-1.5 py-0.5 text-xs capitalize text-surface-z6">
                  {proxy.get('status')}
                </span>
              </div>
            {/snippet}
          </List>
        {/if}

      <!-- ── Analytics (recent sessions list) ──────────────────── -->
      {:else if rail === 'analytics'}
        {#if allWsSessions.length === 0}
          <p class="px-4 py-6 text-sm text-surface-z4">No sessions in the last 30 days.</p>
        {:else}
          <List
            items={sessionListItems}
            class="py-1"
            onselect={(v, proxy) => {
              rail = 'repos';
              selectedId = proxy.get('repoId') as string;
              repoTab = 'sessions';
            }}
          >
            {#snippet itemContent(proxy)}
              <div class="flex w-full items-start gap-2 py-0.5">
                <span class="mt-0.5 shrink-0 rounded bg-surface-z3 px-1.5 py-0.5 font-mono text-xs text-surface-z6">
                  {proxy.get('repoName')}
                </span>
                <div class="min-w-0 flex-1">
                  <p class="text-xs font-medium text-surface-z8 truncate">{proxy.label}</p>
                  <p class="text-xs text-surface-z5">{proxy.get('when')}</p>
                </div>
                {#if proxy.get('hasFtr')}
                  <span class="shrink-0 inline-flex items-center rounded-full border px-1.5 py-0.5 text-xs font-semibold {ftrBadge(proxy.get('ftr') as number)}">
                    {Math.round((proxy.get('ftr') as number) * 100)}%
                  </span>
                {/if}
              </div>
            {/snippet}
          </List>
        {/if}

      <!-- ── Settings ───────────────────────────────────────────── -->
      {:else if rail === 'settings'}
        <List
          items={settingsListItems}
          class="py-1"
        >
          {#snippet itemContent(proxy)}
            <div class="flex w-full items-center gap-3 py-0.5">
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium text-surface-z8">{proxy.label}</p>
                <p class="text-xs text-surface-z5">{proxy.get('desc')}</p>
              </div>
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5 shrink-0 text-surface-z4">
                <path d="M6 4l4 4-4 4"/>
              </svg>
            </div>
          {/snippet}
        </List>
      {/if}
    </div>
  </aside>

  <!-- ══ DETAIL PANE ═══════════════════════════════════════════════ -->
  <div class="flex flex-1 flex-col min-w-0 overflow-hidden">

    <!-- ── Repo detail ─────────────────────────────────────────── -->
    {#if rail === 'repos' && activeRepo}
      <div class="shrink-0 border-b border-surface-z3 px-6 pt-5 pb-0">
        <div class="flex items-start justify-between mb-4">
          <div>
            <h1 class="text-xl font-semibold text-surface-z8">{activeRepo.name}</h1>
            <div class="mt-1 flex items-center gap-3 text-xs text-surface-z5">
              {#if activeRepo.stack.length > 0}
                <span>{activeRepo.stack[0]}</span>
                <span class="text-surface-z3">·</span>
              {/if}
              <span>indexed {timeAgo(activeRepo.lastIndexedAt)}</span>
            </div>
          </div>
          <button class="rounded-md border border-surface-z3 px-3 py-1.5 text-xs text-surface-z6 hover:border-primary-z5 hover:text-surface-z8 transition-colors">
            Re-index
          </button>
        </div>

        <div class="flex">
          {#each repoTabs as tab}
            <button
              class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px
                     {repoTab === tab.key
                       ? 'border-primary-z6 text-primary-z6'
                       : 'border-transparent text-surface-z5 hover:text-surface-z7'}"
              onclick={() => repoTab = tab.key}
            >
              {tab.label}
            </button>
          {/each}
        </div>
      </div>

      <div class="flex-1 overflow-auto p-6">

        {#if repoTab === 'sessions'}
          <div class="space-y-5">
            <div class="grid grid-cols-3 gap-3">
              {#each [
                { label: 'Sessions (30d)', value: repoSessions.length, ftr: null },
                { label: 'Avg FTR',        value: repoSessions.filter(s => s.ftrScore !== null).length
                                                    ? Math.round(avgFtr * 100) + '%'
                                                    : '—',                         ftr: avgFtr },
                { label: 'Stack',          value: activeRepo.stack[0] ?? '—',     ftr: null },
              ] as stat}
                <div class="rounded-lg border border-surface-z3 bg-surface-z2 p-4">
                  <div class="text-2xl font-semibold {stat.ftr != null && stat.ftr > 0 ? ftrColor(stat.ftr) : 'text-surface-z8'}">
                    {stat.value}
                  </div>
                  <div class="mt-0.5 text-xs text-surface-z5">{stat.label}</div>
                </div>
              {/each}
            </div>

            {#if repoSessions.length === 0}
              <div class="flex items-center justify-center rounded-lg border border-surface-z3 bg-surface-z2 p-10">
                <p class="text-sm text-surface-z4">No sessions in the last 30 days.</p>
              </div>
            {:else}
              <div class="overflow-hidden rounded-lg border border-surface-z3">
                <table class="w-full text-sm">
                  <thead>
                    <tr class="border-b border-surface-z3 bg-surface-z2">
                      <th class="px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">Task</th>
                      <th class="w-20 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">FTR</th>
                      <th class="w-24 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">Status</th>
                      <th class="w-24 px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-surface-z5">When</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each repoSessions as s}
                      <tr class="border-b border-surface-z2 last:border-0 hover:bg-surface-z2 transition-colors">
                        <td class="px-4 py-3">
                          <span class="font-medium text-surface-z8">{s.taskDescription}</span>
                        </td>
                        <td class="px-4 py-3">
                          {#if s.ftrScore !== null}
                            <span class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-semibold {ftrBadge(s.ftrScore)}">
                              {Math.round(s.ftrScore * 100)}%
                            </span>
                          {:else}
                            <span class="text-xs text-surface-z4">—</span>
                          {/if}
                        </td>
                        <td class="px-4 py-3">
                          <span class="text-xs capitalize text-surface-z6">{s.status}</span>
                        </td>
                        <td class="px-4 py-3 text-xs text-surface-z4">{timeAgo(s.createdAt)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
          </div>

        {:else if repoTab === 'analytics'}
          <div class="space-y-5">
            <div class="grid grid-cols-3 gap-4">
              {#each [
                { label: 'Sessions (30d)', value: repoSessions.length,                                         sub: 'task sessions' },
                { label: 'Avg FTR',        value: repoSessions.filter(s=>s.ftrScore!==null).length ? Math.round(avgFtr*100)+'%' : '—', sub: 'first-try-right' },
                { label: 'Stack',          value: activeRepo.stack[0] ?? '—',                                  sub: 'primary language' },
              ] as stat}
                <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
                  <div class="mb-1 text-xs text-surface-z5">{stat.label}</div>
                  <div class="text-3xl font-semibold text-surface-z8">{stat.value}</div>
                  <div class="mt-1 text-xs text-surface-z4">{stat.sub}</div>
                </div>
              {/each}
            </div>
          </div>

        {:else if repoTab === 'context'}
          <div class="flex items-center justify-center rounded-lg border border-surface-z3 bg-surface-z2 p-12">
            <p class="text-sm text-surface-z5">Context packs coming soon.</p>
          </div>

        {:else if repoTab === 'agents'}
          <div class="flex items-center justify-center rounded-lg border border-surface-z3 bg-surface-z2 p-12">
            <p class="text-sm text-surface-z5">Agents coming soon.</p>
          </div>

        {:else if repoTab === 'drift'}
          <div class="flex items-center justify-center rounded-lg border border-surface-z3 bg-surface-z2 p-12">
            <p class="text-sm text-surface-z5">No drift detected — all docs are in sync.</p>
          </div>
        {/if}

      </div>

    <!-- ── Library detail ──────────────────────────────────────── -->
    {:else if rail === 'libraries' && activeLib}
      <div class="shrink-0 border-b border-surface-z3 px-6 py-5">
        <div class="flex items-start justify-between">
          <div>
            <h1 class="text-xl font-semibold text-surface-z8">{activeLib.name}</h1>
            <div class="mt-1 flex items-center gap-3 text-xs text-surface-z5">
              <span class="capitalize">{activeLib.sourceType}</span>
              <span class="text-surface-z3">·</span>
              <span>{activeLib.sectionCount} sections</span>
              <span class="text-surface-z3">·</span>
              <span class="capitalize">{activeLib.indexStatus}</span>
            </div>
          </div>
          <div class="flex gap-2">
            <button class="rounded-md border border-surface-z3 px-3 py-1.5 text-xs text-surface-z6 hover:border-primary-z5 transition-colors">Re-index</button>
          </div>
        </div>
      </div>
      <div class="flex-1 overflow-auto p-6">
        <p class="text-sm text-surface-z5">Documents and sections for {activeLib.name} will appear here.</p>
      </div>

    <!-- ── Workspace analytics ─────────────────────────────────── -->
    {:else if rail === 'analytics'}
      <div class="shrink-0 border-b border-surface-z3 px-6 py-5">
        <h1 class="text-xl font-semibold text-surface-z8">Analytics</h1>
        <p class="mt-1 text-xs text-surface-z5">{workspace.name} · last 30 days</p>
      </div>
      <div class="flex-1 overflow-auto p-6 space-y-5">
        <div class="grid grid-cols-3 gap-3">
          {#each [
            { label: 'Sessions',  value: allWsSessions.length },
            { label: 'Avg FTR',   value: allWsSessions.filter(s=>s.ftrScore!==null).length
                                          ? Math.round(allWsSessions.filter(s=>s.ftrScore!==null).reduce((a,s)=>a+(s.ftrScore??0),0)/allWsSessions.filter(s=>s.ftrScore!==null).length*100)+'%'
                                          : '—' },
            { label: 'Repos',     value: data.repos.length },
          ] as stat}
            <div class="rounded-lg border border-surface-z3 bg-surface-z2 p-4">
              <div class="text-2xl font-semibold text-surface-z8">{stat.value}</div>
              <div class="mt-0.5 text-xs text-surface-z5">{stat.label}</div>
            </div>
          {/each}
        </div>
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-5">
          <h3 class="mb-4 text-sm font-semibold text-surface-z7">Sessions by repo</h3>
          <div class="space-y-2.5">
            {#each wsRepos as repo}
              {@const count = data.taskSessions.filter(s => s.repoId === repo.id).length}
              {@const pct   = Math.round(count / Math.max(allWsSessions.length, 1) * 100)}
              <div class="flex items-center gap-3">
                <span class="w-28 shrink-0 truncate text-xs text-surface-z6">{repo.name}</span>
                <div class="h-2 flex-1 overflow-hidden rounded-full bg-surface-z3">
                  <div class="h-full rounded-full bg-primary-z5 transition-all" style="width:{pct}%"></div>
                </div>
                <span class="w-16 shrink-0 text-right text-xs text-surface-z4">{count} sessions</span>
              </div>
            {/each}
          </div>
        </div>
      </div>

    <!-- ── Settings detail ─────────────────────────────────────── -->
    {:else if rail === 'settings'}
      <div class="shrink-0 border-b border-surface-z3 px-6 py-5">
        <h1 class="text-xl font-semibold text-surface-z8">Settings</h1>
        <p class="mt-1 text-xs text-surface-z5">{data.user.name} · {data.user.email}</p>
      </div>
      <div class="flex-1 overflow-auto p-6 max-w-lg">
        <p class="text-sm text-surface-z5">Select a section from the list.</p>
      </div>

    {:else}
      <div class="flex flex-1 items-center justify-center">
        <p class="text-sm text-surface-z4">Select an item from the list</p>
      </div>
    {/if}

  </div>
</div>
