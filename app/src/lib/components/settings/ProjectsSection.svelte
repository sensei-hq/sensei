<script lang="ts">
  import { onMount } from 'svelte';
  import { Select } from '@rokkit/ui';
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

  // Static role picker options (label → wire value); '' == unassigned.
  const roleOptions = [
    { label: 'Unassigned', value: '' },
    ...ROLES.map((r) => ({ label: r.label, value: r.id })),
  ];
</script>

<div class="max-w-[940px]">
  <p class="text-sm text-ink-soft leading-normal m-0 mb-6">
    A single-repo project is the default. Multi-repo projects are auto-grouped from
    sibling folders and name patterns. Split when they shouldn't be together.
  </p>

  {#if loading}
    <div class="text-center p-12 bg-paper-mute rounded-lg border border-paper-edge">
      <span class="kanji text-4xl text-accent opacity-20 block mb-4">組</span>
      <p class="text-sm text-ink-soft">Loading projects…</p>
    </div>
  {:else if error}
    <div class="mb-6 p-4 rounded-md border border-danger bg-paper-mute flex items-start gap-4">
      <div>
        <div class="text-sm font-semibold text-danger">Could not load projects</div>
        <div class="text-xs text-ink-mute mt-1 font-mono select-text">{error}</div>
      </div>
    </div>
  {:else if projects.length === 0}
    <div class="text-center p-12 bg-paper-mute rounded-lg border border-paper-edge">
      <span class="kanji text-4xl text-accent opacity-20 block mb-4">組</span>
      <p class="text-sm text-ink-soft">No projects detected yet. Run the scan stage first.</p>
    </div>
  {:else}
    <div class="flex flex-col gap-3">
      {#each projects as project (project.id)}
        {@const repoFolders = project.folders.filter(f => f.kind !== 'folder')}
        {@const isMulti = repoFolders.length > 1}
        {@const isConfirmed = confirmed[project.id] !== false}
        {@const path = projectPath(project)}
        {@const stackChips = [
          ...(project.stack?.languages ?? []),
          ...(project.stack?.frameworks ?? []),
          ...(project.stack?.runtimes ?? []),
        ]}
        <div
          data-testid={`project-card-${project.id}`}
          data-confirmed={isConfirmed}
          class="rounded-lg p-4 border border-paper-edge transition-opacity duration-fast"
          class:bg-paper-mute={isConfirmed}
          class:bg-paper-soft={!isConfirmed}
          class:opacity-55={!isConfirmed}
        >
          <div class="grid grid-cols-[auto_1fr_auto_auto] gap-3 items-center">
            <!-- Kanji avatar -->
            <div class="kanji text-2xl text-accent w-10 h-10 flex items-center justify-center rounded-full bg-paper-soft border border-paper-edge">
              {project.icon?.value ?? '組'}
            </div>

            <!-- Editable name + meta -->
            <div class="min-w-0">
              <input
                value={project.name}
                onchange={(e) => rename(project.id, (e.target as HTMLInputElement).value)}
                aria-label="Project name"
                class="display text-xl font-normal bg-transparent border-none outline-none w-full p-0 text-ink"
              />
              <div class="font-mono text-xs text-ink-soft mt-0.5">
                {#if path}{path} · {/if}{repoFolders.length} {repoFolders.length === 1 ? 'repo' : 'repos'}
              </div>
            </div>

            <!-- Multi-repo badge — solid accent fill with paper text for contrast.
                 Was text-accent + accent-soft bg, which gave near-zero contrast. -->
            {#if isMulti}
              <span
                class="font-mono py-1 px-2 text-[11px] uppercase tracking-wider text-on-primary bg-accent rounded-sm"
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
              class:bg-ink={isConfirmed}
              class:text-paper-soft={isConfirmed}
              class:border={!isConfirmed}
              class:border-ink-soft={!isConfirmed}
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
                <span class="font-mono text-[11px] text-ink-mute bg-paper-mute rounded-sm py-0.5 px-1.5">{chip}</span>
              {/each}
            </div>
          {/if}

          <!-- Folder pills -->
          {#if project.folders.length > 0}
            <div class="flex flex-wrap gap-2 mt-3 pl-13">
              {#each project.folders as folder (folder.id)}
                <span class="font-mono inline-flex items-center gap-2 text-[11px] py-1 pl-2 pr-1 bg-paper-soft border border-paper-edge rounded-sm">
                  <!-- The visible {folder.name} labels this row; the Select trigger
                       takes its accessible name from its value text. (rokkit Select
                       v1.3.1 drops aria-*/data-* — only `class` reaches the DOM.) -->
                  <span class="text-ink-mute" id={`role-lbl-${folder.id}`}>{folder.name}</span>
                  <Select
                    size="sm"
                    items={roleOptions}
                    value={folder.role ?? ''}
                    onchange={(v) => setRole(folder, v as string)}
                  />
                </span>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <p class="text-xs text-ink-soft leading-normal mt-6">
      More options — external integrations, clients, custom rules — per project later from its Settings.
    </p>
  {/if}
</div>
