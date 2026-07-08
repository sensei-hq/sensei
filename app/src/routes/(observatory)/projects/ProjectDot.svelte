<script lang="ts">
  // The status dot beside a project name. Colour is driven by the same
  // rule the mockup uses: a live warning wins, then FTR bands — ≥0.8 healthy
  // (success), ≥0.6 neutral (ink-mute), below that back to warning.
  let { ftr, warn }: { ftr: number; warn: boolean } = $props();

  const toneClass = $derived(
    warn ? 'bg-warning'
    : ftr >= 0.8 ? 'bg-success'
    : ftr >= 0.6 ? 'bg-ink-mute'
    : 'bg-warning',
  );

  const label = $derived(
    warn || ftr < 0.6 ? 'needs attention' : ftr >= 0.8 ? 'healthy' : 'ok',
  );
</script>

<span
  data-component="project-dot"
  role="status"
  aria-label={label}
  class="inline-block w-[7px] h-[7px] rounded-full shrink-0 {toneClass}"
></span>
