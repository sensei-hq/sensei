<script lang="ts">
  import { goto } from '$app/navigation';
  import { healthState } from '$lib/health-state.svelte.js';
  import HealthView from './HealthView.svelte';

  function onEnter() { goto('/', { replaceState: true }); }
  function onRecheck() { /* Phase 1a: no-op; Phase 2 wires healthState.recheck() */ }
  function onCopyScript() {
    if (healthState.remedy) navigator.clipboard?.writeText(healthState.remedy.script);
  }
</script>

<HealthView state={healthState} {onEnter} {onRecheck} {onCopyScript} />
