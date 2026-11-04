<script lang="ts">
    import ActionItems from './ActionItems.svelte';
    import type { ActionItem } from './action-items.js';
    import type { ItemActions } from './action-items-state.svelte.js';

    let {
        items,
        error = null,
        actions,
    }: { items: ActionItem[]; error?: string | null; actions: ItemActions } = $props();
</script>

<ActionItems {items} {error} {actions} />
