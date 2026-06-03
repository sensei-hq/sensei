<script lang="ts">
  interface Props {
    size?: number;
    tone?: 'accent' | 'success' | 'ink';
  }
  let { size = 10, tone = 'accent' }: Props = $props();

  const toneClass = $derived(
    tone === 'success' ? 'border-success'
    : tone === 'ink'   ? 'border-ink'
    :                    'border-accent',
  );
</script>

<span
  data-component="spinner"
  class="block rounded-full border-[1.5px] border-t-transparent {toneClass}"
  style="width: {size}px; height: {size}px; animation: spinner-spin 0.9s linear infinite;"
  aria-hidden="true"
></span>

<style>
  @keyframes spinner-spin {
    to { transform: rotate(360deg); }
  }
</style>
