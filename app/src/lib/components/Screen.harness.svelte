<script lang="ts">
  import Screen from './Screen.svelte';

  let { withHeader = true }: { withHeader?: boolean } = $props();
</script>

{#if withHeader}
  <Screen>
    {#snippet header()}<h1>Title</h1>{/snippet}
    <p>Body</p>
  </Screen>
{:else}
  <Screen><p>Body</p></Screen>
{/if}
