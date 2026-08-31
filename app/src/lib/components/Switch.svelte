<script lang="ts">
    import { Switch as RokkitSwitch } from '@rokkit/ui';

    let {
        value = $bindable(false),
        label = '',
        disabled = false,
        onchange = undefined,
    }: {
        value?: boolean;
        label?: string;
        /** Blocks interaction — e.g. while a write is in flight, or when the
         *  underlying decision cannot be made at all. */
        disabled?: boolean;
        /** Fires with the new value after a toggle. For a switch whose truth
         *  lives on a server: bind nothing, pass `value` one-way, and let the
         *  handler adopt whatever the server rules. */
        onchange?: (next: boolean) => void;
    } = $props();

    // Rokkit's Switch reads each option's label independently. We have a
    // single "Enable X" string regardless of state, so we put the same label
    // on both options. aria-label then resolves to it.
    const options = $derived([
        { value: false, label },
        { value: true,  label },
    ] as const);
</script>

<RokkitSwitch
    options={options as any}
    bind:value
    size="md"
    {disabled}
    onchange={(next) => onchange?.(next as boolean)}
/>
