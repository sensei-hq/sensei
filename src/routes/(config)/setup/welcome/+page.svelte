<script lang="ts">
  const pillars = [
    { kanji: '観', title: 'Observe', sub: 'FTR \u00b7 turns \u00b7 corrections' },
    { kanji: '師', title: 'Teach',   sub: 'patterns \u00b7 rules \u00b7 skills' },
    { kanji: '静', title: 'Local',   sub: 'on your machine' },
  ];
</script>

<div class="welcome">
  <h1 class="display hero">
    A teacher does not<br/>
    <span class="hero-accent">write the code.</span>
  </h1>

  <p class="body-primary">
    Sensei watches how you and your AI assistants work together — the sessions that
    completed cleanly, the ones that didn't, and the patterns underneath both.
  </p>

  <p class="body-secondary">
    The next few minutes: install the local components, point to your folders, confirm
    what was found. Nothing leaves your machine.
  </p>

  <div class="pillars">
    {#each pillars as pillar}
      <div class="pillar">
        <div class="kanji pillar-kanji">{pillar.kanji}</div>
        <div class="display pillar-title">{pillar.title}</div>
        <div class="pillar-sub">{pillar.sub}</div>
      </div>
    {/each}
  </div>
</div>

<style>
  .welcome { max-width: 720px; }

  .hero {
    font-size: 54px; font-weight: 300; line-height: 1.08;
    margin: 0 0 32px; letter-spacing: -0.02em;
  }
  .hero-accent { color: var(--shu); }

  .body-primary {
    font-size: 16px; color: var(--sumi-2); line-height: 1.7;
    max-width: 560px; margin: 0 0 28px;
  }
  .body-secondary {
    font-size: 14px; color: var(--sumi-3); line-height: 1.7;
    max-width: 560px; margin: 0 0 44px;
  }

  .pillars {
    display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 18px;
    padding: 22px 0; border-top: var(--hairline); border-bottom: var(--hairline);
  }
  .pillar-kanji { font-size: 26px; color: var(--shu); margin-bottom: 8px; }
  .pillar-title { font-size: 20px; }
  .pillar-sub { font-size: 12px; color: var(--sumi-3); margin-top: 3px; }
</style>
