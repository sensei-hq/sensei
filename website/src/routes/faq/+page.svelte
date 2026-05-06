<script lang="ts">
  const faqs = [
    { q: 'Which AI assistants does sensei work with?',
      a: 'Any AI coding assistant that speaks MCP — Claude Code, Cursor, Windsurf, Copilot, Codex, and Aider. Sensei connects via its MCP server and adapts to each platform\'s capabilities through a feature registry.' },
    { q: 'What does sensei ship with?',
      a: 'A full toolkit: 20 slash commands for phased development (from /idea through /validate), 8 specialist agents (analyst, developer, acceptance tester, security reviewer, performance engineer, DevOps/SRE, UX designer, and persona reviewer), plus skills, hooks, and MCP tools for code search, pattern detection, and call graph analysis.' },
    { q: 'How does the phased workflow work?',
      a: 'Sensei organizes development into phases — idea, analyze, blueprint, experiment, plan, build, and validate. Each phase produces artifacts that carry context forward. You can follow the full workflow or pick a recipe (Lean, Exploratory, Maintenance) that fits the task.' },
    { q: 'Does sensei see my code?',
      a: 'Only what passes through your AI tool\'s session, plus what it indexes locally from your repos. Everything is stored in a local PostgreSQL database you fully control — inspect, export, or delete at any time.' },
    { q: 'Will it slow down my machine?',
      a: 'The Rust daemon is lightweight and event-driven — it only does work when sessions happen. Ollama can use additional resources when running local inference models, but sensei recommends models based on your hardware and degrades gracefully if Ollama isn\'t available.' },
    { q: 'What is local inference used for?',
      a: 'Ollama powers on-device tasks like pattern detection, code similarity analysis, semantic search embeddings, prompt classification, and docstring generation. This keeps routine analysis off cloud APIs, reducing cost. All features still work without Ollama — it falls back to heuristics.' },
    { q: 'What hardware do I need for local inference?',
      a: 'Sensei detects your hardware and recommends models accordingly. 8GB RAM supports Gemma3:12b minimum. 16GB is recommended for Gemma3:27b. With 16GB+ and a GPU, sensei can run a full multi-model consensus panel. Local inference is always optional.' },
    { q: 'What MCP tools does sensei expose?',
      a: 'Code intelligence tools including hybrid search (full-text + semantic + structural), call graph analysis (get_callers, get_callees), pattern detection and matching, duplicate detection, library documentation fetching, and session management (checkpoints, context packing).' },
    { q: 'What are the built-in agents?',
      a: 'Eight specialist agents wrap different review perspectives: analyst (problem analysis), developer (implementation verification), acceptance-tester (user-perspective testing), security-reviewer (OWASP auditing), performance-engineer (bottleneck analysis), devops-sre (deployability), ux-designer (usability), and a generic persona-reviewer that loads project-specific personas.' },
    { q: 'How does pattern detection work?',
      a: 'Sensei indexes your codebase and detects recurring patterns, idioms, and anti-patterns. Patterns have a lifecycle: detected → surfaced → enforced → growing. You decide which to adopt. Once adopted, sensei applies them in future sessions and flags deviations during code review.' },
    { q: 'What is First-Try Rate (FTR)?',
      a: 'FTR measures how often an AI session completes its task without corrections. Sensei tracks FTR per-project, per-module, and per-pattern to surface where your workflow is strong and where it needs attention.' },
    { q: 'Can I export my data?',
      a: 'Yes. Settings → Export gives you a JSON dump of every pattern, memory, and adopted teaching. Import is also supported.' },
    { q: 'Does sensei work across multiple projects?',
      a: 'Yes. Sensei supports solution grouping — logical products spanning multiple repos with shared profiles, cross-repo graph detection, and solution-scoped metrics.' },
    { q: 'What\'s the pricing model?',
      a: 'Sensei is free during the preview period. We\'re experimenting and learning what\'s useful. If we move to a paid tier, early adopters and supporters will receive a permanent discount. The core promise — quiet, local, observant — never changes regardless of pricing.' },
  ];
</script>

<svelte:head>
  <title>FAQ — Sensei</title>
</svelte:head>

<div class="page">
  <nav class="page-nav">
    <a href="/" class="back-link">
      <span class="kanji" style="font-size: 18px; color: var(--shu);">先生</span>
      <span class="display" style="font-size: 15px;">Sensei</span>
    </a>
  </nav>

  <div class="page-content">
    <div class="section-tag">Frequently asked</div>
    <h1 class="display page-title">Questions & answers.</h1>
    <p class="page-intro display">
      Everything you might want to know before downloading.
    </p>

    <div class="faq-list">
      {#each faqs as it, i}
        <details class="faq-item" class:last={i === faqs.length - 1}>
          <summary class="faq-summary display">
            <span>{it.q}</span>
            <span class="faq-toggle">+</span>
          </summary>
          <div class="faq-answer">{it.a}</div>
        </details>
      {/each}
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
    margin: 0 0 20px;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .page-intro {
    font-size: 17px;
    color: var(--sumi-2);
    font-weight: 300;
    line-height: 1.6;
    margin: 0 0 64px;
  }
  .faq-item {
    border-top: var(--hairline);
    padding: 28px 0;
  }
  .faq-item.last { border-bottom: var(--hairline); }
  .faq-summary {
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    font-size: 17px;
    color: var(--sumi);
    font-weight: 400;
    list-style: none;
  }
  .faq-summary::-webkit-details-marker { display: none; }
  .faq-toggle { color: var(--sumi-3); }
  .faq-answer {
    font-size: 14px;
    color: var(--sumi-2);
    line-height: 1.7;
    margin-top: 16px;
    max-width: 720px;
  }
  @media (max-width: 900px) {
    .page-nav { padding: 20px 24px; }
    .page-content { padding: 48px 24px 80px; }
    .page-title { font-size: 32px; }
  }
</style>
