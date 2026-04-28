<script lang="ts">
  const stats = [
    { kanji: '観', label: 'Observing', sub: 'sessions tracked in real time' },
    { kanji: '師', label: 'Teaching', sub: 'after ~3 sessions per project' },
    { kanji: '静', label: 'Local', sub: 'nothing leaves your machine' },
  ];
</script>

<div class="done">
  <h1 class="display hero">
    The observatory<br/>
    <span class="hero-accent">is ready.</span>
  </h1>

  <p class="body">
    Sensei is watching. Work normally with your AI assistants — sensei learns
    from every session and begins teaching after it has enough data.
  </p>

  <div class="pillars">
    {#each stats as s}
      <div class="pillar">
        <div class="kanji pillar-kanji">{s.kanji}</div>
        <div class="display pillar-title">{s.label}</div>
        <div class="pillar-sub">{s.sub}</div>
      </div>
    {/each}
  </div>
</div>

<style>
  .done { max-width: 720px; }
  .hero { font-size: 54px; font-weight: 300; line-height: 1.08; margin: 0 0 32px; letter-spacing: -0.02em; }
  .hero-accent { color: var(--shu); }
  .body { font-size: 16px; color: var(--sumi-2); line-height: 1.7; max-width: 560px; margin: 0 0 44px; }
  .pillars {
    display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 18px;
    padding: 22px 0; border-top: var(--hairline); border-bottom: var(--hairline);
  }
  .pillar-kanji { font-size: 26px; color: var(--shu); margin-bottom: 8px; }
  .pillar-title { font-size: 20px; }
  .pillar-sub { font-size: 12px; color: var(--sumi-3); margin-top: 3px; }
</style>
