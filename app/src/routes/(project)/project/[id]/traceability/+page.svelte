<script lang="ts">
    import { PageHeader, StatusDot } from '$lib/components';
    let { data } = $props();

    type DriftStatus = 'drifted' | 'broken' | string;
    const dotStatus = (s: DriftStatus): 'ok' | 'warn' | 'fail' =>
        s === 'broken' ? 'fail' : s === 'drifted' ? 'warn' : 'ok';
</script>

<PageHeader title="Traceability">
    {#snippet right()}
        <div class="text-sm text-ink-z7">
            <span>{data.total} tracked</span>
            <span> · </span>
            <span style="color: oklch(var(--color-warning-z5) / 1);">{data.drifted} drifted</span>
            <span> · </span>
            <span style="color: oklch(var(--color-primary-z5) / 1);">{data.broken} broken</span>
        </div>
    {/snippet}
</PageHeader>
<div class="px-6 py-6">
    <ul class="list-none m-0 p-0">
        {#each data.driftItems as item (item.id)}
            <li class="drift-row flex gap-2.5 py-2 border-b border-surface-z2 text-sm items-start">
                <span class="mt-1"><StatusDot status={dotStatus(item.status)} /></span>
                <span class="flex-1">{item.detail ?? item.status}</span>
            </li>
        {/each}
    </ul>
</div>

<style>
    .drift-row:last-child {
        border-bottom: none;
    }
</style>
