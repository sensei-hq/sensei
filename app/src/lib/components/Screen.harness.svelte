<script lang="ts">
  import Screen from './Screen.svelte';

  let { withHeader = true, fill = false }: { withHeader?: boolean; fill?: boolean } = $props();
</script>

{#if withHeader}
  <Screen {fill} data-screen="probe" data-extra="x">
    {#snippet header()}<h1>Title</h1>{/snippet}
    <p>Body</p>
  </Screen>
{:else}
  <Screen><p>Body</p></Screen>
{/if}
