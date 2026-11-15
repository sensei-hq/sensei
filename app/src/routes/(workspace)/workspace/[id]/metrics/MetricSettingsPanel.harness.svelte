<script lang="ts">
  import MetricSettingsPanel from './MetricSettingsPanel.svelte';
  import type { MetricSettingsState } from './metric-settings-state.svelte.js';

  let { panel }: { panel: MetricSettingsState } = $props();
</script>

<MetricSettingsPanel {panel} />
