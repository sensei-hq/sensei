<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { ScreenState } from '$lib/components';
  import AtlasScreen from './AtlasScreen.svelte';

  let { data } = $props();
</script>

{#if data.atlas}
  <AtlasScreen data={data.atlas} />
{:else}
  <div class="p-8">
    <ScreenState status="error" error={data.error} onretry={() => invalidateAll()} />
  </div>
{/if}
