<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Patterns</h2>
  {#if data.followed.length > 0}
    <section>
      <h3 class="sub">Followed ({data.followed.length})</h3>
      <ul class="pattern-list">
        {#each data.followed as p (p.id)}
          <li class="pattern-row"><span class="p-name">{p.name}</span><span class="p-lifecycle">{p.lifecycle}</span></li>
        {/each}
      </ul>
    </section>
  {/if}
  {#if data.antiPatterns.length > 0}
    <section>
      <h3 class="sub anti">Anti-patterns ({data.antiPatterns.length})</h3>
      <ul class="pattern-list">
        {#each data.antiPatterns as p (p.id)}
          <li class="pattern-row anti"><span class="p-name">{p.name}</span><span class="p-lifecycle">{p.lifecycle}</span></li>
        {/each}
      </ul>
    </section>
  {/if}
</div>
<style>
  .section-page { padding: 24px; }
  .sub { font-size: 12px; font-weight: 600; opacity: 0.6; margin: 16px 0 8px; text-transform: uppercase; letter-spacing: .05em; }
  .sub.anti { color: var(--red, red); }
  .pattern-list { list-style: none; margin: 0; padding: 0; }
  .pattern-row { display: flex; gap: 12px; padding: 7px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .pattern-row.anti { opacity: 0.8; }
  .p-name { flex: 1; }
  .p-lifecycle { font-size: 11px; opacity: 0.5; font-family: monospace; }
</style>
