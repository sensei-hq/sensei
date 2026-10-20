<script lang="ts">
    import { railIcon } from "../settings-nav";
  import { PageHeader } from '$lib/components';
  import ProvidersSection from '$lib/components/settings/ProvidersSection.svelte';
</script>

<PageHeader kanji="鍵" icon={railIcon("/settings/providers")} eyebrow="Settings" title="Providers" />
<div class="max-w-[820px] mx-auto px-12 pt-8 pb-16">
  <ProvidersSection />
</div>
