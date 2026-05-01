<script lang="ts">
  // TODO: Wire to daemon settings API
</script>

<div class="stage-placeholder">
  <div class="kanji placeholder-icon">名</div>
  <p class="placeholder-msg">Preferences will be configured here.</p>
</div>

<style>
  .stage-placeholder {
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    min-height: 300px; gap: 16px;
  }
  .placeholder-icon { font-size: 48px; color: var(--shu); opacity: 0.3; }
  .placeholder-msg { font-size: 14px; color: var(--sumi-3); }
</style>
