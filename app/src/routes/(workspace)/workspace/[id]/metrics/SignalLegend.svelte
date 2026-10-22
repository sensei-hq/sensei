<script lang="ts">
    // The trend legend for the "all signals" grid: improving · worsening · flat,
    // matching the coloured top-rule on each cell.
    const ITEMS = [
        { label: 'improving', cls: 'bg-success' },
        { label: 'worsening', cls: 'bg-accent' },
        { label: 'flat', cls: 'bg-ink-faint' },
    ];
</script>

<div data-component="signal-legend" class="flex items-center gap-4 text-xs text-ink-mute">
    {#each ITEMS as item (item.label)}
        <span class="flex items-center gap-1">
            <span class="inline-block w-3 h-0.5 {item.cls}"></span>{item.label}
        </span>
    {/each}
</div>
