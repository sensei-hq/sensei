<script lang="ts">
    import { PageHeader } from '$lib/components';
    let { data } = $props();
</script>

<PageHeader title="Memories" />
<div class="px-6 py-6">
    {#if data.pendingShare > 0}
        <div
            class="text-sm px-3.5 py-2.5 rounded-md mb-4"
            style="background: color-mix(in oklch, var(--accent) 10%, transparent); color: var(--accent);"
        >
            {data.pendingShare} memories pending collective share
        </div>
    {/if}
    <ul class="list-none m-0 p-0">
        {#each data.memories as m (m.id)}
            <li
                class="memory-row flex gap-3 py-2 border-b border-paper-mute text-sm"
            >
                <span class="flex-1">{m.title}</span>
                <span class="opacity-50 text-xs">{m.type}</span>
                <span class="text-xs font-mono"
                    >{Math.round((m.strength ?? 0) * 100)}%</span
                >
            </li>
        {/each}
    </ul>
</div>

<style>
    .memory-row:last-child {
        border-bottom: none;
    }
</style>
