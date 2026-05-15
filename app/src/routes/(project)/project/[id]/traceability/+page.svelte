<script lang="ts">
    let { data } = $props();
</script>

<div class="px-6 py-6">
    <h2 class="text-xl font-normal m-0 mb-2">Traceability</h2>
    <div class="text-sm opacity-70 mb-4">
        <span>{data.total} tracked</span>
        <span> · </span>
        <span style="color: oklch(var(--color-warning-z5) / 1);"
            >{data.drifted} drifted</span
        >
        <span> · </span>
        <span style="color: oklch(var(--color-primary-z5) / 1);"
            >{data.broken} broken</span
        >
    </div>
    <ul class="list-none m-0 p-0">
        {#each data.driftItems as item (item.id)}
            <li
                class="drift-row flex gap-2.5 py-2 border-b border-surface-z2 text-sm"
                class:drifted={item.status === "drifted"}
                class:broken={item.status === "broken"}
            >
                <span class="status-dot w-2 h-2 rounded-full mt-1 shrink-0"
                ></span>
                <span class="flex-1">{item.detail ?? item.status}</span>
            </li>
        {/each}
    </ul>
</div>

<style>
    .drift-row:last-child {
        border-bottom: none;
    }
    .status-dot {
        background: oklch(var(--color-success-z5) / 1);
    }
    .drift-row.drifted .status-dot {
        background: oklch(var(--color-warning-z5) / 1);
    }
    .drift-row.broken .status-dot {
        background: oklch(var(--color-primary-z5) / 1);
    }
</style>
