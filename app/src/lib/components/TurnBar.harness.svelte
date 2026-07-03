<script lang="ts">
  import TurnBar from './TurnBar.svelte';

  let {
    turns,
    corrections,
    width,
    height,
  }: {
    turns: number;
    corrections: number;
    width?: number;
    height?: number;
  } = $props();
</script>

<TurnBar {turns} {corrections} {width} {height} />
