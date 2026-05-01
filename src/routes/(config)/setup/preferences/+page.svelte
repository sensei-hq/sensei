<script lang="ts">
  import { wizardState } from '$lib/wizard-state.svelte.js';

  const prefs = $derived(wizardState.preferences);
</script>

<div class="preferences">
  <!-- ── Stage header (matches WizHeader from mockup) ──── -->
  <div class="wiz-header">
    <div class="wiz-label"><span class="kanji wiz-label-kanji">名</span> Step</div>
    <h1 class="display wiz-title">Preferences</h1>
    <p class="wiz-tagline">A few small choices before you step in. Anything here can be changed later.</p>
  </div>

  <!-- ── Display name ─────────────────────────────────── -->
  <section class="section">
    <header class="section-header">
      <span class="kanji section-kanji">名</span>
      <div class="section-info">
        <h3 class="display section-title">What should sensei call you?</h3>
        <p class="section-sub">Used in the morning digest and when raising a teaching.</p>
      </div>
      <input
        type="text"
        class="name-input"
        value={prefs.displayName}
        oninput={(e) => { wizardState.preferences.displayName = e.currentTarget.value; }}
        placeholder="your name"
      />
    </header>
  </section>

  <!-- ── Shared learnings ───────────────────────────────── -->
  <section class="section">
    <header class="section-header">
      <span class="kanji section-kanji">共</span>
      <div class="section-info">
        <h3 class="display section-title">Shared learnings</h3>
        <p class="section-sub">
          Contribute anonymized patterns to a collective pool — and pull what others have learned.
        </p>
      </div>
    </header>
    <div class="section-body">
      <div class="row">
        <div class="row-text">
          <div class="row-label">Contribute my learnings</div>
          <div class="row-hint">Anonymized patterns only. No code, no commit messages, no project names.</div>
        </div>
        <button class="toggle" class:on={prefs.contributeLearnings} aria-label="Toggle contribute learnings"
                onclick={() => { wizardState.preferences.contributeLearnings = !prefs.contributeLearnings; }}>
          <span class="toggle-thumb"></span>
        </button>
      </div>
      <div class="row">
        <div class="row-text">
          <div class="row-label">Review before sharing</div>
          <div class="row-hint">Each learning shows up in a queue for your approval before it leaves your machine.</div>
        </div>
        <button class="toggle" class:on={prefs.reviewBeforeShare} aria-label="Toggle review before sharing"
                onclick={() => { wizardState.preferences.reviewBeforeShare = !prefs.reviewBeforeShare; }}>
          <span class="toggle-thumb"></span>
        </button>
      </div>
      <div class="row">
        <div class="row-text">
          <div class="row-label">Sharing schedule</div>
        </div>
        <select class="sel" value={prefs.shareSchedule}
                onchange={(e) => { wizardState.preferences.shareSchedule = e.currentTarget.value; }}>
          <option value="off">Off · manual only</option>
          <option value="daily">Daily</option>
          <option value="weekly-saturday">Every Saturday</option>
          <option value="monthly">Monthly</option>
        </select>
      </div>
      <div class="row">
        <div class="row-text">
          <div class="row-label">Download collective learnings</div>
          <div class="row-hint">Reviewed before they enter your library.</div>
        </div>
        <select class="sel" value={prefs.downloadCollective}
                onchange={(e) => { wizardState.preferences.downloadCollective = e.currentTarget.value; }}>
          <option value="never">Never</option>
          <option value="weekly">Weekly</option>
          <option value="daily">Daily</option>
          <option value="on-demand">On demand</option>
        </select>
      </div>
    </div>
  </section>

  <!-- ── Sensei behavior ────────────────────────────────── -->
  <section class="section">
    <header class="section-header">
      <span class="kanji section-kanji">師</span>
      <div class="section-info">
        <h3 class="display section-title">Sensei behavior</h3>
        <p class="section-sub">How forward sensei is — when it nudges, how it phrases corrections.</p>
      </div>
    </header>
    <div class="section-body">
      <div class="row">
        <div class="row-text">
          <div class="row-label">Correction tone</div>
          <div class="row-hint">How direct sensei is when something repeats.</div>
        </div>
        <div class="segment">
          {#each [
            { value: 'gentle', label: 'Gentle' },
            { value: 'balanced', label: 'Balanced' },
            { value: 'direct', label: 'Direct' },
          ] as opt}
            <button
              class="segment-btn"
              class:active={prefs.correctionAggressiveness === opt.value}
              onclick={() => { wizardState.preferences.correctionAggressiveness = opt.value; }}
            >{opt.label}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <div class="row-text">
          <div class="row-label">Morning digest</div>
          <div class="row-hint">The Today view. Off keeps the dashboard quiet.</div>
        </div>
        <div class="segment">
          {#each [
            { value: 'off', label: 'Off' },
            { value: 'daily', label: 'Daily' },
            { value: 'weekly', label: 'Weekly' },
          ] as opt}
            <button
              class="segment-btn"
              class:active={prefs.digestCadence === opt.value}
              onclick={() => { wizardState.preferences.digestCadence = opt.value; }}
            >{opt.label}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <div class="row-text">
          <div class="row-label">Nudge on regression</div>
          <div class="row-hint">If FTR drops sharply on a project, sensei surfaces it on Today.</div>
        </div>
        <button class="toggle" class:on={prefs.nudgeOnRegression} aria-label="Toggle nudge on regression"
                onclick={() => { wizardState.preferences.nudgeOnRegression = !prefs.nudgeOnRegression; }}>
          <span class="toggle-thumb"></span>
        </button>
      </div>
    </div>
  </section>

  <!-- ── Telemetry ──────────────────────────────────────── -->
  <section class="section">
    <header class="section-header">
      <span class="kanji section-kanji">守</span>
      <div class="section-info">
        <h3 class="display section-title">Telemetry</h3>
        <p class="section-sub">
          Help us improve sensei itself — separate from shared learnings, this is about
          the app, not your work.
        </p>
      </div>
    </header>
    <div class="section-body">
      <div class="row">
        <div class="row-text">
          <div class="row-label">Anonymized usage telemetry</div>
          <div class="row-hint">Crashes, performance, which views you visit. Never code, prompts, or session content.</div>
        </div>
        <button class="toggle" class:on={prefs.anonymizedTelemetry} aria-label="Toggle anonymized telemetry"
                onclick={() => { wizardState.preferences.anonymizedTelemetry = !prefs.anonymizedTelemetry; }}>
          <span class="toggle-thumb"></span>
        </button>
      </div>
      <div class="row">
        <div class="row-text">
          <div class="row-label">Show welcome message on first entry</div>
          <div class="row-hint">The greeting toast that appears when you first open the observatory each day.</div>
        </div>
        <button class="toggle" class:on={prefs.showWelcome} aria-label="Toggle welcome message"
                onclick={() => { wizardState.preferences.showWelcome = !prefs.showWelcome; }}>
          <span class="toggle-thumb"></span>
        </button>
      </div>
    </div>
  </section>
