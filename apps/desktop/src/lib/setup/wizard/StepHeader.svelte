<script lang="ts">
  import type { WizStage } from '../types.js';

  let { stage, subtitle = '' }: {
    stage: WizStage;
    subtitle?: string;
  } = $props();
</script>

<header class="step-header">
  <div class="step-label"><span class="kanji">{stage.n}</span> STEP</div>
  <h1 class="display headline">{stage.title}</h1>
  {#if subtitle}
    <p class="subtitle">{subtitle}</p>
  {/if}
</header>

<style>
  .step-header {
    margin-bottom: var(--space-8);
  }
  .step-label {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: var(--space-2);
  }
  .step-label .kanji {
    color: var(--shu);
    margin-right: 4px;
  }
  .headline {
    font-size: 40px;
    color: var(--sumi);
    margin: 0 0 var(--space-2) 0;
    line-height: 1.15;
  }
  .subtitle {
    font-size: 15px;
    color: var(--sumi-3);
    margin: 0;
    line-height: 1.5;
  }
</style>
