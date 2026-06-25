<script lang="ts">
  import ProgressCard from './ProgressCard.svelte';
  let {
    label = 'Installing',
    trailing,
    percent = 0,
    left,
    right,
    activity,
    note,
  }: {
    label?: string;
    trailing?: string;
    percent?: number;
    left?: string;
    right?: string;
    activity?: string;
    note?: string;
  } = $props();
</script>

<ProgressCard {label} {trailing} {percent} {left} {right} {activity} {note} />
