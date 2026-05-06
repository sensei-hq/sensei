<script lang="ts">
    let { data } = $props();
</script>

<div class="px-6 py-6">
    <header class="flex items-baseline gap-4 mb-5">
        <h2 class="text-xl font-normal m-0">Instruments</h2>
        <span class="text-ui opacity-60">{data.tools.length} tools</span>
    </header>

    {#if data.tools.length === 0}
        <p class="text-ui opacity-50">
            No instruments associated with this project yet.
        </p>
    {:else}
        <ul class="list-none m-0 p-0">
            {#each data.tools as tool (tool.id)}
                <li
                    class="tool-row flex items-center gap-2.5 py-2 border-b border-surface-z2 text-ui"
                >
                    <span class="font-semibold flex-1">{tool.name}</span>
                    <span class="opacity-50 text-xs">{tool.kind}</span>
                    <span
                        class="scope-badge text-2xs px-1.5 py-px rounded-md font-mono"
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
