<svelte:head>
  <title>Privacy Policy — Sensei</title>
</svelte:head>

<div class="page">
  <nav class="page-nav">
    <a href="/" class="back-link">
      <span class="kanji" style="font-size: 18px; color: var(--shu);">先生</span>
      <span class="display" style="font-size: 15px;">Sensei</span>
    </a>
  </nav>

  <div class="page-content">
    <div class="section-tag">Privacy policy</div>
    <h1 class="display page-title">Your data stays yours.</h1>
    <p class="updated">Last updated: April 2026</p>

    <div class="prose">
      <h2>Summary</h2>
      <p>
        Sensei is local-first software. Your code, sessions, patterns, and memories are
        stored on your machine in a local PostgreSQL database. We do not collect, transmit,
        or have access to any of your data.
      </p>

      <h2>What sensei stores</h2>
      <p>
        Sensei stores the following data locally on your machine:
      </p>
      <ul>
        <li>Session transcripts between you and your AI coding assistants</li>
        <li>Detected patterns, idioms, and adopted memories</li>
        <li>Codebase index data (file structure, symbols, call graphs)</li>
        <li>Semantic search embeddings (generated locally via Ollama when available)</li>
        <li>Session metrics (first-try rates, tool usage, cost tracking)</li>
        <li>Your configuration preferences</li>
      </ul>

      <h2>What sensei does not do</h2>
      <ul>
        <li>Does not make outbound network requests (except to the AI assistant you already use)</li>
        <li>Does not collect telemetry or usage analytics</li>
        <li>Does not phone home, check licenses, or validate accounts</li>
        <li>Does not transmit any data to our servers — we don't operate servers</li>
        <li>Does not include tracking pixels, analytics scripts, or fingerprinting</li>
      </ul>

      <h2>Third-party services</h2>
      <p>
        Sensei connects to your AI coding assistant (Claude Code, Cursor, etc.) via MCP.
        Those services have their own privacy policies. Sensei does not add to or modify
        what those services already see.
      </p>
      <p>
        When Ollama is used for local inference, all model execution happens on your
        hardware. No data is sent to external model providers.
      </p>

      <h2>Update checks</h2>
      <p>
        Sensei does not check for updates automatically. Updates are checked manually
        from Help → Check for Updates, which makes a single request to GitHub's
        releases API.
      </p>

      <h2>Data deletion</h2>
      <p>
        All sensei data lives in a single directory on your machine. Deleting that
        directory removes everything. You can also export all data to JSON before deletion.
        Uninstalling the app removes all application files.
      </p>

      <h2>This website</h2>
      <p>
        This website is a static site. It does not use cookies, does not run analytics,
        and does not collect personal information. Font files are loaded from Google Fonts,
        which is subject to Google's privacy policy.
      </p>

      <h2>Contact</h2>
      <p>
        Questions about this policy can be directed to the project maintainers via
        <a href="https://github.com/sensei-hq/sensei/issues">GitHub Issues</a>.
      </p>
    </div>
  </div>
</div>

<style>
  .page {
    background: var(--paper);
    color: var(--sumi);
    min-height: 100vh;
    font-family: var(--font-ui);
  }
  .page-nav {
    padding: 24px 56px;
    border-bottom: var(--hairline);
  }
  .back-link {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    text-decoration: none;
  }
  .page-content {
    max-width: 720px;
    margin: 0 auto;
    padding: 80px 56px 120px;
  }
  .section-tag {
    font-size: 11px;
    letter-spacing: 0.22em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 16px;
  }
  .page-title {
    font-size: 44px;
    font-weight: 300;
    margin: 0 0 12px;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .updated {
    font-size: 12px;
    color: var(--sumi-3);
    margin: 0 0 56px;
  }
  .prose {
    font-size: 15px;
    color: var(--sumi-2);
    line-height: 1.75;
  }
  .prose h2 {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 400;
    color: var(--sumi);
    margin: 48px 0 16px;
    letter-spacing: -0.01em;
  }
  .prose h2:first-child { margin-top: 0; }
  .prose p { margin: 0 0 16px; }
  .prose ul { margin: 0 0 16px; padding-left: 20px; }
  .prose li { margin-bottom: 8px; }
  .prose a {
    color: var(--shu);
    text-decoration: none;
  }
  .prose a:hover { text-decoration: underline; }
  @media (max-width: 900px) {
    .page-nav { padding: 20px 24px; }
    .page-content { padding: 48px 24px 80px; }
    .page-title { font-size: 32px; }
  }
</style>
