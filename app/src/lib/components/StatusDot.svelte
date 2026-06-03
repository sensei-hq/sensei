<script lang="ts">
  type Status = 'ok' | 'busy' | 'warn' | 'fail' | 'idle';
  type Size = 'sm' | 'md' | 'lg';

  let {
    status,
    size = 'md',
    label,
  }: { status: Status; size?: Size; label?: string } = $props();

  const sizeClass = $derived(
    ({ sm: 'w-1.5 h-1.5', md: 'w-2 h-2', lg: 'w-2.5 h-2.5' })[size],
  );

  const toneClass = $derived(
    ({
      ok:   'bg-success',
      busy: 'bg-accent',
      warn: 'bg-warning',
      fail: 'bg-accent',
      idle: 'bg-paper-edge',
    })[status],
  );
</script>

{#if label}
  <span
    data-component="status-dot"
    role="status"
    aria-label={label}
    class="inline-block rounded-full {sizeClass} {toneClass}"
  ></span>
{:else}
  <span
    data-component="status-dot"
    aria-hidden="true"
    class="inline-block rounded-full {sizeClass} {toneClass}"
  ></span>
{/if}
