<script lang="ts">
    let { data } = $props();
</script>

<div class="px-6 py-6">
    <h2 class="text-xl font-normal m-0 mb-4">Patterns</h2>
    {#if data.followed.length > 0}
        <section class="mb-5">
            <h3
                class="text-xs font-semibold opacity-60 m-0 mb-2 uppercase tracking-wider"
            >
                Followed ({data.followed.length})
            </h3>
            <ul class="list-none m-0 p-0">
                {#each data.followed as p (p.id)}
                    <li
                        class="pattern-row flex gap-3 py-1.75 border-b border-surface-z2 text-ui"
                    >
                        <span class="flex-1">{p.name}</span>
                        <span class="text-2xs opacity-50 font-mono"
                            >{p.lifecycle}</span
                        >
                    </li>
                {/each}
            </ul>
        </section>
    {/if}
    {#if data.antiPatterns.length > 0}
        <section>
            <h3
                class="text-xs font-semibold m-0 mb-2 uppercase tracking-wider"
                style="color: oklch(var(--color-primary-z5) / 1);"
            >
                Anti-patterns ({data.antiPatterns.length})
            </h3>
            <ul class="list-none m-0 p-0">
                {#each data.antiPatterns as p (p.id)}
                    <li
                        class="pattern-row flex gap-3 py-1.75 border-b border-surface-z2 text-ui opacity-80"
                    >
                        <span class="flex-1">{p.name}</span>
                        <span class="text-2xs opacity-50 font-mono"
                            >{p.lifecycle}</span
                        >
                    </li>
                {/each}
            </ul>
        </section>
    {/if}
</div>

<style>
    .pattern-row:last-child {
        border-bottom: none;
    }
</style>
