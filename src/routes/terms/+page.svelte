<svelte:head>
  <title>Terms of Use — Sensei</title>
</svelte:head>

<div class="page">
  <nav class="page-nav">
    <a href="/" class="back-link">
      <span class="kanji" style="font-size: 18px; color: var(--shu);">先生</span>
      <span class="display" style="font-size: 15px;">Sensei</span>
    </a>
  </nav>

  <div class="page-content">
    <div class="section-tag">Terms of use</div>
    <h1 class="display page-title">Simple terms.</h1>
    <p class="updated">Last updated: April 2026</p>

    <div class="prose">
      <h2>The short version</h2>
      <p>
        Sensei is free during the preview period. We provide it as-is, with no warranty.
        Your data is yours — we never see it.
      </p>

      <h2>License</h2>
      <p>
        Sensei is proprietary software. The source code is available for reference at
        <a href="https://github.com/mizukisu/sensei-dev">github.com/mizukisu/sensei-dev</a>.
        Use of the software is subject to the license terms in the repository.
      </p>

      <h2>Use at your own risk</h2>
      <p>
        Sensei is provided "as is" without warranty of any kind, express or implied.
        The authors are not liable for any damages arising from the use of this software.
        Sensei interacts with your local files and AI coding tools — you are responsible
        for reviewing any actions taken during your sessions.
      </p>

      <h2>Local inference</h2>
      <p>
        Sensei can optionally use Ollama for local model inference. This runs on your
        hardware and may use significant compute resources depending on the model size
        and your hardware configuration. Sensei provides hardware-aware recommendations
        but you are responsible for managing your system resources.
      </p>

      <h2>Third-party services</h2>
      <p>
        Sensei connects to AI coding assistants (Claude Code, Cursor, etc.) via MCP.
        Your use of those services is governed by their respective terms. Sensei does
        not modify or extend the terms of those services.
      </p>

      <h2>Sponsorship and donations</h2>
      <p>
        Donations via Ko-fi or GitHub Sponsors are voluntary and non-refundable.
        Donations do not grant additional features, support, or guarantees beyond
        what is available in the free software.
      </p>

      <h2>Changes to these terms</h2>
      <p>
        We may update these terms from time to time. Changes will be posted on this
        page with an updated date. Continued use of sensei after changes constitutes
        acceptance of the new terms.
      </p>

      <h2>Contact</h2>
      <p>
        Questions about these terms can be directed to the project maintainers via
        <a href="https://github.com/mizukisu/sensei-dev/issues">GitHub Issues</a>.
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
