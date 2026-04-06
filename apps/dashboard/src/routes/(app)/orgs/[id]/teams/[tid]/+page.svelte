<script lang="ts">
  import { BreadCrumbs, Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const breadcrumbs = $derived([
    { label: 'Orgs', href: '/orgs' },
    { label: data.org.name, href: `/orgs/${data.org.id}` },
    { label: 'Teams' },
    { label: data.team.name },
  ]);

  const memberColumns = [
    { name: 'name', label: 'Name', sortable: true },
    { name: 'email', label: 'Email', sortable: true },
    { name: 'role', label: 'Role', sortable: true },
  ];

  const repoColumns = [
    { name: 'name', label: 'Name', sortable: true },
    { name: 'remoteUrl', label: 'Remote URL', sortable: false },
    { name: 'lastIndexedDisplay', label: 'Last Indexed', sortable: true },
  ];

  const repoRows = $derived(data.repos.map(r => ({
    ...r,
    lastIndexedDisplay: r.lastIndexedAt
      ? new Date(r.lastIndexedAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
      : 'Never',
  })));
</script>

<div class="mb-4">
  <BreadCrumbs items={breadcrumbs} />
</div>

<div class="mb-6">
  <h1 class="text-2xl font-semibold text-surface-z8 mb-1">{data.team.name}</h1>
  <p class="text-sm text-surface-z5">{data.team.description}</p>
</div>

<div class="grid gap-6" style="grid-template-columns: repeat(auto-fit, minmax(320px, 1fr))">
  <div>
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">
      Members ({data.members.length})
    </h2>
    <Table data={data.members} columns={memberColumns} />
  </div>

  <div>
    <h2 class="text-xs font-semibold text-surface-z5 uppercase tracking-wider mb-3">
      Repos ({data.repos.length})
    </h2>
    <Table data={repoRows} columns={repoColumns} />
  </div>
</div>
