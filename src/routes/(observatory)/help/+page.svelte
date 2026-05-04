<div class="help-page">
  <h1>Sensei Help</h1>

  <section>
    <h2>Quick Start</h2>
    <ol>
      <li><strong>Scan</strong> — Point Sensei at your project folders to index them.</li>
      <li><strong>Watch</strong> — Sensei observes your AI coding sessions and learns your patterns.</li>
      <li><strong>Teach</strong> — Adopt recommendations to improve your first-try rate.</li>
    </ol>
  </section>

  <section>
    <h2>Keyboard Shortcuts</h2>
    <table class="shortcuts">
      <thead>
        <tr><th scope="col">Shortcut</th><th scope="col">Action</th></tr>
      </thead>
      <tbody>
        <tr><td>⌘N</td><td>New project</td></tr>
        <tr><td>⌘W</td><td>Close window</td></tr>
        <tr><td>⌘,</td><td>Preferences</td></tr>
        <tr><td>⌘\</td><td>Toggle sidebar</td></tr>
        <tr><td>⌘1</td><td>Today</td></tr>
        <tr><td>⌘2</td><td>Projects</td></tr>
        <tr><td>⌘3</td><td>Sessions</td></tr>
        <tr><td>⌘0</td><td>Show Observatory</td></tr>
        <tr><td>⌘?</td><td>This help window</td></tr>
      </tbody>
    </table>
  </section>

  <section>
    <h2>FAQ</h2>
    <dl>
      <dt>What is FTR?</dt>
      <dd>First-Try Rate — the percentage of AI coding tasks completed without needing corrections. Higher is better.</dd>
      <dt>What is the daemon?</dt>
      <dd>A background process that indexes your repositories and watches for new sessions. It runs at <code>localhost:7749</code> by default.</dd>
      <dt>What does memory scope mean?</dt>
      <dd>Memories can be <em>project-scoped</em> (apply to one project) or shared to the <em>collective</em> (apply globally across all projects).</dd>
    </dl>
  </section>
</div>

<style>
  .help-page { padding: 32px; max-width: 640px; }
  h1 { font-size: 22px; margin-bottom: 24px; }
  h2 { font-size: 15px; font-weight: 700; margin: 28px 0 10px; }
  ol, ul { font-size: 14px; line-height: 1.7; padding-left: 20px; }
  .shortcuts { border-collapse: collapse; font-size: 13px; width: 100%; }
  .shortcuts th { padding: 5px 12px 5px 0; font-size: 11px; opacity: 0.5; font-weight: 600; text-align: left; }
  .shortcuts td { padding: 5px 12px 5px 0; border-bottom: 1px solid var(--border); }
  .shortcuts td:first-child { font-family: monospace; font-size: 14px; font-weight: 600; min-width: 60px; }
  dl { font-size: 13px; }
  dt { font-weight: 700; margin-top: 12px; }
  dd { opacity: 0.7; margin: 4px 0 0 0; line-height: 1.6; }
  code { background: var(--surface-3); padding: 1px 5px; border-radius: 3px; font-size: 12px; }
</style>
