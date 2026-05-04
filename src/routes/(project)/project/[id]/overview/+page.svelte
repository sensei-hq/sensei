<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Overview — {data.project?.name}</h2>
  <p class="hint">FTR 14d: {Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100)}%</p>
</div>
<style>
  .section-page { padding: 24px; }
  .hint { opacity: 0.5; font-size: 13px; }
</style>
