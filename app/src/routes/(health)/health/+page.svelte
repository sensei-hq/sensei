<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { healthState } from '$lib/health-state.svelte.js';
  import HealthView from './HealthView.svelte';

  onMount(() => { healthState.init(); });

  function onEnter()    { goto('/', { replaceState: true }); }
  function onVerify()   { healthState.verify(); }
  function onCopyScript() {
    if (healthState.remedy) navigator.clipboard?.writeText(healthState.remedy.script);
  }
</script>

<HealthView state={healthState} {onEnter} {onVerify} {onCopyScript} />
