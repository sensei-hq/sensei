<script lang="ts">
  import { getContext, onMount } from 'svelte';
  import { page } from '$app/state';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { ProjectListItem } from '$lib/types.js';

  interface ProjectCtx {
    readonly value: ProjectListItem | null;
    readonly loading: boolean;
  }
  const ctx = getContext<ProjectCtx>('project');

  const projectId = $derived(page.params.id ?? '');

  type Repo = {
    repo_id: string;
    name: string;
    path: string;
    role?: string;
    language?: string;
    project_id?: string;
  };

  let repos = $state<Repo[]>([]);

  onMount(async () => {
    if (!projectId) return;
    const api = senseiApi(appState.port);
    const allRepos = await api.getRepos();
    repos = allRepos.filter((r: Repo) => r.project_id === projectId);
  });

  const project = $derived(ctx.value);
  const stackTags = $derived([
    ...(project?.stack?.languages ?? []),
    ...(project?.stack?.frameworks ?? []),
  ]);
</script>

<div class="max-w-[820px] mx-auto px-12 py-10 pb-16">
  {#if ctx.loading}
    <p class="text-sm text-ink-soft">Loading project…</p>
  {:else if !project}
    <p class="text-sm text-ink-soft">Project not found.</p>
  {:else}
    <!-- Header — mirror of the pre-restructure top of the page. -->
    <header class="mb-7">
      <h1 class="display text-2xl font-normal m-0 mb-1">{project.name}</h1>
      {#if project.goal}
        <p class="text-sm text-ink-mute m-0 mb-3 leading-normal">{project.goal}</p>
      {/if}
      {#if stackTags.length > 0}
        <div class="flex gap-1.5 flex-wrap">
          {#each stackTags as tag}
            <span class="px-2.5 py-1 rounded-full text-xs bg-paper-mute text-ink-soft">{tag}</span>
          {/each}
        </div>
      {/if}
    </header>

    <section class="mb-7">
      <h3 class="text-sm font-medium m-0 mb-3.5 text-ink">Repositories</h3>
      {#if repos.length === 0}
        <p class="text-sm text-ink-soft">No repositories linked to this project.</p>
      {:else}
        <div class="flex flex-col gap-0.5">
          {#each repos as repo (repo.repo_id)}
            <div class="repo-row flex items-center gap-3 px-3.5 py-2.5 rounded-md transition-colors duration-fast">
              <span class="text-sm font-medium text-ink">{repo.name}</span>
              <span class="text-xs text-ink-soft font-mono flex-1">{repo.path}</span>
              {#if repo.role}
                <span class="text-xs uppercase tracking-wide text-ink-soft px-2 py-0.5 rounded-full bg-paper-mute">{repo.role}</span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section>
      <h3 class="text-sm font-medium m-0 mb-3.5 text-ink">Recommendations</h3>
      <div class="flex flex-col items-center gap-2 px-5 py-10 bg-paper-mute border border-paper-mute rounded-lg text-center">
        <span class="kanji text-2xl text-accent opacity-40">薦</span>
        <p class="text-sm text-ink-soft leading-normal max-w-[420px] m-0">
          Recommendations appear once sensei has observed enough sessions in
          this project.
        </p>
      </div>
    </section>
  {/if}
</div>

<style>
  .repo-row:hover {
    background: var(--paper-mute);
  }
</style>
