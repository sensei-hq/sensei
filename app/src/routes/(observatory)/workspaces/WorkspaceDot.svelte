<script lang="ts">
  import { workspaceDotView } from './buckets.js';

  // The status dot beside a workspace name: the shared FTR band, unless a live
  // warning wins (workspaceDotView). Renders; decides nothing.
  let { ftr, warn }: { ftr: number | null; warn: boolean } = $props();

  const view = $derived(workspaceDotView(ftr, warn));
</script>

<span
  data-component="workspace-dot"
  role="status"
  aria-label={view.label}
  class="inline-block w-[7px] h-[7px] rounded-full shrink-0 {view.dot}"
></span>
