<script lang="ts">
  import 'uno.css'
  import '../app.css'
  import { vibe } from '@rokkit/states'
  import { themable } from '@rokkit/actions'
  import { page } from '$app/stores'
  import { setContext, onMount } from 'svelte'

  let { children } = $props()

  // Root and option-* routes are full-screen (no layout chrome)
  const isStandalone = $derived(
    $page.url.pathname === '/' || $page.url.pathname.startsWith('/option-')
  )

  // Provide kavach instance to all child components via Svelte context
  const kavach = $state({})
  setContext('kavach', kavach)

  onMount(async () => {
    const { createKavach } = await import('kavach')
    const { adapter, logger } = await import('$kavach/auth')
    const { invalidateAll } = await import('$app/navigation')
    const instance = createKavach(adapter, { logger, invalidateAll })
    Object.assign(kavach, instance)
    instance.onAuthChange($page.url)
  })
</script>

<svelte:head>
  <title>Sensei</title>
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

<div class="flex h-screen w-screen flex-col overflow-hidden bg-surface-z1 text-surface-z8">
  <main class="min-h-0 flex-1 overflow-auto {isStandalone ? '' : 'p-6'}">
    {@render children()}
  </main>
</div>
