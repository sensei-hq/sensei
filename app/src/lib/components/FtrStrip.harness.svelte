<script lang="ts">
  import FtrStrip from './FtrStrip.svelte';

  let {
    data,
    width,
    height,
    dim,
  }: {
    data: { rate: number }[];
    width?: number;
    height?: number;
    dim?: boolean;
  } = $props();
</script>

<FtrStrip {data} {width} {height} {dim} />
