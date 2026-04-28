<script lang="ts">
  // Projects page — confirm/edit auto-detected project groupings
  // TODO: Wire to ScanProjectState from scan page (shared state)
  // For now, placeholder until daemon SSE is implemented
</script>

<div class="step">
  <p class="step-desc">Review and confirm the detected project groupings. Merge, rename, or reorganize as needed.</p>
  <div class="placeholder">
    <span class="kanji placeholder-icon">場</span>
    <p>Projects will appear here after scan completes.</p>
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
