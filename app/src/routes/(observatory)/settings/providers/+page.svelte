<script lang="ts">
  import { PageHeader } from '$lib/components';
  import ProvidersSection from '$lib/components/settings/ProvidersSection.svelte';
</script>

<PageHeader kanji="鍵" eyebrow="Settings" title="Providers" />
<div class="max-w-[820px] mx-auto px-12 pt-8 pb-16">
  <ProvidersSection />
</div>
