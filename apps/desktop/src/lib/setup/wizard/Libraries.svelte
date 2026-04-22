<script lang="ts">
  import type { WizardState, WizUpdate } from '../types.js';
  import { MOCK_LIBRARIES } from '../mock.js';

  let { wizState, update }: {
    wizState: WizardState;
    update: WizUpdate;
  } = $props();

  const libraries = MOCK_LIBRARIES;

  const detectedCount = $derived(libraries.length);
  const wrappedCount = $derived(
    libraries.filter(l => wizState.libraries[l.id]).length
  );

  function toggle(id: string) {
    update({ libraries: { ...wizState.libraries, [id]: !wizState.libraries[id] } });
  }

  const DOC_LABELS: Record<string, string> = {
    indexed: 'docs indexed',
    partial: 'partial',
    schema: 'schema only',
    none: 'no docs',
  };
</script>

<section class="step">
  <div class="step-label"><span class="kanji">七</span> STEP</div>
  <h1 class="display headline">Libraries</h1>
  <p class="subtitle">
    Libraries without their own MCP — sensei indexes docs &amp; code and wraps
    them with its own tools. Anything with a proper MCP (like Postgres or Stripe)
    comes in the next step.
  </p>

  <!-- Filter chips -->
  <div class="chips">
    <span class="chip">{detectedCount} detected</span>
    <span class="chip chip-active">{wrappedCount} will be wrapped</span>
  </div>

  <!-- Section label -->
  <div class="section-heading">Detected &middot; sensei will wrap</div>

  <!-- Library list -->
  <div class="lib-list">
    {#each libraries as lib}
      {@const checked = !!wizState.libraries[lib.id]}
      <button class="lib-row" onclick={() => toggle(lib.id)}>
        <!-- Checkbox -->
        <div class="checkbox" class:checked>
          {#if checked}<span class="check-mark">&#10003;</span>{/if}
        </div>

        <!-- Main info -->
        <div class="lib-info">
          <div class="lib-name-row">
            <span class="lib-name">{lib.name}</span>
            <span class="version-badge">{lib.version}</span>
          </div>
          <div class="lib-why">{lib.why}</div>
        </div>

        <!-- Right meta -->
        <div class="lib-meta">
          <span class="meta-lang">{lib.lang}</span>
          <span class="meta-usage">{lib.usage}+ uses</span>
          <span class="doc-badge" class:doc-indexed={lib.docs === 'indexed'} class:doc-partial={lib.docs === 'partial'} class:doc-faded={lib.docs === 'schema' || lib.docs === 'none'}>
            {DOC_LABELS[lib.docs]}
          </span>
        </div>
      </button>
    {/each}
  </div>

  <!-- Add library button -->
  <button class="add-btn">+ Add a library</button>
</section>

<style>
  .step {
    padding: var(--space-10) var(--space-12);
    max-width: 780px;
  }

  .step-label {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
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
    margin: 0 0 var(--space-6) 0;
    line-height: 1.55;
    max-width: 620px;
  }

  /* ── Filter chips ───────────────────────────────────────── */
  .chips {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  .chip {
    font-size: 12px;
    font-weight: 500;
    padding: 5px 14px;
    border-radius: 20px;
    border: var(--hairline);
    color: var(--sumi-2);
    background: var(--paper);
  }

  .chip-active {
    border-color: var(--sumi-4);
    color: var(--sumi);
  }

  /* ── Section heading ────────────────────────────────────── */
  .section-heading {
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin-bottom: var(--space-4);
  }

  /* ── Library list ───────────────────────────────────────── */
  .lib-list {
    display: flex;
    flex-direction: column;
  }

  .lib-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-4);
    border-top: var(--hairline);
    background: transparent;
    border-left: none;
    border-right: none;
    border-bottom: none;
    text-align: left;
    cursor: pointer;
    font-family: var(--font-ui);
    transition: background 0.12s;
  }

  .lib-row:last-child {
    border-bottom: var(--hairline);
  }

  .lib-row:hover {
    background: var(--paper-2);
  }

  /* ── Checkbox ───────────────────────────────────────────── */
  .checkbox {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    border: 2px solid var(--paper-edge);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 2px;
    transition: all 0.12s;
  }

  .checkbox.checked {
    background: var(--shu);
    border-color: var(--shu);
  }

  .check-mark {
    font-size: 12px;
    color: white;
    line-height: 1;
    font-weight: 600;
  }

  /* ── Library info ───────────────────────────────────────── */
  .lib-info {
    flex: 1;
    min-width: 0;
  }

  .lib-name-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .lib-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--sumi);
  }

  .version-badge {
    font-size: 11px;
    color: var(--sumi-3);
    font-family: var(--font-mono);
    background: var(--paper-3);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .lib-why {
    font-size: 13px;
    color: var(--sumi-3);
    margin-top: 2px;
  }

  /* ── Right meta ─────────────────────────────────────────── */
  .lib-meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-shrink: 0;
    margin-top: 3px;
  }

  .meta-lang {
    font-size: 12px;
    color: var(--sumi-2);
  }

  .meta-usage {
    font-size: 12px;
    color: var(--sumi-3);
  }

  .doc-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 4px;
    white-space: nowrap;
  }

  .doc-indexed {
    color: var(--jade);
    border: 1px solid var(--jade);
  }

  .doc-partial {
    color: var(--amber);
    border: 1px solid var(--amber);
  }

  .doc-faded {
    color: var(--sumi-4);
    border: var(--border-card);
  }

  /* ── Add button ─────────────────────────────────────────── */
  .add-btn {
    margin-top: var(--space-6);
    font-size: 13px;
    color: var(--sumi-2);
    background: none;
    border: 1px dashed var(--paper-edge);
    border-radius: var(--radius);
    padding: var(--space-3) var(--space-5);
    cursor: pointer;
    font-family: var(--font-ui);
    transition: all 0.12s;
  }

  .add-btn:hover {
    border-color: var(--sumi-4);
    color: var(--sumi);
  }
</style>
