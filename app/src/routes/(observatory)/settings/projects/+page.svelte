<script lang="ts">
    import { railIcon } from "../settings-nav";
  import { onMount } from 'svelte';
  import { PageHeader, Screen } from '$lib/components';
  import ProjectsSection from '$lib/components/settings/ProjectsSection.svelte';
  import { ProjectsSectionState } from '$lib/components/settings/projects-section-state.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';

  // The page owns the state so the header and the search can sit in the
  // screen's fixed header above the list (#271, #293).
  const projects = new ProjectsSectionState();
  const api = senseiApi(appState.port);
  onMount(() => {
    void projects.load(api);
  });
</script>

<Screen>
  {#snippet header()}
    <div class="bg-paper" data-testid="settings-projects-header">
      <PageHeader kanji="組" icon={railIcon("/settings/projects")} eyebrow="Settings" title="Projects" />
      <div class="max-w-[820px] mx-auto px-12 pb-3">
        <input
          type="search"
          bind:value={projects.query}
          placeholder="Search projects or repositories"
          aria-label="Search projects or repositories"
          class="w-full text-sm px-3 py-2 rounded-md border border-paper-edge bg-paper-soft text-ink
                 placeholder:text-ink-faint outline-none focus:border-ink-soft"
        />
      </div>
    </div>
  {/snippet}
  <div class="max-w-[820px] mx-auto px-12 pt-4 pb-16" data-testid="settings-projects">
    <ProjectsSection state={projects} {api} />
  </div>
</Screen>
