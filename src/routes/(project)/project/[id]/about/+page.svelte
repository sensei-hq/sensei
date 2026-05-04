<script lang="ts">
  let { data } = $props();
  let p = $derived(data.project);
</script>
<div class="section-page">
  <h2>{p?.name ?? '—'}</h2>
  {#if p?.client}<p class="meta">Client: {p.client}</p>{/if}
  {#if p?.goal}<p class="goal">{p.goal}</p>{/if}
  <section>
    <h3 class="sub">Repos ({data.repos.length})</h3>
    <ul class="repo-list">
      {#each data.repos as repo (repo.id)}
        <li class="repo-row"><span class="repo-name">{repo.name}</span><span class="repo-path">{repo.path}</span></li>
      {/each}
    </ul>
  </section>
  {#if p?.stack}
    <section>
      <h3 class="sub">Stack</h3>
      <div class="stack-tags">
        {#each [...(p.stack.languages ?? []), ...(p.stack.frameworks ?? [])] as t}<span class="tag">{t}</span>{/each}
      </div>
    </section>
  {/if}
</div>
<style>
  .section-page { padding: 24px; max-width: 600px; }
  .meta, .goal { font-size: 13px; opacity: 0.7; margin: 4px 0; }
  .sub { font-size: 12px; font-weight: 600; opacity: 0.6; margin: 20px 0 8px; text-transform: uppercase; letter-spacing: .05em; }
  .repo-list { list-style: none; margin: 0; padding: 0; }
  .repo-row { display: flex; gap: 12px; padding: 6px 0; font-size: 13px; border-bottom: 1px solid var(--border); }
  .repo-name { font-weight: 600; }
  .repo-path { opacity: 0.5; font-size: 12px; font-family: monospace; overflow: hidden; text-overflow: ellipsis; }
  .stack-tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .tag { background: var(--surface-3); font-size: 12px; padding: 3px 8px; border-radius: 4px; }
</style>
