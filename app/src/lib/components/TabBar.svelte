<script lang="ts">
    let {
        tabs,
        active = $bindable(),
        class: className = "",
    }: {
        tabs: [string, string][];
        active: string;
        class?: string;
    } = $props();
</script>

<div class="flex gap-0 border-b border-surface-z2 {className}">
    {#each tabs as [key, label]}
        <button
            class="tab px-4 py-2 border-none bg-none text-sm cursor-pointer border-b-2 border-transparent -mb-px text-surface-z6"
            class:active={active === key}
            onclick={() => (active = key)}>{label}</button
        >
    {/each}
</div>

<style>
    .tab:hover {
        color: oklch(var(--color-surface-z8) / 1);
    }
    .tab.active {
        color: oklch(var(--color-surface-z9) / 1);
        border-bottom-color: oklch(var(--color-primary-z5) / 1);
    }
</style>
