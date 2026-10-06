<script lang="ts">
  import ToggleChip from './ToggleChip.svelte';

  let {
    label,
    pressed = false,
    size,
    testid,
  }: { label: string; pressed?: boolean; size?: 'sm' | 'xs'; testid?: string } = $props();

  let presses = $state(0);
</script>

<ToggleChip {label} {pressed} {size} {testid} onpress={() => (presses += 1)} />
<output data-testid="presses">{presses}</output>
