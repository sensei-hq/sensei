<script lang="ts">
  // Observatory — "Today" daily view
  // Placeholder until the full mockup is implemented.
</script>

<div class="observatory">
  <div class="greeting">
    <p class="date-label">{new Date().toLocaleDateString('en-US', { weekday: 'short', day: 'numeric', month: 'short' })}</p>
    <h1 class="display hero">The observatory is open.</h1>
  </div>

  <div class="hero-card">
    <span class="kanji hero-kanji">観</span>
    <div class="hero-body">
      <p class="display hero-koan">Still listening.</p>
      <p class="hero-text">
        Sensei is watching your sessions. A few early signals are forming,
        but nothing confident enough to teach yet.
      </p>
    </div>
  </div>

  <p class="hint">
    Start a session with your assistant. Sensei watches in silence,
    learns the shape of each project, and later begins to teach.
  </p>
</div>

<style>
  .observatory {
    max-width: 720px;
    margin: 0 auto;
    padding: 48px 48px 64px;
  }
  .greeting {
    margin-bottom: 40px;
  }
  .date-label {
    font-size: 10.5px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin: 0 0 8px;
  }
  .hero {
    font-size: 28px;
    font-weight: 400;
    margin: 0;
    letter-spacing: -0.01em;
  }

  /* ── Hero card ────────────────────────────────────────── */
  .hero-card {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 28px;
    padding: 32px 34px 30px;
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius-lg);
    margin-bottom: 32px;
  }
  .hero-kanji {
    font-size: 72px;
    color: var(--shu);
    opacity: 0.55;
    line-height: 1;
  }
  .hero-koan {
    font-size: 24px;
    font-weight: 400;
    margin: 0 0 12px;
    letter-spacing: -0.01em;
  }
  .hero-text {
    font-size: 14px;
    color: var(--sumi-2);
    line-height: 1.65;
    margin: 0;
    max-width: 520px;
  }

  .hint {
    font-size: 12px;
    color: var(--sumi-3);
    line-height: 1.7;
    max-width: 480px;
  }
</style>
