<script lang="ts">
    import { PageHeader } from "$lib/components";
    import InferenceAssignmentsPanel from "../InferenceAssignmentsPanel.svelte";
</script>

<PageHeader kanji="想" eyebrow="Settings" title="Inference" />
<div class="max-w-[720px] mx-auto px-12 pt-8 pb-16">
    <InferenceAssignmentsPanel />
</div>
