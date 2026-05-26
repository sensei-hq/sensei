<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import { ROLES } from '$lib/setup/types.js';
  import { commonParent } from '$lib/scan-state.svelte.js';
  import type { DaemonProject, DaemonProjectFolder } from '$lib/setup/contracts.js';

  let loading = $state(true);
  let error = $state<string | null>(null);

  const projects = $derived(wizardState.projects.projects);
  const confirmed = $derived(wizardState.projects.confirmed);

  onMount(async () => {
    try {
      await wizardState.refreshProjects();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  function projectPath(p: DaemonProject): string {
    if (p.folders.length === 0) return '';
    return commonParent(p.folders.map(f => f.path));
  }

  function rename(id: string, value: string) {
    const p = projects.find(x => x.id === id);
    if (p) p.name = value;
  }

  function toggleConfirm(id: string) {
    wizardState.projects.confirmed[id] = !confirmed[id];
  }

  function setRole(folder: DaemonProjectFolder, role: string) {
    folder.role = role;
  }

  function roleClass(role: string | null): string {
    switch (role) {
      case 'frontend': return 'text-primary-z6 bg-primary-z2';
      case 'backend':  return 'text-success-z6 bg-success-z2';
      case 'library':  return 'text-warning-z6 bg-warning-z2';
      default:         return 'text-surface-z7 bg-surface-z3';
    }
  }
</script>

<div class="max-w-[940px]">
  <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
    A single-repo project is the default. Multi-repo projects are auto-grouped from
    sibling folders and name patterns. Split when they shouldn't be together.
  </p>

  {#if loading}
    <div class="text-center p-12 bg-surface-z2 rounded-lg border border-surface-z3">
      <span class="kanji text-4xl text-primary-z5 opacity-20 block mb-4">組</span>
      <p class="text-sm text-surface-z6">Loading projects…</p>
    </div>
  {:else if error}
    <div class="mb-6 p-4 rounded-md border border-danger-z5 bg-surface-z2 flex items-start gap-4">
      <div>
        <div class="text-sm font-semibold text-danger-z5">Could not load projects</div>
        <div class="text-xs text-surface-z7 mt-1 font-mono">{error}</div>
      </div>
    </div>
  {:else if projects.length === 0}
    <div class="text-center p-12 bg-surface-z2 rounded-lg border border-surface-z3">
      <span class="kanji text-4xl text-primary-z5 opacity-20 block mb-4">組</span>
      <p class="text-sm text-surface-z6">No projects detected yet. Run the scan stage first.</p>
    </div>
  {:else}
    <div class="flex flex-col gap-3">
      {#each projects as project (project.id)}
        {@const isMulti = project.folders.length > 1}
        {@const isConfirmed = confirmed[project.id] !== false}
        {@const path = projectPath(project)}
        {@const stackChips = [
          ...(project.stack.languages ?? []),
          ...(project.stack.frameworks ?? []),
          ...(project.stack.runtimes ?? []),
        ]}
        <div
          data-testid={`project-card-${project.id}`}
          data-confirmed={isConfirmed}
          class="rounded-lg p-4 border transition-opacity duration-fast"
          class:bg-surface-z2={isConfirmed}
          class:border-surface-z3={isConfirmed}
          class:bg-surface-z1={!isConfirmed}
          class:border-surface-z2={!isConfirmed}
          class:opacity-55={!isConfirmed}
        >
          <div class="grid grid-cols-[auto_1fr_auto_auto] gap-3 items-center">
            <!-- Kanji avatar -->
            <div class="kanji text-2xl text-primary-z5 w-10 h-10 flex items-center justify-center rounded-full bg-surface-z1 border border-surface-z3">
              {project.icon?.value ?? '組'}
            </div>

            <!-- Editable name + meta -->
            <div class="min-w-0">
              <input
                value={project.name}
                onchange={(e) => rename(project.id, (e.target as HTMLInputElement).value)}
                aria-label="Project name"
                class="display text-xl font-normal bg-transparent border-none outline-none w-full p-0 text-surface-z9"
              />
              <div class="mono text-xs text-surface-z6 mt-0.5">
                {#if path}{path} · {/if}{project.folders.length} {project.folders.length === 1 ? 'folder' : 'folders'}
              </div>
            </div>

            <!-- Multi-repo badge -->
            {#if isMulti}
              <span
                class="mono py-1 px-2 text-[11px] uppercase tracking-wider text-primary-z6 border border-primary-z5 rounded-sm bg-primary-z2"
                >multi-repo</span
              >
            {:else}
              <span></span>
            {/if}

            <!-- Confirm toggle -->
            <button
              type="button"
              aria-pressed={isConfirmed}
              aria-label={isConfirmed ? 'Unconfirm project' : 'Confirm project'}
              onclick={() => toggleConfirm(project.id)}
              class="w-6 h-6 rounded-md flex items-center justify-center transition-colors duration-fast"
              class:bg-surface-z9={isConfirmed}
              class:text-surface-z1={isConfirmed}
              class:border={!isConfirmed}
              class:border-surface-z5={!isConfirmed}
            >
              {#if isConfirmed}
                <svg width="12" height="12" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M3 8 L7 12 L13 4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
                </svg>
              {/if}
            </button>
          </div>

          <!-- Stack chips -->
          {#if stackChips.length > 0}
            <div class="flex flex-wrap gap-1 mt-3 pl-13">
              {#each stackChips as chip}
                <span class="mono text-[11px] text-surface-z7 bg-surface-z3 rounded-sm py-0.5 px-1.5">{chip}</span>
              {/each}
            </div>
          {/if}

          <!-- Folder pills -->
          {#if project.folders.length > 0}
            <div class="flex flex-wrap gap-2 mt-3 pl-13">
              {#each project.folders as folder (folder.id)}
                <span class="mono inline-flex items-center gap-2 text-[11px] py-1 pl-2 pr-1 bg-surface-z1 border border-surface-z3 rounded-sm">
                  <span class="text-surface-z8">{folder.name}</span>
                  <select
                    value={folder.role ?? 'backend'}
                    onchange={(e) => setRole(folder, (e.target as HTMLSelectElement).value)}
                    aria-label={`Role for ${folder.name}`}
                    class="role-select mono text-[11px] py-0.5 px-1.5 rounded-sm border-none outline-none cursor-pointer font-semibold {roleClass(folder.role)}"
                  >
                    {#each ROLES as r}
                      <option value={r.id}>{r.label}</option>
                    {/each}
                  </select>
                </span>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <p class="text-xs text-surface-z6 leading-normal mt-6">
      More options — external integrations, clients, custom rules — per project later from its Settings.
    </p>
  {/if}
</div>

<style>
  .role-select {
    appearance: none;
    -webkit-appearance: none;
  }
</style>
