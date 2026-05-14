<script lang="ts">
  let { title, subtitle } = $props<{ title: string; subtitle?: string }>();
</script>

<div class="step-header">
  <h2>{title}</h2>
  {#if subtitle}<p>{subtitle}</p>{/if}
</div>

<style>
  .step-header { padding: 1rem; }
</style>