</div>

<style>
  .preferences { max-width: 760px; }

  /* ── WizHeader — matches mockup WizHeader ──── */
  .wiz-header {
    margin-bottom: 32px;
    position: sticky; top: 0; z-index: 2;
    background: var(--paper); padding-bottom: 16px;
  }
  .wiz-label {
    font-size: 11px; color: var(--sumi-3); letter-spacing: 0.12em;
    text-transform: uppercase; margin-bottom: 8px;
  }
  .wiz-label-kanji { color: var(--shu); margin-right: 8px; }
  .wiz-title {
    font-size: 36px; font-weight: 300; letter-spacing: -0.02em;
    margin: 0 0 6px;
  }
  .wiz-tagline { font-size: 14px; color: var(--sumi-3); margin: 0; }

  /* ── Section ─────────────────────────────── */
  .section { padding-top: 24px; padding-bottom: 4px; border-top: var(--hairline); }
  .section-header {
    display: flex; align-items: center; gap: 14px; margin-bottom: 14px;
  }
  .section-kanji { font-size: 22px; color: var(--shu); line-height: 1; width: 30px; text-align: center; flex-shrink: 0; }
  .section-info { flex: 1; min-width: 0; }
  .section-title { font-size: 17px; font-weight: 400; margin: 0; color: var(--sumi); }
  .section-sub {
    font-size: 12px; color: var(--sumi-3); margin: 3px 0 0;
    line-height: 1.5;
  }
  .section-body { padding-left: 44px; }

  /* ── Row ──────────────────────────────────── */
  .row {
    display: grid; grid-template-columns: 1fr auto; gap: 32px;
    align-items: center; padding: 11px 0;
    border-bottom: 1px solid var(--paper-edge);
  }
  .row-text { min-width: 0; }
  .row-label { font-size: 13px; color: var(--sumi); }
  .row-hint {
    font-size: 11.5px; color: var(--sumi-3); margin-top: 3px;
    line-height: 1.45; max-width: 460px;
  }

  /* ── Toggle ──────────────────────────────── */
  .toggle {
    width: 36px; height: 20px; border-radius: 999px; position: relative;
    background: var(--paper-edge); cursor: pointer; padding: 0;
    border: none; transition: background 0.15s;
  }
  .toggle.on { background: var(--shu); }
  .toggle-thumb {
    position: absolute; top: 2px; left: 2px;
    width: 16px; height: 16px; border-radius: 50%;
    background: var(--paper); transition: left 0.18s ease;
    box-shadow: 0 1px 3px rgba(0,0,0,0.18);
  }
  .toggle.on .toggle-thumb { left: 18px; }

  /* ── Segment ─────────────────────────────── */
  .segment {
    display: inline-flex; border: var(--hairline);
    border-radius: 5px; overflow: hidden;
  }
  .segment-btn {
    padding: 6px 12px; font-size: 11.5px;
    border: none; border-left: var(--hairline);
    background: var(--paper); color: var(--sumi-3);
    cursor: pointer; font-family: var(--font-ui);
  }
  .segment-btn:first-child { border-left: none; }
  .segment-btn.active { background: var(--paper-3); color: var(--sumi); }

  /* ── Select ──────────────────────────────── */
  .sel {
    font-size: 12px; padding: 6px 10px; border: var(--hairline);
    border-radius: 5px; background: var(--paper); color: var(--sumi);
    cursor: pointer; font-family: inherit;
  }

  /* ── Name input (inline with section header) ── */
  .name-input {
    width: 200px; padding: 8px 12px; font-size: 14px;
    border: var(--hairline); border-radius: 6px;
    background: var(--paper); color: var(--sumi);
    font-family: inherit; outline: none; flex-shrink: 0;
  }
  .name-input:focus { border-color: var(--shu); }
</style>
