<script lang="ts">
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  let { data } = $props();
</script>
<div class="projects-page">
  <h2>Projects</h2>
  {#if data.projects.length === 0}
    <p class="empty-hint">No projects yet. Set up a project to get started.</p>
  {:else}
    <div class="project-grid">
      {#each data.projects as proj (proj.id)}
        <button
          type="button"
          class="project-card"
          onclick={() => openProjectWindow(proj.id, proj.name).catch(console.error)}
        >
          <span class="proj-kanji">{proj.icon?.value ?? '場'}</span>
          <span class="proj-name">{proj.name}</span>
          {#if proj.client}<span class="proj-client">{proj.client}</span>{/if}
          <span class="proj-maturity">{proj.maturity}</span>
          <span class="open-hint">↗</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
<style>
  .projects-page { padding: 24px; }
  .project-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 16px; margin-top: 16px; }
  .project-card { background: var(--surface-2); border-radius: 10px; padding: 20px 16px; display: flex; flex-direction: column; gap: 6px; cursor: pointer; border: none; text-align: left; color: inherit; transition: background .15s; }
  .project-card:hover { background: var(--surface-3); }
  .proj-kanji { font-size: 32px; }
  .proj-name { font-size: 15px; font-weight: 700; }
  .proj-client { font-size: 12px; opacity: 0.6; }
  .proj-maturity { font-size: 11px; opacity: 0.4; font-family: monospace; }
  .open-hint { font-size: 10px; opacity: 0.3; margin-top: auto; }
  .empty-hint { opacity: 0.5; font-size: 13px; }
</style>
