<script lang="ts">
    import { onMount } from "svelte";
    import { scanState } from "$lib/scan.js";
    import ScanProgress from "$lib/components/indexer/ScanProgress.svelte";

    // The scan is owned by this stage. Auto-engage on mount so the page
    // reflects current daemon state without a manual button — the
    // singleton's idempotency guard makes repeat calls cheap.
    onMount(() => { void scanState.start(); });
</script>

<ScanProgress variant="full" />
