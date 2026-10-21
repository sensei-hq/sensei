<script lang="ts">
    import { railIcon } from "../settings-nav";
    import { PageHeader, Screen } from "$lib/components";
    import InferenceAssignmentsPanel from "../InferenceAssignmentsPanel.svelte";
    import LocalModelsPanel from "../LocalModelsPanel.svelte";
</script>

<Screen>
    {#snippet header()}
    <PageHeader kanji="想" icon={railIcon("/settings/inference")} eyebrow="Settings" title="Inference" />
    {/snippet}
    <div class="max-w-[720px] mx-auto px-12 pt-8 pb-16 flex flex-col gap-5">
        <InferenceAssignmentsPanel />
        <LocalModelsPanel />
    </div>
</Screen>
