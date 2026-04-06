<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const ftrColor = (score: number | null): string => {
    if (score === null) return '#94a3b8';
    if (score >= 0.8) return '#166534';
    if (score >= 0.5) return '#92400e';
    return '#991b1b';
  };

  const fmtFtr = (score: number | null) =>
    score !== null ? score.toFixed(3) : '—';
</script>

<a href="/repos">← Repos</a>
<h1>Team FTR Leaderboard</h1>
<p style="color:#64748b;font-size:0.875rem">Last 30 days — ranked by average FTR score</p>

{#if data.members.length === 0}
  <p style="color:#64748b;margin-top:24px">No team member data yet. Members appear here once they complete sessions with <code>checkpoint()</code>.</p>
{:else}
  <table class="main-table">
    <thead>
      <tr>
        <th class="rank-col">#</th>
        <th>Member</th>
        <th>Role</th>
        <th class="num">Avg FTR (30d)</th>
        <th class="num">Sessions</th>
      </tr>
    </thead>
    <tbody>
      {#each data.members as member, i (member.userId)}
        <tr>
          <td class="rank-col mono" style="color:#94a3b8">{i + 1}</td>
          <td class="mono" style="font-size:0.875rem">{member.email}</td>
          <td>
            <span class="badge" class:badge-admin={member.role === 'admin' || member.role === 'owner'}>
              {member.role}
            </span>
          </td>
          <td class="num mono" style="font-weight:700;color:{ftrColor(member.avgFtr)}">
            {fmtFtr(member.avgFtr)}
          </td>
          <td class="num mono">{member.sessionCount}</td>
        </tr>
        {#if member.repoBreakdown.length > 0}
          <tr class="breakdown-row">
            <td></td>
            <td colspan="4">
              <div class="repo-pills">
                {#each member.repoBreakdown as repo (repo.repoId)}
                  <span class="repo-pill">
                    <span class="repo-name">{repo.repoName}</span>
                    <span class="repo-ftr" style="color:{ftrColor(repo.avgFtr)}">
                      {fmtFtr(repo.avgFtr)}
                    </span>
                    <span class="repo-count" style="color:#94a3b8">({repo.sessionCount})</span>
                  </span>
                {/each}
              </div>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
{/if}

<style>
  h1 { font-size: 1.5rem; font-weight: 700; color: #1e293b; margin: 8px 0 4px; }
  .main-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; margin-top: 16px; }
  .main-table th {
    text-align: left;
    padding: 6px 12px 6px 0;
    border-bottom: 2px solid #e2e8f0;
    color: #64748b;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .main-table td { padding: 10px 12px 10px 0; border-bottom: 1px solid #f1f5f9; vertical-align: top; }
  .main-table tr:hover td { background: #f8fafc; }
  .breakdown-row td { padding-top: 0; border-bottom: 1px solid #f1f5f9; }
  .rank-col { width: 2rem; }
  .num { text-align: right; }
  .mono { font-family: monospace; }
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    background: #f1f5f9;
    color: #475569;
  }
  .badge-admin { background: #ede9fe; color: #5b21b6; }
  .repo-pills { display: flex; flex-wrap: wrap; gap: 6px; padding-bottom: 4px; }
  .repo-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 0.75rem;
  }
  .repo-name { color: #475569; }
  .repo-ftr { font-family: monospace; font-weight: 600; }
  .repo-count { font-family: monospace; }
</style>
