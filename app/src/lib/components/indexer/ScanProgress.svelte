<script lang="ts">
    import ScanMini from "./ScanMini.svelte";
    import ScanBand from "./ScanBand.svelte";
    import ScanFull from "./ScanFull.svelte";

    // One shared progress component, three variants, one state. The variant
    // is static per mount point — runtime switching happens through the
    // state's own visibility (`expanded`, run status), never this prop.
    let { variant }: { variant: "mini" | "band" | "full" } = $props();
</script>

{#if variant === "mini"}
    <ScanMini />
{:else if variant === "band"}
    <ScanBand />
{:else}
    <ScanFull />
{/if}
