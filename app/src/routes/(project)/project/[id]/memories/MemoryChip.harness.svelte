<script lang="ts">
    import MemoryChip from './MemoryChip.svelte';
    import type { ChipTone } from './memories-state.svelte.js';

    let {
        tone,
        label,
    }: {
        tone: ChipTone;
        label: string;
    } = $props();
</script>

<MemoryChip {tone} {label} />
