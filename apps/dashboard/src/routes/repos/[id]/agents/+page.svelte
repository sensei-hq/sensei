<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const statusColor: Record<string, string> = {
    present: 'status-present',
    stale:   'status-stale',
    missing: 'status-missing',
  };

  const statusLabel: Record<string, string> = {
    present: 'Fresh',
    stale:   'Stale',
    missing: 'Missing',
  };

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString();
  }

  function shortPath(path: string): string {
    const home = path.indexOf('.claude/skills/');
    return home !== -1 ? '~/.claude/skills/' + path.slice(home + '.claude/skills/'.length) : path;
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Agent Skills</h1>

{#if data.agent}
  <p>Configured for: <strong>{data.agent === 'claude' ? 'Claude Code' : data.agent}</strong></p>

  {#if data.skills.length > 0}
    <table>
      <thead>
        <tr>
          <th>Category</th>
          <th>File</th>
          <th>Generated</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {#each data.skills as skill}
          <tr>
            <td>{skill.category}</td>
            <td><code>{shortPath(skill.path)}</code></td>
            <td>{formatDate(skill.generatedAt)}</td>
            <td class={statusColor[skill.status] ?? ''}>{statusLabel[skill.status] ?? skill.status}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p>No skill files found on disk.</p>
  {/if}

  <form method="POST" action="?/regenerate">
    <button type="submit">Regenerate Skills</button>
  </form>
{:else}
  <p>No agent skills configured for this repo.</p>
  <p>Run <code>sensei setup --agent claude</code> in the repo directory to generate skills.</p>
{/if}

<style>
  .status-present { color: green; }
  .status-stale   { color: goldenrod; }
  .status-missing { color: red; }
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; }
</style>
