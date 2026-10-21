<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { Screen, ScreenState } from '$lib/components';
  import AtlasScreen from './AtlasScreen.svelte';

  let { data } = $props();
</script>

{#if data.atlas}
  <AtlasScreen data={data.atlas} />
{:else}
  <Screen>
    <div class="p-8">
      <ScreenState status="error" error={data.error} onretry={() => invalidateAll()} />
    </div>
  </Screen>
{/if}
