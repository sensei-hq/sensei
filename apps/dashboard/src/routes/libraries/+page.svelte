<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();

  let search = '';
  $: filtered = data.libraries.filter((l: any) =>
    l.name.toLowerCase().includes(search.toLowerCase()) ||
    (l.description ?? '').toLowerCase().includes(search.toLowerCase())
  );
</script>

<h1>Libraries</h1>

<input type="search" placeholder="Search libraries…" bind:value={search} />

<table>
  <thead>
    <tr><th>Name</th><th>Ecosystem</th><th>Version</th><th>Description</th><th>llms.txt</th></tr>
  </thead>
  <tbody>
    {#each filtered as lib}
      <tr>
        <td>
          {#if lib.homepage_url}<a href={lib.homepage_url} target="_blank">{lib.name}</a>
          {:else}{lib.name}{/if}
        </td>
        <td>{lib.ecosystem}</td>
        <td>{lib.version ?? '—'}</td>
        <td>{lib.description ?? '—'}</td>
        <td>
          {#if lib.llms_txt_url}
            {lib.llms_txt_fetched_at ? '✓ cached' : 'not fetched'}
          {:else}—{/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
