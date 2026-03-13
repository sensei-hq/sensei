<script lang="ts">
  import { List } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data } = $props();

  // Map repos to the shape Rokkit List expects: { label, description, href }
  const items = $derived(data.repos.map(r => ({
    label: r.name,
    description: `${r.file_count} files · ${r.symbol_count} symbols · ${(r.stack ?? []).join(', ') || 'unknown stack'} · Traceability: 0 links`,
    href: `/repos/${r.id}`,
    meta: r.last_indexed_at
      ? `Last indexed ${new Date(r.last_indexed_at).toLocaleString()}`
      : 'Never indexed',
  })));
</script>

<h1>Indexed Repos</h1>

{#if items.length === 0}
  <p>No repos indexed yet. Run <code>sensei init</code> in your project directory.</p>
{:else}
  <List {items} />
{/if}
