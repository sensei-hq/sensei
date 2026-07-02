<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { ProjectListItem } from '$lib/types.js';
  import ProjectSidebar from '$lib/components/ProjectSidebar.svelte';
  import { setContext } from 'svelte';

  interface Props {
    children?: import('svelte').Snippet;
  }

  const { children }: Props = $props();

  const projectId = $derived(page.params.id ?? '');

  let project = $state<ProjectListItem | null>(null);
  let loading = $state(true);

  onMount(async () => {
    if (!projectId) {
      loading = false;
      return;
    }
    const api = senseiApi(appState.port);
    const projects = await api.listProjects();
    project = projects.find((p) => p.id === projectId) ?? null;
    loading = false;
  });

  const projectKanji = $derived(project?.icon?.value ?? '場');
  const projectName = $derived(project?.name ?? projectId);
  const client = $derived(project?.client ?? null);

  // Sections + kanji glyphs match the mockup's ProjectSidebarRouted list.
  // Badges are placeholder zeroes until the per-project counts endpoint lands;
  // dropping badge to undefined keeps the sidebar clean until real data flows.
  const sections = [
    { id: 'overview', kanji: '全', label: 'Overview' },
    { id: 'sessions', kanji: '録', label: 'Sessions' },
    { id: 'memories', kanji: '覚', label: 'Memories' },
    { id: 'traceability', kanji: '巻', label: 'Traceability' },
    { id: 'libraries', kanji: '庫', label: 'Libraries' },
    { id: 'instruments', kanji: '具', label: 'Instruments' },
    { id: 'patterns', kanji: '紋', label: 'Patterns' },
    { id: 'impact', kanji: '果', label: 'Impact' },
    { id: 'about', kanji: '識', label: 'About' },
  ];

  // Expose the loaded project to child pages so they don't each re-fetch it.
  setContext('project', {
    get value() {
      return project;
    },
    get loading() {
      return loading;
    },
  });
</script>

<div class="grid grid-cols-[240px_1fr] min-h-full">
  <ProjectSidebar
    {projectId}
    {projectName}
    {projectKanji}
    {client}
    {sections}
  />
  <main class="overflow-auto">
    {@render children?.()}
  </main>
</div>
