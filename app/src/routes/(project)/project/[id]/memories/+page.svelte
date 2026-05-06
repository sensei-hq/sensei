<script lang="ts">
    let { data } = $props();
</script>

<div class="px-6 py-6">
    <h2 class="text-xl font-normal m-0 mb-4">Memories</h2>
    {#if data.pendingShare > 0}
        <div
            class="text-ui px-3.5 py-2.5 rounded-md mb-4"
            style="background: oklch(var(--color-primary-z5) / 0.10); color: oklch(var(--color-primary-z5) / 1);"
        >
            {data.pendingShare} memories pending collective share
        </div>
    {/if}
    <ul class="list-none m-0 p-0">
        {#each data.memories as m (m.id)}
            <li
                class="memory-row flex gap-3 py-2 border-b border-surface-z2 text-ui"
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
