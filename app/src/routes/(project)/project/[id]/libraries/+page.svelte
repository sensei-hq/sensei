<script lang="ts">
    import { PageHeader } from '$lib/components';
    let { data } = $props();
</script>

<PageHeader title="Libraries">
    {#snippet right()}
        <div class="text-sm text-ink-z6">
            <span>{data.wrappedCount} wrapped</span>
            <span class="opacity-40"> · </span>
            <span>{data.unwrappedCount} unwrapped</span>
        </div>
    {/snippet}
</PageHeader>
<div class="px-6 py-6">

    {#if data.libraries.length === 0}
        <p class="text-sm opacity-50">
            No libraries associated with this project yet.
        </p>
    {:else}
        <ul class="list-none m-0 p-0">
            {#each data.libraries as lib (lib.id)}
                <li
                    class="lib-row flex items-center gap-2.5 py-2 border-b border-surface-z2 text-sm"
                >
                    <span class="font-semibold flex-1">{lib.name}</span>
                    <span class="opacity-50 text-xs">{lib.ecosystem}</span>
                    <span
                        class="scope-badge text-xs px-1.5 py-px rounded-md font-mono"
                        class:global={lib.scope === "global"}
                        class:proj={lib.scope === "project"}
                    >
                        [{lib.scope}]
                    </span>
                </li>
            {/each}
        </ul>
    {/if}
</div>

<style>
    .lib-row:last-child {
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
