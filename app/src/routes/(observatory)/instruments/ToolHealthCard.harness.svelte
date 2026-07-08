<script lang="ts">
    import type { ToolHealthSource } from "$lib/types.js";
    import ToolHealthCard from "./ToolHealthCard.svelte";

    let {
        source,
        active,
        onopen,
    }: {
        source: ToolHealthSource;
        active?: boolean;
        onopen?: (source: ToolHealthSource) => void;
    } = $props();
</script>

<ToolHealthCard {source} {active} {onopen} />
