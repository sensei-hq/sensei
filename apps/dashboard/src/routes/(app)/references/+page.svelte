<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();
</script>

<h1>References</h1>

{#if data.references.length === 0}
  <p>No references yet.</p>
{:else}
  <ul>
    {#each data.references as ref}
      <li>
        <a href={ref.url} target="_blank">{ref.title ?? ref.url}</a>
        {#if ref.description}<br><small>{ref.description}</small>{/if}
        {#if ref.tags?.length}<br><span class="tags">{ref.tags.join(', ')}</span>{/if}
      </li>
    {/each}
  </ul>
{/if}
