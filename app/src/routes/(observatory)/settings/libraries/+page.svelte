<script lang="ts">
  import { PageHeader } from '$lib/components';
  import LibrariesSection from '$lib/components/settings/LibrariesSection.svelte';
</script>

<PageHeader kanji="書" eyebrow="Settings" title="Libraries" />
<div class="max-w-[820px] mx-auto px-12 pt-8 pb-16">
  <LibrariesSection />
</div>
