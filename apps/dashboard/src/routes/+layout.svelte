<script lang="ts">
  import 'uno.css'
  import '../app.css'
  import { setContext, onMount } from 'svelte'
  import { page } from '$app/stores'
  import { invalidateAll } from '$app/navigation'
  import { vibe } from '@rokkit/states'
  import { themable } from '@rokkit/actions'
  import { kavach as kavachInstance, logger } from '$kavach/auth'

  let { children } = $props()

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const kavach = $state<Record<string, any>>({})
  setContext('kavach', kavach)

  onMount(() => {
    kavachInstance.configure({ invalidateAll, logger })
    Object.assign(kavach, kavachInstance)
    kavachInstance.onAuthChange()
  })
</script>

<svelte:head>
  <title>Sensei</title>
</svelte:head>
<svelte:body use:themable={{ theme: vibe, storageKey: 'sensei-theme' }} />

{@render children()}
