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
    { name: 'role', label: 'Role', sortable: true },
    { name: 'joinedDisplay', label: 'Joined', sortable: true },
  ];

  const memberRows = $derived((data.members as { id: string; name: string; slug: string; avatarUrl: string | null; role: string; joinedAt: string }[]).map(m => ({
    ...m,
    joinedDisplay: m.joinedAt
      ? new Date(m.joinedAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
      : '—',
  })));

  const repoColumns = [
    { name: 'name', label: 'Name', sortable: true },
    { name: 'remoteUrl', label: 'Remote URL', sortable: false },
    { name: 'lastIndexedDisplay', label: 'Last Indexed', sortable: true },
  ];

  const repoRows = $derived((data.repos as { id: string; name: string; remoteUrl: string | null; stack: string[] | null; lastIndexedAt: string | null }[]).map(r => ({
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
    <p class="text-sm text-surface-z5 font-mono">{data.org.slug}</p>
  </div>
  <span class={planBadgeClass(data.org.plan ?? '')}>{data.org.plan}</span>
</div>

<Tabs options={tabOptions} bind:value={activeTab} />

<div class="mt-6">
  {#if activeTab === 'overview'}
    <!-- Summary counts -->
    <div class="grid gap-3 mb-8" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); max-width: 520px">
      <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
        <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.members.length}</div>
        <div class="text-xs uppercase tracking-wider text-surface-z5">Members</div>
      </div>
      <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
        <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.teams.length}</div>
        <div class="text-xs uppercase tracking-wider text-surface-z5">Teams</div>
      </div>
      <div class="p-4 rounded-lg border border-surface-z3 bg-surface-z1 text-center">
        <div class="text-2xl font-semibold text-primary-z6 leading-tight mb-1">{data.repos.length}</div>
        <div class="text-xs uppercase tracking-wider text-surface-z5">Repos</div>
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
            <span class={planBadgeClass(data.org.plan ?? '')}>{data.org.plan}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-surface-z5">Created</span>
            <span class="text-surface-z7">{new Date(data.org.createdAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })}</span>
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
    {#if data.teams.length === 0}
      <p class="text-sm text-surface-z4">No teams yet.</p>
    {:else}
      <div class="grid gap-3" style="grid-template-columns: repeat(auto-fill, minmax(240px, 1fr))">
        {#each (data.teams as { id: string; name: string; slug: string }[]) as team}
          <a
            href="/team/{team.slug}"
            class="flex flex-col p-4 rounded-lg border border-surface-z3 bg-surface-z1 no-underline transition-colors hover:border-primary-z5 hover:bg-surface-z2"
          >
            <div class="font-semibold text-surface-z8">{team.name}</div>
            <p class="text-xs text-surface-z5 mt-1 font-mono">{team.slug}</p>
          </a>
        {/each}
      </div>
    {/if}

  {:else if activeTab === 'members'}
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">Members ({data.members.length})</h2>
    {#if data.members.length === 0}
      <p class="text-sm text-surface-z4">No members yet.</p>
    {:else}
      <Table data={memberRows} columns={memberColumns} />
    {/if}

  {:else if activeTab === 'repos'}
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider">Repos ({data.repos.length})</h2>
      <button class="px-3 py-1.5 rounded-lg border border-surface-z3 bg-surface-z1 text-sm text-surface-z5 cursor-not-allowed opacity-60" disabled>
        Add Repo
      </button>
    </div>
    {#if data.repos.length === 0}
      <p class="text-sm text-surface-z4">No repos indexed for this org yet.</p>
    {:else}
      <Table data={repoRows} columns={repoColumns} />
    {/if}
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
