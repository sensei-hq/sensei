<script lang="ts">
  import { PageHeader } from '$lib/components';
  import InstrumentsSection from '$lib/components/settings/InstrumentsSection.svelte';
</script>

<PageHeader kanji="器" eyebrow="Settings" title="Instruments" />
<div class="max-w-[820px] mx-auto px-12 pt-8 pb-16">
  <InstrumentsSection />
</div>
