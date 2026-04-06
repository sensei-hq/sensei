<script lang="ts">
  import { Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const fmt = (iso: string | null) =>
    iso ? new Date(iso).toLocaleString() : '—';

  const ftrColor = (score: number | null): string => {
    if (score === null) return '#94a3b8';
    if (score >= 0.8) return '#166534';
    if (score >= 0.5) return '#92400e';
    return '#991b1b';
  };

  const columns = [
    { name: 'email',       label: 'Email',       sortable: true },
    { name: 'role',        label: 'Role',        sortable: true },
    { name: 'lastActive',  label: 'Last Active', sortable: true },
    { name: 'avgFtr',      label: 'Avg FTR (30d)', sortable: true },
    { name: 'sessions',    label: 'Sessions',    sortable: true },
  ];

  const rows = $derived(data.members.map(m => ({
    email: m.email,
    role: m.role,
    lastActive: fmt(m.lastActive),
    avgFtr: m.avgFtr !== null ? m.avgFtr.toFixed(3) : '—',
    sessions: m.sessionCount,
  })));
</script>

<a href="/repos">← Repos</a>
<h1>Members</h1>
<p style="color:#64748b;font-size:0.875rem">{data.members.length} member{data.members.length === 1 ? '' : 's'} in this account</p>

{#if data.members.length === 0}
  <p style="color:#64748b;margin-top:24px">No members found. Member data appears here once the <code>core.profile_accounts</code> table is populated.</p>
{:else}
  <div style="margin-top:16px">
    <table class="members-table">
      <thead>
        <tr>
          <th>Email</th>
          <th>Role</th>
          <th>Last Active</th>
          <th class="num">Avg FTR (30d)</th>
          <th class="num">Sessions</th>
        </tr>
      </thead>
      <tbody>
        {#each data.members as member (member.userId)}
          <tr>
            <td class="mono" style="font-size:0.875rem">{member.email}</td>
            <td>
              <span
                class="badge"
                style={member.role === 'admin' || member.role === 'owner'
                  ? 'background:#ede9fe;color:#5b21b6'
                  : 'background:#f1f5f9;color:#475569'}
              >{member.role}</span>
            </td>
            <td style="color:#64748b;font-size:0.8rem;white-space:nowrap">{fmt(member.lastActive)}</td>
            <td class="num mono" style="font-weight:700;color:{ftrColor(member.avgFtr)}">
              {member.avgFtr !== null ? member.avgFtr.toFixed(3) : '—'}
            </td>
            <td class="num mono">{member.sessionCount}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  h1 { font-size: 1.5rem; font-weight: 700; color: #1e293b; margin: 8px 0 4px; }
  .members-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
  .members-table th {
    text-align: left;
    padding: 6px 12px 6px 0;
    border-bottom: 2px solid #e2e8f0;
    color: #64748b;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .members-table td { padding: 10px 12px 10px 0; border-bottom: 1px solid #f1f5f9; }
  .members-table tr:hover td { background: #f8fafc; }
  .num { text-align: right; }
  .mono { font-family: monospace; }
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
  }
</style>
