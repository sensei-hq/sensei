<script lang="ts">
  import { onMount } from 'svelte';

  let metrics = $state({ ftr: 0, sessions: 0 });

  onMount(async () => {
    const res = await fetch('/api/metrics');
    metrics = await res.json();
  });
</script>

<div class="observatory">
  <h1>Observatory</h1>
  <p>FTR: {metrics.ftr}%</p>
  <p>Sessions: {metrics.sessions}</p>
</div>
