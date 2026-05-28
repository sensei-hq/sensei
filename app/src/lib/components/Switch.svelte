<script lang="ts">
    import { Switch as RokkitSwitch } from '@rokkit/ui';

    let {
        value = $bindable(false),
        label = '',
    }: {
        value?: boolean;
        label?: string;
    } = $props();

    // Rokkit's Switch reads each option's label independently. We have a
    // single "Enable X" string regardless of state, so we put the same label
    // on both options. aria-label then resolves to it.
    const options = $derived([
        { value: false, label },
        { value: true,  label },
    ] as const);
</script>

<RokkitSwitch options={options as any} bind:value size="md" />
