<script lang="ts">
  let { children } = $props();
</script>

<div class="project-window">
  {@render children()}
</div>

<style>
  .project-window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
</style>
