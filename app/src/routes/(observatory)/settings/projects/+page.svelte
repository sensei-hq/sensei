<script lang="ts">
  import { PageHeader } from '$lib/components';
  import ProjectsSection from '$lib/components/settings/ProjectsSection.svelte';
</script>

<PageHeader kanji="組" eyebrow="Settings" title="Projects" />
<div class="max-w-[820px] mx-auto px-12 pt-8 pb-16" data-testid="settings-projects">
  <ProjectsSection />
</div>
