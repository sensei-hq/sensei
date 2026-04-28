<script lang="ts">
  // Instruments page — recommended MCP servers for the detected stack
  // TODO: Load from daemon API after scan/index
</script>

<div class="step">
  <p class="step-desc">Recommended MCP servers for your stack. Toggle to enable data sources and tools.</p>
  <div class="placeholder">
    <span class="kanji placeholder-icon">具</span>
    <p>Instrument recommendations will appear after indexing.</p>
  </div>
</div>

<style>
  .step { max-width: 780px; }
  .step-desc { font-size: 14px; color: var(--sumi-3); line-height: 1.6; margin: 0 0 24px; }
  .placeholder {
    text-align: center; padding: var(--space-12); background: var(--paper-2);
    border-radius: var(--radius-lg); border: var(--border-card);
  }
  .placeholder-icon { font-size: 48px; color: var(--shu); opacity: 0.2; display: block; margin-bottom: var(--space-4); }
  .placeholder p { font-size: 13px; color: var(--sumi-3); }
</style>
