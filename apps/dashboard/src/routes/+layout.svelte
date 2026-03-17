<script lang="ts">
  import 'uno.css'
  import '../app.css'
  import { vibe } from '@rokkit/states'
  import { themable } from '@rokkit/actions'
  import { page } from '$app/stores'

  let { children } = $props()

  // Root and option-* routes are full-screen (no layout chrome)
  const isStandalone = $derived(
    $page.url.pathname === '/' || $page.url.pathname.startsWith('/option-')
  )
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
