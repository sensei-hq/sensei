<script lang="ts">
  import { setContext, onMount } from 'svelte'
  import { page } from '$app/stores'
  import { invalidateAll } from '$app/navigation'
  import { createKavach } from 'kavach'
  import  { adapter, logger } from '$kavach/auth'

  let { children } = $props()

  // Provide kavach instance to all child components via Svelte context
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const kavach = $state<Record<string, any>>({})
  setContext('kavach', kavach)

  onMount(async () => {


    const instance = createKavach(adapter, { logger, invalidateAll })
    Object.assign(kavach, instance)
    instance.onAuthChange($page.url)
  })
</script>

{@render children()}
