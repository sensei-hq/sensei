<script lang="ts">
  import { goto } from '$app/navigation'
  import { AuthProvider } from '@kavach/ui'
  import { providers } from '$kavach/providers'

  function onSuccess() {
    goto('/')
  }
</script>

<div class="flex min-h-screen flex-col items-center justify-center bg-surface-z0 px-6">
  <div class="w-full max-w-sm">
    <div class="mb-8 text-center">
      <h1 class="text-surface-z9 text-2xl font-black">Sign in to Sensei</h1>
      <p class="text-surface-z6 mt-1 text-sm">Powered by kavach</p>
    </div>

    <div class="flex flex-col gap-4 rounded-2xl border border-surface-z3 bg-surface-z1 p-6">
      {#each providers as p (p.name)}
        <AuthProvider
          name={p.name}
          mode={p.mode ?? 'oauth'}
          onsuccess={onSuccess}
          label={p.label}
        />
      {/each}
    </div>
  </div>
</div>
