<script lang="ts">
  import Eyebrow from './Eyebrow.svelte';

  let { label, tone }: { label: string; tone?: string } = $props();
</script>

<Eyebrow {tone}>{label}</Eyebrow>
