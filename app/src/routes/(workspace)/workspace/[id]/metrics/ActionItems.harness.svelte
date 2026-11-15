<script lang="ts">
    import ActionItems from './ActionItems.svelte';
    import type { ActionItem } from './action-items.js';
    import type { ItemActions } from './action-items-state.svelte.js';

    let {
        items,
        error = null,
        actions,
        scope = 'ftr',
    }: { items: ActionItem[]; error?: string | null; actions: ItemActions; scope?: string } = $props();
</script>

<ActionItems {items} {error} {actions} {scope} />
