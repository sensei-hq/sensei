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

<div class="flex gap-0 border-b border-paper-edge {className}">
    {#each tabs as [key, label]}
        <button
            class="tab px-4 py-2 border-none bg-none text-sm cursor-pointer border-b-2 border-transparent -mb-px text-ink-soft"
            class:active={active === key}
            onclick={() => (active = key)}>{label}</button
        >
    {/each}
</div>

<style>
    .tab:hover {
        color: var(--ink-mute);
    }
    .tab.active {
        color: var(--ink);
        border-bottom-color: var(--accent);
    }
</style>
