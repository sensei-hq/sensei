<script lang="ts">
  import type { WizardState } from '../types.js';

  let { wizState }: {
    wizState: WizardState;
  } = $props();

  const projectCount = $derived(wizState.projects.filter((p: any) => p.confirmed).length);
  const repoCount = $derived(
    wizState.projects.filter((p: any) => p.confirmed).reduce((sum: number, p: any) => sum + p.repos.length, 0)
  );
  const libCount = $derived(
    Object.values(wizState.libraries).filter(Boolean).length + wizState.libExtras.length
  );
  const mcpCount = $derived(Object.values(wizState.mcps).filter(Boolean).length);
  const assistantCount = $derived(Object.values(wizState.acps).filter(Boolean).length);

  const stats = $derived([
    { label: 'Projects',   value: projectCount },
    { label: 'Repos',      value: repoCount },
    { label: 'Libraries',  value: libCount },
    { label: 'MCPs',       value: mcpCount },
    { label: 'Assistants', value: assistantCount },
  ]);
</script>

<div class="done">
  <!-- Hero kanji -->
  <div class="kanji hero-kanji">観</div>

  <!-- Headline -->
  <h1 class="display hero-title">The observatory is ready.</h1>

  <!-- Body -->
  <p class="hero-body">
    Start a session with your assistant. Sensei will watch in silence for a few days,
    then begin to teach.
  </p>

  <!-- Stats grid -->
  <div class="stats-grid">
    {#each stats as stat, i}
      <div class="stat-cell" class:stat-last={i === stats.length - 1}>
        <div class="display stat-value">{stat.value}</div>
        <div class="stat-label">{stat.label}</div>
      </div>
    {/each}
  </div>

  <!-- Footer quote -->
  <p class="footer-quote">
    — &middot; the first session is always the teacher
  </p>
</div>

<style>
  .done {
    max-width: 680px;
    margin: 20px auto 0;
    text-align: center;
  }

  .hero-kanji {
    font-size: 92px;
    color: var(--shu);
    margin-bottom: 10px;
    line-height: 1;
  }

  .hero-title {
    font-size: 44px;
    font-weight: 300;
    letter-spacing: -0.02em;
    margin: 0 0 16px;
    color: var(--sumi);
  }

  .hero-body {
    font-size: 15px;
    color: var(--sumi-2);
    line-height: 1.6;
    max-width: 480px;
    margin: 0 auto 36px;
    font-family: var(--font-ui);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 0;
    border: var(--hairline);
    border-radius: var(--radius-lg);
    background: var(--paper-2);
    overflow: hidden;
    text-align: left;
  }

  .stat-cell {
    padding: 18px 20px;
    border-right: var(--hairline);
  }

  .stat-last {
    border-right: none;
  }

  .stat-value {
    font-size: 30px;
    font-weight: 400;
    color: var(--sumi);
  }

  .stat-label {
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin-top: 2px;
  }

  .footer-quote {
    font-size: 11px;
    color: var(--sumi-3);
    margin-top: 36px;
    font-style: italic;
    font-family: var(--font-mono);
  }
</style>
