<svelte:head>
  <title>Docs — Sensei</title>
</svelte:head>

<div class="page">
  <nav class="page-nav">
    <a href="/" class="back-link">
      <span class="kanji" style="font-size: 18px; color: var(--shu);">先生</span>
      <span class="display" style="font-size: 15px;">Sensei</span>
    </a>
  </nav>

  <div class="page-content">

    <!-- Philosophy -->
    <section class="doc-section">
      <div class="section-tag">Philosophy</div>
      <h1 class="display page-title">
        The master observes before teaching.
      </h1>
      <div class="prose">
        <p>
          AI coding tools are getting louder. More suggestions, more autocompletes,
          more interrupting. Sensei moves the other way.
        </p>
        <p>
          Sensei is a patient observer. It sits beside your editor and AI assistants,
          watching the shape of each session — the prompts, the responses, the corrections.
          It speaks rarely, and only when it has something specific to say. Most days it is
          completely silent — and that is the feature.
        </p>
        <p>
          The kanji throughout the app are not decoration. Each one names a phase of
          practice: <span class="kanji" style="color: var(--shu);">観</span> observation,
          <span class="kanji" style="color: var(--shu);">察</span> recognition,
          <span class="kanji" style="color: var(--shu);">覚</span> adoption,
          <span class="kanji" style="color: var(--shu);">練</span> refinement.
          They are what we ask of the user, and what we ask of ourselves.
        </p>
      </div>
    </section>

    <!-- Objective -->
    <section class="doc-section">
      <div class="section-tag">Objective</div>
      <h2 class="display section-title">
        Make AI-assisted development measurably better.
      </h2>
      <div class="prose">
        <p>
          Sensei exists to close the feedback loop between developers and their AI tools.
          Today, teams adopt AI assistants and hope for the best. There's no visibility into
          what's working, what's not, and where the friction lives. Sensei provides that visibility.
        </p>
        <p>
          It tracks First-Try Rate — how often a session completes without corrections — per project,
          per module, per pattern. It detects recurring idioms and anti-patterns. It surfaces
          coaching recommendations with projected impact. And it does all of this locally,
          without sending a single byte off your machine.
        </p>
      </div>
    </section>

    <!-- What you get -->
    <section class="doc-section">
      <div class="section-tag">What you get</div>
      <h2 class="display section-title">
        The full toolkit.
      </h2>

      <div class="toolkit-grid">
        <div class="toolkit-card">
          <div class="kanji toolkit-kanji">令</div>
          <h3 class="display toolkit-title">20 Commands</h3>
          <p class="toolkit-desc">
            Phased development workflow from <code>/idea</code> through <code>/validate</code>.
            Cross-cutting commands for brainstorming, review, and session management.
            Utility commands for checkpoints, commits, mockups, and library docs.
          </p>
        </div>
        <div class="toolkit-card">
          <div class="kanji toolkit-kanji">士</div>
          <h3 class="display toolkit-title">8 Agents</h3>
          <p class="toolkit-desc">
            Specialist perspectives that run autonomously: analyst, developer, acceptance tester,
            security reviewer, performance engineer, DevOps/SRE, UX designer, and
            a generic persona reviewer for project-specific roles.
          </p>
        </div>
        <div class="toolkit-card">
          <div class="kanji toolkit-kanji">具</div>
          <h3 class="display toolkit-title">MCP Tools</h3>
          <p class="toolkit-desc">
            Hybrid code search (full-text + semantic + structural), call graph analysis,
            pattern detection and matching, duplicate detection, library doc fetching,
            and session management with token-budgeted context packing.
          </p>
        </div>
        <div class="toolkit-card">
          <div class="kanji toolkit-kanji">技</div>
          <h3 class="display toolkit-title">Skills & Hooks</h3>
          <p class="toolkit-desc">
            Pattern-based development, unknown library detection, doc drift detection,
            TDD enforcement, code review automation, and session hooks for quality gates
            at every stage of the workflow.
          </p>
        </div>
      </div>
    </section>

    <!-- Architecture -->
    <section class="doc-section">
      <div class="section-tag">Architecture</div>
      <h2 class="display section-title">
        How it's built.
      </h2>
      <div class="prose">
        <p>
          Sensei is four components: a <strong>Rust daemon</strong> (<code>senseid</code>) that indexes
          your codebase and serves an HTTP API; an <strong>MCP server</strong> (<code>sensei-mcp</code>)
          that translates AI tool calls into daemon requests; a <strong>CLI</strong> (<code>sensei</code>)
          for setup, scanning, and management; and a <strong>Tauri desktop app</strong> for visual
          observation and configuration.
        </p>
        <p>
          Data lives in a local <strong>PostgreSQL</strong> database with pgvector for semantic search
          embeddings. <strong>Ollama</strong> provides optional local inference for pattern detection,
          code similarity, prompt classification, and embedding generation — recommended models are
          chosen based on your hardware. All features degrade gracefully without Ollama.
        </p>
        <p>
          The system supports multiple AI platforms simultaneously through a capability registry
          that adapts features to each platform's strengths.
        </p>
      </div>
    </section>

    <!-- Local inference -->
    <section class="doc-section">
      <div class="section-tag">Local inference</div>
      <h2 class="display section-title">
        On-device intelligence.
      </h2>
      <div class="prose">
        <p>
          Sensei uses Ollama for tasks that don't need cloud APIs: pattern detection, code similarity
          analysis, semantic search embeddings, prompt classification, and docstring generation.
          This keeps routine analysis local and reduces API costs.
        </p>
        <p>
          Hardware-aware model selection recommends the right models for your machine:
        </p>
      </div>
      <div class="hw-grid">
        <div class="hw-card">
          <div class="mono hw-spec">8 GB</div>
          <div class="hw-model">Gemma3:12b</div>
          <div class="hw-note">Minimum viable</div>
        </div>
        <div class="hw-card">
          <div class="mono hw-spec">16 GB</div>
          <div class="hw-model">Gemma3:27b</div>
          <div class="hw-note">Recommended</div>
        </div>
        <div class="hw-card">
          <div class="mono hw-spec">16 GB + GPU</div>
          <div class="hw-model">MOE panel</div>
          <div class="hw-note">Multi-model consensus</div>
        </div>
      </div>
      <div class="prose" style="margin-top: 32px;">
        <p>
          When multiple models are available, sensei runs a consensus panel — models debate insights
          for root cause analysis and impact prediction. The reasoning is transparent and visible
          in the desktop app.
        </p>
      </div>
    </section>

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
    max-width: 800px;
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
    font-size: 48px;
    font-weight: 300;
    margin: 0 0 32px;
    letter-spacing: -0.025em;
    line-height: 1.15;
  }
  .section-title {
    font-size: 36px;
    font-weight: 300;
    margin: 0 0 28px;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  .doc-section {
    margin-bottom: 100px;
  }
  .prose {
    font-size: 16px;
    color: var(--sumi-2);
    line-height: 1.75;
  }
  .prose p { margin: 0 0 18px; }
  .prose strong { color: var(--sumi); font-weight: 500; }
  .prose code {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--paper-2);
    padding: 2px 6px;
    border-radius: 4px;
  }
  .toolkit-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
    margin-top: 8px;
  }
  .toolkit-card {
    padding: 28px;
    border: var(--hairline);
    border-radius: 12px;
    background: var(--paper-2);
  }
  .toolkit-kanji {
    font-size: 32px;
    color: var(--shu);
    margin-bottom: 14px;
  }
  .toolkit-title {
    font-size: 20px;
    font-weight: 400;
    margin: 0 0 12px;
    letter-spacing: -0.01em;
  }
  .toolkit-desc {
    font-size: 13px;
    color: var(--sumi-2);
    line-height: 1.65;
    margin: 0;
  }
  .toolkit-desc code {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--paper-3);
    padding: 1px 5px;
    border-radius: 3px;
  }
  .hw-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 20px;
    margin-top: 20px;
  }
  .hw-card {
    padding: 20px;
    border: var(--hairline);
    border-radius: 10px;
    text-align: center;
  }
  .hw-spec { font-size: 13px; color: var(--sumi-3); margin-bottom: 6px; }
  .hw-model { font-size: 16px; font-weight: 500; color: var(--sumi); }
  .hw-note { font-size: 11px; color: var(--sumi-3); margin-top: 4px; }
  @media (max-width: 900px) {
    .page-nav { padding: 20px 24px; }
    .page-content { padding: 48px 24px 80px; }
    .page-title { font-size: 32px; }
    .section-title { font-size: 28px; }
    .toolkit-grid { grid-template-columns: 1fr; }
    .hw-grid { grid-template-columns: 1fr; }
  }
</style>
