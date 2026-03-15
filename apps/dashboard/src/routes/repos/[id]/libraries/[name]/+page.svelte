<!-- apps/dashboard/src/routes/repos/[id]/libraries/[name]/+page.svelte -->
<script lang="ts">
  import type { PageData } from './$types';
  const { data }: { data: PageData } = $props();

  function shortContent(content: string | null | undefined): string {
    if (!content) return '';
    return content.length > 300 ? content.slice(0, 300) + '...' : content;
  }
</script>

<a href="/repos/{data.repo.id}/libraries">← Library Docs</a>
<h1>{data.libName}</h1>

{#if data.skillPath}
  <p>Skill file: <code>{data.skillPath}</code></p>
{/if}

{#if data.sections.length > 0}
  <p>{data.sections.length} sections indexed</p>
  <table>
    <thead>
      <tr>
        <th>Title</th>
        <th>Component</th>
        <th>Description</th>
        <th>Content / Link</th>
      </tr>
    </thead>
    <tbody>
      {#each data.sections as section}
        <tr>
          <td>{section.title}</td>
          <td>{section.component ?? '—'}</td>
          <td>{section.description}</td>
          <td>
            {#if section.url}
              <a href={section.url} target="_blank">↗ View</a>
            {:else if section.content}
              <details>
                <summary>Preview</summary>
                <pre>{shortContent(section.content)}</pre>
              </details>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p>No sections indexed for this library.</p>
{/if}

<style>
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; vertical-align: top; }
  pre { white-space: pre-wrap; font-size: 0.85em; max-width: 50ch; }
</style>
