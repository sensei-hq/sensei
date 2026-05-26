<script lang="ts">
    import { PageHeader } from '$lib/components';
    let { data } = $props();
</script>

<PageHeader title="Instruments">
    {#snippet right()}
        <span class="text-sm text-ink-z6">{data.tools.length} tools</span>
    {/snippet}
</PageHeader>
<div class="px-6 py-6">

    {#if data.tools.length === 0}
        <p class="text-sm opacity-50">
            No instruments associated with this project yet.
        </p>
    {:else}
        <ul class="list-none m-0 p-0">
            {#each data.tools as tool (tool.id)}
                <li
                    class="tool-row flex items-center gap-2.5 py-2 border-b border-surface-z2 text-sm"
                >
                    <span class="font-semibold flex-1">{tool.name}</span>
                    <span class="opacity-50 text-xs">{tool.kind}</span>
                    <span
                        class="scope-badge text-xs px-1.5 py-px rounded-md font-mono"
                        class:global={tool.scope === "global"}
                        class:proj={tool.scope === "project"}
                    >
                        [{tool.scope}]
                    </span>
                </li>
            {/each}
        </ul>
    {/if}
</div>

<style>
    .tool-row:last-child {
        border-bottom: none;
    }
    .scope-badge.global {
        background: oklch(var(--color-surface-z3) / 1);
        opacity: 0.7;
    }
    .scope-badge.proj {
        background: oklch(var(--color-primary-z5) / 0.15);
        color: oklch(var(--color-primary-z5) / 1);
    }
</style>
