<script lang="ts">
  let { data } = $props();
</script>

<div class="flex min-h-screen items-center justify-center bg-surface-z1">
  <div class="w-full max-w-sm rounded-2xl border border-surface-z3 bg-surface-z2 p-8 text-center">
    <div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-primary-z6 text-xl font-bold text-white">⬡</div>
    {#if data.error}
      <h1 class="text-lg font-semibold text-surface-z8 mb-2">Something went wrong</h1>
      <p class="text-sm text-error-z6">{data.error}</p>
      <p class="mt-4 text-xs text-surface-z4">Run <code class="font-mono">sensei login</code> again to retry.</p>
    {:else}
      <h1 class="text-lg font-semibold text-surface-z8 mb-2">Completing CLI login…</h1>
      <p class="text-sm text-surface-z5">Redirecting back to your terminal.</p>
    {/if}
  </div>
</div>
