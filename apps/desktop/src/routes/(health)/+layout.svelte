<script lang="ts">
  let { children } = $props();
</script>

<div class="health-shell">
  <div class="drag-region drag-spacer"></div>
  <main class="health-main">
    {@render children()}
  </main>
</div>

<style>
  .health-shell {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--paper);
    font-family: var(--font-ui);
    color: var(--sumi);
    overflow: hidden;
  }
  .drag-spacer {
    height: 32px;
    flex-shrink: 0;
  }
  .health-main {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 64px;
  }
</style>
