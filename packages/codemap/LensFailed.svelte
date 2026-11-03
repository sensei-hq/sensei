<script lang="ts">
  // A lens whose own read failed (review r2 N1): why, and a way to try again —
  // never "Reading…" beside an error that has already happened.
  import { Button } from '@rokkit/ui';

  let { reason, onretry }: { reason: string; onretry: () => void } = $props();
</script>

<div class="flex flex-col items-start gap-2" data-testid="lens-failed">
  <p class="m-0 text-sm text-danger">Could not read this lens: {reason}</p>
  <Button size="sm" style="outline" icon="i-solar-refresh-bold-duotone" label="Retry" onclick={onretry} />
</div>
