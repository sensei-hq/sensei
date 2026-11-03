<script lang="ts">
    import { railIcon } from "../settings-nav";
  import { PageHeader, Screen } from '$lib/components';
  import LibrariesSection from '$lib/components/settings/LibrariesSection.svelte';
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="書" icon={railIcon("/settings/libraries")} eyebrow="Settings" title="Libraries" />
    {/snippet}
    <div class="max-w-[820px] mx-auto px-12 pt-8 pb-16">
      <LibrariesSection />
    </div>
</Screen>
