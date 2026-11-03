<script lang="ts">
    import { railIcon } from "../settings-nav";
  import { PageHeader, Screen } from '$lib/components';
  import InstrumentsSection from '$lib/components/settings/InstrumentsSection.svelte';
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="器" icon={railIcon("/settings/instruments")} eyebrow="Settings" title="Instruments" />
    {/snippet}
    <div class="max-w-[820px] mx-auto px-12 pt-8 pb-16">
      <InstrumentsSection />
    </div>
</Screen>
