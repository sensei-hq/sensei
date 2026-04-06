<script lang="ts">
  import { Tabs, BreadCrumbs, Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  let activeTab = $state('overview');

  const tabOptions = [
    { value: 'overview', label: 'Overview' },
    { value: 'teams', label: 'Teams' },
    { value: 'members', label: 'Members' },
    { value: 'repos', label: 'Repos' },
  ];

  const breadcrumbs = $derived([
    { label: 'Orgs', href: '/orgs' },
    { label: data.org.name },
  ]);

  const memberColumns = [
    { name: 'name', label: 'Name', sortable: true },
    { name: 'email', label: 'Email', sortable: true },
    { name: 'role', label: 'Role', sortable: true },
    { name: 'teamsDisplay', label: 'Teams', sortable: false },
    { name: 'joinedDisplay', label: 'Joined', sortable: true },
  ];

  const memberRows = $derived(data.members.map(m => ({
    ...m,
    teamsDisplay: m.teams.join(', '),
    joinedDisplay: new Date(m.joinedAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' }),
  })));

  const repoColumns = [
    { name: 'name', label: 'Name', sortable: true },
    { name: 'provider', label: 'Provider', sortable: true },
    { name: 'remoteUrl', label: 'Remote URL', sortable: false },
    { name: 'lastIndexedDisplay', label: 'Last Indexed', sortable: true },
  ];

  const repoRows = $derived(data.repos.map(r => ({
    ...r,
    lastIndexedDisplay: r.lastIndexedAt
      ? new Date(r.lastIndexedAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
      : 'Never',
  })));

  const planBadgeClass = (plan: string) => {
    if (plan === 'team') return 'badge-plan badge-plan-team';
    if (plan === 'enterprise') return 'badge-plan badge-plan-enterprise';
    return 'badge-plan badge-plan-free';
  };
</script>

<div class="mb-4">
  <BreadCrumbs items={breadcrumbs} />
</div>

<div class="flex items-start justify-between gap-4 mb-6">
  <div>
    <h1 class="text-2xl font-semibold text-surface-z8 mb-1">{data.org.name}</h1>
    <p class="text-sm text-surface-z5">{data.org.description}</p>
  </div>
  <span class={planBadgeClass(data.org.plan)}>{data.org.plan}</span>
</div>

<Tabs options={tabOptions} bind:value={activeTab} />

<div class="mt-6">
  {#if activeTab === 'overview'}
    <!-- Stat cards -->
    <div class="grid gap-3 mb-8" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); max-width: 520px">
      <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
        <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.stats.totalSessions}</div>
        <div class="text-xs uppercase tracking-wider text-surface-z5">Sessions</div>
      </div>
      <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
        <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">${data.stats.totalCostUsd.toFixed(2)}</div>
        <div class="text-xs uppercase tracking-wider text-surface-z5">Total Cost</div>
      </div>
      <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
        <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{(data.stats.avgFtrScore * 100).toFixed(0)}%</div>
        <div class="text-xs uppercase tracking-wider text-surface-z5">Avg FTR</div>
      </div>
    </div>

    <div class="mb-6">
      <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">Details</h2>
      <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 max-w-md">
        <div class="flex flex-col gap-2 text-sm">
          <div class="flex justify-between">
            <span class="text-surface-z5">Slug</span>
            <span class="font-mono text-surface-z7">{data.org.slug}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-surface-z5">Plan</span>
            <span class={planBadgeClass(data.org.plan)}>{data.org.plan}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-surface-z5">Created</span>
            <span class="text-surface-z7">{new Date(data.org.createdAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-surface-z5">Members</span>
            <span class="text-surface-z7">{data.members.length}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-surface-z5">Teams</span>
            <span class="text-surface-z7">{data.teams.length}</span>
          </div>
        </div>
      </div>
    </div>

  {:else if activeTab === 'teams'}
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Teams ({data.teams.length})</h2>
      <button class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z5 cursor-not-allowed opacity-60" disabled>
        Create Team
      </button>
    </div>
    <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(240px, 1fr))">
      {#each data.teams as team}
        <a
          href="/orgs/{data.org.id}/teams/{team.id}"
          class="flex flex-col p-4 rounded-lg border border-surface-z3 bg-surface-z1 no-underline transition-colors hover:border-primary-z5 hover:bg-surface-z2"
        >
          <div class="font-semibold text-surface-z8 mb-1">{team.name}</div>
          <p class="text-xs text-surface-z5 mb-3 leading-snug">{team.description}</p>
          <div class="flex gap-3 text-xs mt-auto">
            <span class="text-surface-z4">members <span class="font-medium text-surface-z7">{team.memberCount}</span></span>
            <span class="text-surface-z3">·</span>
            <span class="text-surface-z4">repos <span class="font-medium text-surface-z7">{team.repoCount}</span></span>
          </div>
        </a>
      {/each}
    </div>

  {:else if activeTab === 'members'}
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">Members ({data.members.length})</h2>
    <Table data={memberRows} columns={memberColumns} />

  {:else if activeTab === 'repos'}
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Repos ({data.repos.length})</h2>
      <button class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z5 cursor-not-allowed opacity-60" disabled>
        Add Repo
      </button>
    </div>
    <Table data={repoRows} columns={repoColumns} />
  {/if}
</div>

<style>
  .badge-plan {
    padding: 2px 8px;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: capitalize;
    flex-shrink: 0;
    display: inline-block;
  }
  .badge-plan-free { background: #f1f5f9; color: #475569; }
  .badge-plan-team { background: #dbeafe; color: #1d4ed8; }
  .badge-plan-enterprise { background: #ede9fe; color: #5b21b6; }
</style>
