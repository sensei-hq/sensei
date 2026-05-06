<script lang="ts">
    let {
        value = $bindable(false),
        label = "",
    }: {
        value?: boolean;
        label?: string;
    } = $props();
</script>

<button
    class="switch"
    class:on={value}
    aria-label={label}
    onclick={() => (value = !value)}
>
    <span class="switch-thumb"></span>
</button>

<style>
    .switch {
        width: 36px;
        height: 20px;
        border-radius: 999px;
        position: relative;
        background: oklch(var(--color-surface-z3));
        cursor: pointer;
        padding: 0;
        border: none;
        transition: background 0.15s;
        flex-shrink: 0;
    }
    .switch.on {
        background: oklch(var(--color-primary-600) / 1);
    }
    .switch-thumb {
        position: absolute;
        top: 2px;
        left: 2px;
        width: 16px;
        height: 16px;
        border-radius: 50%;
        background: oklch(var(--color-surface-z1));
        transition: left 0.18s ease;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18);
    }
    .switch.on .switch-thumb {
        left: 18px;
    }
</style>
