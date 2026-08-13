<script lang="ts">
    import ActionItems from './ActionItems.svelte';
    import type { ActionItem } from './action-items.js';

    let { items, error = null }: { items: ActionItem[]; error?: string | null } = $props();
</script>

<ActionItems {items} {error} />
