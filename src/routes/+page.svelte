<script lang="ts">
  import { browser } from '$app/environment';
  import { vibe } from '@rokkit/states';
  import MockToday from '$lib/components/mock/MockToday.svelte';
  import MockSessions from '$lib/components/mock/MockSessions.svelte';
  import MockInsights from '$lib/components/mock/MockInsights.svelte';
  import MockMemory from '$lib/components/mock/MockMemory.svelte';
  import MockInstruments from '$lib/components/mock/MockInstruments.svelte';

  const GITHUB = 'https://github.com/sensei-hq/sensei';
  const RELEASES = 'https://github.com/sensei-hq/sensei-releases';
  const RELEASE_BASE = `${RELEASES}/releases/latest/download`;

  // ── Platform detection ──────────────────────────────────────
  let os = $state('macOS');
  let dlFile = $state('Sensei_aarch64.dmg');

  if (browser) {
    const ua = navigator.userAgent || '';
    if (/Win/.test(ua)) { os = 'Windows'; dlFile = 'Sensei_x86_64-setup.exe'; }
    else if (/Linux/.test(ua)) { os = 'Linux'; dlFile = 'sensei_amd64.AppImage'; }
  }

  // ── Theme toggle ────────────────────────────────────────────
  let isDark = $derived(vibe.mode === 'dark');

  function toggleTheme() {
    vibe.mode = isDark ? 'light' : 'dark';
  }

  // ── Data ────────────────────────────────────────────────────
  const navLinks = [
    ['#how', 'How'],
    ['#gallery', 'Screens'],
    ['#philosophy', 'Philosophy'],
    ['#privacy', 'Privacy'],
    ['#faq', 'FAQ'],
  ] as const;

  const steps = [
    { kanji: '観', phase: 'Watch', title: 'It sits beside you',
      text: 'Sensei sits beside your editor and AI tools, capturing the shape of each session — the prompts, the responses, the corrections.',
      sub: 'Local only. Nothing leaves your machine.' },
    { kanji: '察', phase: 'Notice', title: 'It begins to see',
      text: 'After a few days, patterns surface. Recurring frictions. Idioms forming. Things you taught the assistant once and may want to teach it again.',
      sub: 'You decide what\'s signal and what isn\'t.' },
    { kanji: '覚', phase: 'Adopt', title: 'It remembers, with consent',
      text: 'Worthy patterns become memories — small, named lessons sensei applies to future sessions on your behalf, with your blessing.',
      sub: 'Adopt, refine, or dismiss. Always your call.' },
  ];

  const screens = [
    { caption: 'Today', num: '01',
      sub: 'The morning briefing. One observation that\'s worth your attention. Everything else stays out of sight.' },
    { caption: 'Sessions', num: '02',
      sub: 'The week in review. Going well, not going well, things noticed — three lanes, no charts to decode.' },
    { caption: 'Insights', num: '03',
      sub: 'What sensei has noticed. Patterns with confidence and provenance. You decide which become memories.' },
    { caption: 'Memories', num: '04',
      sub: 'Adopted teachings. Each one named, dated, and traceable to the sessions it came from. No black box.' },
    { caption: 'Instruments', num: '05',
      sub: 'Your tools, observed. Try them in isolation, replay what the assistant did, watch toolset health over time.' },
  ];

  const privacyItems = [
    { k: '蔵', title: 'Local storage only',
      text: 'Transcripts, patterns, and memories are stored in a local PostgreSQL database on your machine. Sensei never makes outbound network requests beyond the AI assistant you already use.' },
    { k: '鍵', title: 'No telemetry',
      text: 'We don\'t track usage. Updates are checked manually from Help → Check for Updates. Local inference via Ollama means even model calls stay on your hardware.' },
    { k: '破', title: 'Easy to delete',
      text: 'One data directory. Delete it and sensei forgets everything. Export to JSON anytime.' },
  ];

  const faqs = [
    { q: 'Which AI assistants does it work with?',
      a: 'Any AI coding assistant that speaks MCP — Claude Code, Cursor, Windsurf, Copilot, Codex, and Aider. Sensei connects via its MCP server and adapts to each platform\'s capabilities.' },
    { q: 'What does sensei ship with?',
      a: 'A full toolkit: 20 slash commands for phased development (from /idea to /validate), 8 specialist agents (analyst, developer, security reviewer, and more), plus skills, hooks, and MCP tools for code search, pattern detection, and call graph analysis.' },
    { q: 'Does sensei see my code?',
      a: 'Only what passes through your AI tool\'s session, plus what it indexes locally. Everything is stored in a local PostgreSQL database you fully control — inspect, export, or delete at any time.' },
    { q: 'Will it slow down my machine?',
      a: 'The Rust daemon is lightweight and event-driven. Ollama can use additional resources when running local inference models — sensei recommends models based on your hardware and degrades gracefully if Ollama isn\'t available.' },
    { q: 'What is local inference used for?',
      a: 'Ollama powers on-device tasks like pattern detection, code similarity, semantic search embeddings, and prompt classification — keeping routine analysis off cloud APIs and reducing cost.' },
    { q: 'Can I export my data?',
      a: 'Yes. Settings → Export gives you a JSON dump of every pattern, memory, and adopted teaching. Import is also supported.' },
    { q: 'What\'s the long-term plan?',
      a: 'Sensei stays local-first and free. We may add an optional paid tier later for cross-machine sync, but the core promise — quiet, local, observant — never changes.' },
  ];

  const footerCols = [
    { title: 'Product', links: [
      { label: 'Download', href: '#' },
      { label: 'FAQ', href: '/faq' },
      { label: 'Docs', href: '/docs' },
      { label: 'Changelog', href: `${GITHUB}/releases` },
    ]},
    { title: 'Legal', links: [
      { label: 'Privacy', href: '/privacy' },
      { label: 'Terms', href: '/terms' },
    ]},
    { title: 'Source', links: [
      { label: 'GitHub', href: GITHUB },
      { label: 'Issues', href: `${GITHUB}/issues` },
      { label: 'Roadmap', href: `${GITHUB}/projects` },
    ]},
  ];
</script>

<div class="site">

  <!-- ═══ Nav ═══ -->
  <nav class="nav">
    <div class="nav-inner">
      <a href="/" class="logo-link">
        <span class="kanji logo-kanji">先生</span>
        <span class="display logo-text">Sensei</span>
      </a>
      <div class="nav-links">
        {#each navLinks as [href, label]}
          <a {href} class="nav-link">{label}</a>
        {/each}
        <button class="theme-toggle" onclick={toggleTheme} aria-label="Toggle dark mode">
          {#if isDark}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
          {/if}
        </button>
      </div>
    </div>
  </nav>

  <!-- ═══ Hero ═══ -->
  <section class="hero">
    <div class="hero-kanji kanji">観</div>
    <div class="hero-content">
      <div class="hero-tagline">
        <span class="ink-dot" style="width: 8px; height: 8px;"></span>
        <div class="hero-tag-text">Sensei · the patient observer</div>
      </div>
      <h1 class="display hero-heading">
        A quiet companion<br/>
        for AI-assisted <em>work</em>.
      </h1>
      <p class="hero-sub display">
        Sensei watches your sessions with AI assistants —
        then surfaces the patterns you're too close to see. Not a
        chatbot. Not a copilot. A patient observer.
      </p>
      <div class="hero-actions">
        <a href="{RELEASE_BASE}/{dlFile}" class="btn-solid">
          <span class="kanji" style="font-size: 19px; color: var(--shu);">下</span>
          Download for {os}
        </a>
        <a href="#how" class="hero-link">See how it works ↓</a>
      </div>
      <div class="hero-note">Free preview · Local-first · No account required</div>
    </div>
    <div class="hero-screen">
      <MockToday width={1040} height={620} />
    </div>
  </section>

  <!-- ═══ Stats ═══ -->
  <section class="stats">
    <div class="stats-inner">
      {#each [
        { v: '0', k: 'external requests' },
        { v: '<60MB', k: 'memory footprint' },
        { v: 'MCP', k: 'open protocol' },
        { v: 'Preview', k: 'free during preview' },
      ] as stat}
        <div class="stat">
          <div class="display stat-value">{stat.v}</div>
          <div class="stat-label">{stat.k}</div>
        </div>
      {/each}
    </div>
  </section>

  <!-- ═══ What it is ═══ -->
  <section class="what-it-is">
    <div class="what-inner">
      <div>
        <div class="section-tag">What it is</div>
        <h2 class="display what-heading">
          One desktop app.<br/>
          One quiet promise.
        </h2>
      </div>
      <div class="what-body display">
        <p>
          Sensei runs on your machine and observes your sessions with AI
          assistants. It logs nothing remotely; it speaks rarely; it
          remembers what you've actually done.
        </p>
        <p>
          Over weeks, it begins to recognize your patterns — the
          idioms you gravitate toward, the workarounds you've adopted,
          the friction points that keep recurring. When something looks
          worth noticing, it tells you. The rest of the time, it stays
          out of the way.
        </p>
      </div>
    </div>
  </section>

  <!-- ═══ How it works ═══ -->
  <section id="how" class="how-it-works">
    <div class="how-inner">
      <div class="section-tag">How it works</div>
      <h2 class="display how-heading">
        <span style="color: var(--shu);">観 · 察 · 覚</span><br/>
        Watch, notice, adopt.
      </h2>
      <div class="steps-grid">
        {#each steps as s}
          <div class="step-card">
            <div class="kanji step-kanji">{s.kanji}</div>
            <div class="step-phase">{s.phase}</div>
            <h3 class="display step-title">{s.title}</h3>
            <div class="step-text">{s.text}</div>
            <div class="step-sub">{s.sub}</div>
          </div>
        {/each}
      </div>
    </div>
  </section>

  <!-- ═══ Gallery ═══ -->
  <section id="gallery" class="gallery">
    <div class="gallery-inner">
      <div class="section-tag">The screens</div>
      <h2 class="display gallery-heading">
        Five surfaces, one rhythm.
      </h2>
      <p class="gallery-sub display">
        Every screen answers one question and stays quiet otherwise.
      </p>
      <div class="gallery-list">
        {#each screens as s, i}
          <div class="gallery-item" class:reverse={i % 2 !== 0}>
            <div class="gallery-screen">
              {#if s.caption === 'Today'}
                <MockToday width={920} height={580} />
              {:else if s.caption === 'Sessions'}
                <MockSessions width={920} height={580} />
              {:else if s.caption === 'Insights'}
                <MockInsights width={920} height={580} />
              {:else if s.caption === 'Memories'}
                <MockMemory width={920} height={580} />
              {:else}
                <MockInstruments width={920} height={580} />
              {/if}
            </div>
            <div class="gallery-caption">
              <div class="mono gallery-num" style="color: var(--shu);">{s.num}</div>
              <div class="display gallery-name">{s.caption}</div>
              <div class="gallery-desc">{s.sub}</div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </section>

  <!-- ═══ Philosophy ═══ -->
  <section id="philosophy" class="philosophy">
    <div class="philosophy-kanji kanji">静</div>
    <div class="philosophy-inner">
      <div class="section-tag">Sei · stillness</div>
      <h2 class="display philosophy-heading">
        The master observes for a long time before teaching.
      </h2>
      <p class="philosophy-body display">
        AI tools are getting louder. More suggestions, more autocompletes,
        more interrupting. Sensei moves the other way. It speaks rarely,
        and only when it has something specific to say. Most days it is
        completely silent — and that is the feature.
      </p>
      <p class="philosophy-note">
        The kanji throughout the app are not decoration. Each one names
        a phase of practice — observation, recognition, adoption,
        refinement. They are what we ask of the user, and what we ask
        of ourselves as the people who built this.
      </p>
    </div>
  </section>

  <!-- ═══ Privacy ═══ -->
  <section id="privacy" class="privacy">
    <div class="privacy-inner">
      <div class="privacy-left">
        <span class="kanji" style="font-size: 56px; color: var(--shu);">蔵</span>
        <div class="section-tag" style="margin-top: 16px;">Privacy & local-first</div>
        <h2 class="display privacy-heading">
          Your sessions stay on your machine.
        </h2>
      </div>
      <div class="privacy-items">
        {#each privacyItems as it, i}
          <div class="privacy-item" class:bordered={i < 2}>
            <span class="kanji privacy-kanji">{it.k}</span>
            <div>
              <div class="display privacy-title">{it.title}</div>
              <div class="privacy-text">{it.text}</div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </section>

  <!-- ═══ Pricing ═══ -->
  <section class="pricing">
    <div class="pricing-inner">
      <div class="section-tag">Pricing</div>
      <h2 class="display pricing-heading">
        Free during preview.
      </h2>
      <p class="pricing-body display">
        Sensei is in early preview — we're learning what works and what
        doesn't. It's free while we figure that out. If we move to a paid
        tier, early adopters and supporters get a permanent discount.
        No surprises.
      </p>
      <div style="margin-top: 44px;">
        <a href="{RELEASE_BASE}/{dlFile}" class="btn-solid">
          <span class="kanji" style="font-size: 19px; color: var(--shu);">下</span>
          Download for {os}
        </a>
      </div>
    </div>
  </section>

  <!-- ═══ FAQ Summary ═══ -->
  <section id="faq" class="faq">
    <div class="faq-inner">
      <div class="section-tag">Quick answers</div>
      <h2 class="display faq-heading">
        The essentials.
      </h2>
      <div class="faq-cards">
        {#each [
          { q: 'What platforms?', a: 'Claude Code, Cursor, Windsurf, Copilot, Codex, Aider — anything that speaks MCP.' },
          { q: 'What\'s included?', a: '20 commands, 8 agents, skills, hooks, and MCP tools for code search, patterns, and call graph analysis.' },
          { q: 'Will it slow me down?', a: 'Rust daemon, event-driven. Ollama is optional and hardware-aware — it degrades gracefully.' },
          { q: 'Is it free?', a: 'Free during preview. If we move to a paid tier, early adopters get a permanent discount.' },
        ] as card}
          <div class="faq-card">
            <div class="display faq-card-q">{card.q}</div>
            <div class="faq-card-a">{card.a}</div>
          </div>
        {/each}
      </div>
      <a href="/faq" class="faq-more">All questions & answers →</a>
    </div>
  </section>

  <!-- ═══ Support ═══ -->
  <section class="support">
    <div class="support-inner">
      <span class="kanji" style="font-size: 56px; color: var(--shu);">志</span>
      <div class="section-tag" style="margin-top: 14px;">Support development</div>
      <h2 class="display support-heading">
        If sensei has earned a place in your practice, you can help keep it growing.
      </h2>
      <p class="support-body">
        Sensei is built by a small team. Every coffee buys an hour of focused work.
      </p>
      <a href="https://ko-fi.com/senseidev" target="_blank" rel="noopener" class="btn-shu">
        ♥ Buy me a coffee
      </a>
    </div>
  </section>

  <!-- ═══ Footer ═══ -->
  <footer class="footer">
    <div class="footer-inner">
      <div class="footer-brand">
        <div class="footer-logo">
          <span class="kanji" style="font-size: 18px; color: var(--shu); letter-spacing: -0.04em;">先生</span>
          <span class="display" style="font-size: 16px; color: var(--sumi-2);">Sensei</span>
        </div>
        <div class="footer-desc">
          A patient observer for AI-assisted work. Built quietly,
          shipped slowly.
        </div>
        <div class="mono footer-version">v0.4.2</div>
      </div>
      <div class="footer-cols">
        {#each footerCols as col}
          <div>
            <div class="footer-col-title">{col.title}</div>
            <div class="footer-col-links">
              {#each col.links as link}
                <a href={link.href} class="footer-link">{link.label}</a>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </div>
  </footer>

</div>

<style>
  /* ── Base ──────────────────────────────────────────── */
  .site {
    background: var(--paper);
    color: var(--sumi);
    min-height: 100%;
    font-family: var(--font-ui);
  }

  .section-tag {
    font-size: 11px;
    letter-spacing: 0.22em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 16px;
  }

  /* ── Nav ───────────────────────────────────────────── */
  .nav {
    position: sticky;
    top: 0;
    z-index: 50;
    border-bottom: var(--hairline);
    background: color-mix(in srgb, var(--paper) 92%, transparent);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
  .nav-inner {
    max-width: 1200px;
    margin: 0 auto;
    padding: 24px 56px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .logo-link {
    display: flex;
    align-items: baseline;
    gap: 10px;
    text-decoration: none;
  }
  .logo-kanji {
    font-size: 24px;
    color: var(--shu);
    letter-spacing: -0.04em;
  }
  .logo-text {
    font-size: 19px;
    letter-spacing: -0.01em;
    color: var(--sumi);
  }
  .nav-links {
    display: flex;
    gap: 32px;
    font-size: 13px;
    align-items: center;
  }
  .nav-link {
    color: var(--sumi-2);
    text-decoration: none;
    transition: color 0.15s;
  }
  .nav-link:hover { color: var(--sumi); }
  .theme-toggle {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: var(--hairline);
    border-radius: var(--radius);
    background: transparent;
    cursor: pointer;
    transition: background 0.14s;
    color: var(--sumi-2);
  }
  .theme-toggle:hover { background: var(--paper-2); }

  /* ── Hero ──────────────────────────────────────────── */
  .hero {
    padding: 72px 56px 40px;
    max-width: 1200px;
    margin: 0 auto;
    position: relative;
  }
  .hero-kanji {
    position: absolute;
    right: 56px;
    top: 24px;
    font-size: 320px;
    line-height: 1;
    color: var(--kanji-watermark);
    pointer-events: none;
  }
  .hero-content { position: relative; }
  .hero-tagline {
    display: flex;
    align-items: baseline;
    gap: 14px;
    margin-bottom: 24px;
  }
  .hero-tag-text {
    font-size: 11px;
    letter-spacing: 0.22em;
    color: var(--sumi-3);
    text-transform: uppercase;
  }
  .hero-heading {
    font-size: 84px;
    font-weight: 300;
    line-height: 1.02;
    letter-spacing: -0.03em;
    margin: 0;
    max-width: 920px;
  }
  .hero-heading em {
    color: var(--shu);
    font-style: normal;
  }
  .hero-sub {
    font-size: 19px;
    color: var(--sumi-2);
    line-height: 1.55;
    margin-top: 32px;
    max-width: 640px;
    font-weight: 300;
  }
  .hero-actions {
    display: flex;
    gap: 18px;
    align-items: center;
    margin-top: 40px;
  }
  .hero-link {
    font-size: 14px;
    color: var(--sumi-2);
    text-decoration: none;
  }
  .hero-note {
    font-size: 11.5px;
    color: var(--sumi-3);
    margin-top: 16px;
    letter-spacing: 0.05em;
  }
  .hero-screen {
    margin-top: 72px;
    display: flex;
    justify-content: center;
    position: relative;
    overflow-x: auto;
  }

  /* ── Stats ─────────────────────────────────────────── */
  .stats {
    border-top: var(--hairline);
    border-bottom: var(--hairline);
    padding: 32px 56px;
    background: var(--paper-2);
  }
  .stats-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 32px;
  }
  .stat { text-align: center; }
  .stat-value {
    font-size: 30px;
    font-weight: 400;
    color: var(--sumi);
  }
  .stat-label {
    font-size: 10.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin-top: 4px;
  }

  /* ── What it is ────────────────────────────────────── */
  .what-it-is { padding: 120px 56px; }
  .what-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1.6fr;
    gap: 80px;
    align-items: start;
  }
  .what-heading {
    font-size: 44px;
    font-weight: 300;
    margin: 0;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .what-body {
    font-size: 18px;
    line-height: 1.65;
    color: var(--sumi-2);
    font-weight: 300;
  }
  .what-body p:first-child { margin-top: 6px; }

  /* ── How it works ──────────────────────────────────── */
  .how-it-works {
    border-top: var(--hairline);
    border-bottom: var(--hairline);
    padding: 120px 56px;
    background: var(--paper-2);
  }
  .how-inner { max-width: 1200px; margin: 0 auto; }
  .how-heading {
    font-size: 56px;
    font-weight: 300;
    margin: 0 0 72px;
    letter-spacing: -0.025em;
    line-height: 1.05;
  }
  .steps-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 64px;
  }
  .step-card {
    padding: 32px 28px;
    background: var(--paper);
    border: var(--hairline);
    border-radius: 12px;
    position: relative;
  }
  .step-kanji {
    font-size: 48px;
    color: var(--shu);
    line-height: 1;
    margin-bottom: 18px;
  }
  .step-phase {
    font-size: 10.5px;
    letter-spacing: 0.22em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 10px;
  }
  .step-title {
    font-size: 22px;
    font-weight: 400;
    margin: 0 0 16px;
    letter-spacing: -0.01em;
  }
  .step-text {
    font-size: 14px;
    color: var(--sumi-2);
    line-height: 1.65;
    margin-bottom: 20px;
  }
  .step-sub {
    font-size: 11.5px;
    color: var(--sumi-3);
    font-style: italic;
    padding-top: 14px;
    border-top: var(--hairline);
  }

  /* ── Gallery ───────────────────────────────────────── */
  .gallery { padding: 120px 56px 60px; }
  .gallery-inner { max-width: 1200px; margin: 0 auto; }
  .gallery-heading {
    font-size: 56px;
    font-weight: 300;
    margin: 0 0 16px;
    letter-spacing: -0.025em;
    line-height: 1.05;
  }
  .gallery-sub {
    font-size: 17px;
    color: var(--sumi-2);
    max-width: 600px;
    line-height: 1.6;
    font-weight: 300;
    margin: 0 0 80px;
  }
  .gallery-list {
    display: flex;
    flex-direction: column;
    gap: 100px;
  }
  .gallery-item {
    display: grid;
    grid-template-columns: 1fr 320px;
    gap: 64px;
    align-items: center;
  }
  .gallery-item.reverse {
    grid-template-columns: 320px 1fr;
  }
  .gallery-item.reverse .gallery-screen { order: 1; }
  .gallery-item.reverse .gallery-caption { order: 0; }
  .gallery-screen { overflow-x: auto; }
  .gallery-num {
    font-size: 11px;
    margin-bottom: 8px;
  }
  .gallery-name {
    font-size: 32px;
    font-weight: 400;
    margin-bottom: 14px;
    letter-spacing: -0.015em;
  }
  .gallery-desc {
    font-size: 14px;
    color: var(--sumi-2);
    line-height: 1.65;
  }

  /* ── Philosophy ────────────────────────────────────── */
  .philosophy {
    border-top: var(--hairline);
    border-bottom: var(--hairline);
    padding: 160px 56px;
    background: var(--paper-2);
    position: relative;
    overflow: hidden;
  }
  .philosophy-kanji {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    font-size: 480px;
    line-height: 1;
    color: var(--kanji-watermark);
    pointer-events: none;
  }
  .philosophy-inner {
    max-width: 760px;
    margin: 0 auto;
    text-align: center;
    position: relative;
  }
  .philosophy-heading {
    font-size: 48px;
    font-weight: 300;
    margin: 0 0 36px;
    letter-spacing: -0.025em;
    line-height: 1.18;
  }
  .philosophy-body {
    font-size: 17px;
    color: var(--sumi-2);
    font-weight: 300;
    line-height: 1.7;
    margin: 0 0 22px;
  }
  .philosophy-note {
    font-size: 15px;
    color: var(--sumi-2);
    line-height: 1.75;
    margin: 0;
  }

  /* ── Privacy ───────────────────────────────────────── */
  .privacy { padding: 120px 56px; background: var(--paper); }
  .privacy-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    gap: 64px;
    align-items: start;
  }
  .privacy-heading {
    font-size: 40px;
    font-weight: 300;
    margin: 0;
    letter-spacing: -0.025em;
    line-height: 1.15;
  }
  .privacy-items {
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .privacy-item {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 22px;
    padding-bottom: 32px;
  }
  .privacy-item.bordered { border-bottom: var(--hairline); }
  .privacy-kanji { font-size: 32px; color: var(--sumi-2); }
  .privacy-title {
    font-size: 20px;
    margin-bottom: 10px;
    letter-spacing: -0.01em;
  }
  .privacy-text {
    font-size: 15px;
    color: var(--sumi-2);
    line-height: 1.65;
  }

  /* ── Pricing ───────────────────────────────────────── */
  .pricing {
    border-top: var(--hairline);
    border-bottom: var(--hairline);
    padding: 120px 56px;
    background: var(--paper-2);
    text-align: center;
  }
  .pricing-inner { max-width: 760px; margin: 0 auto; }
  .pricing-heading {
    font-size: 56px;
    font-weight: 300;
    margin: 0 0 24px;
    letter-spacing: -0.025em;
    line-height: 1.05;
  }
  .pricing-body {
    font-size: 17px;
    color: var(--sumi-2);
    font-weight: 300;
    line-height: 1.65;
    margin: 0;
  }

  /* ── FAQ ────────────────────────────────────────────── */
  .faq { padding: 120px 56px; }
  .faq-inner { max-width: 960px; margin: 0 auto; }
  .faq-heading {
    font-size: 44px;
    font-weight: 300;
    margin: 0 0 48px;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .faq-cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
  }
  .faq-card {
    padding: 28px;
    border: var(--hairline);
    border-radius: 12px;
    background: var(--paper-2);
  }
  .faq-card-q {
    font-size: 18px;
    font-weight: 400;
    margin-bottom: 12px;
    letter-spacing: -0.01em;
  }
  .faq-card-a {
    font-size: 14px;
    color: var(--sumi-2);
    line-height: 1.65;
  }
  .faq-more {
    display: inline-block;
    margin-top: 32px;
    font-size: 14px;
    color: var(--sumi-2);
    text-decoration: none;
    transition: color 0.15s;
  }
  .faq-more:hover { color: var(--shu); }

  /* ── Support ───────────────────────────────────────── */
  .support {
    border-top: var(--hairline);
    background: var(--paper-2);
    padding: 100px 56px;
    text-align: center;
  }
  .support-inner { max-width: 720px; margin: 0 auto; }
  .support-heading {
    font-size: 32px;
    font-weight: 300;
    margin: 0 0 22px;
    letter-spacing: -0.02em;
    line-height: 1.25;
  }
  .support-body {
    font-size: 14px;
    color: var(--sumi-2);
    line-height: 1.7;
    margin: 0 0 32px;
  }

  /* ── Footer ────────────────────────────────────────── */
  .footer {
    padding: 56px;
    font-size: 12px;
    color: var(--sumi-3);
    border-top: var(--hairline);
  }
  .footer-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 64px;
  }
  .footer-logo {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 12px;
  }
  .footer-desc {
    font-size: 11px;
    color: var(--sumi-3);
    max-width: 280px;
    line-height: 1.6;
  }
  .footer-version {
    font-size: 10px;
    color: var(--sumi-4);
    margin-top: 14px;
  }
  .footer-cols {
    display: flex;
    gap: 56px;
  }
  .footer-col-title {
    font-size: 9.5px;
    letter-spacing: 0.22em;
    color: var(--sumi-4);
    text-transform: uppercase;
    margin-bottom: 12px;
  }
  .footer-col-links {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .footer-link {
    font-size: 12px;
    color: var(--sumi-2);
    text-decoration: none;
  }
  .footer-link:hover { color: var(--sumi); }

  /* ── Responsive ────────────────────────────────────── */
  @media (max-width: 900px) {
    .nav-inner { padding: 20px 24px; }
    .nav-links { gap: 20px; font-size: 12px; }
    .hero { padding: 48px 24px 32px; }
    .hero-kanji { font-size: 180px; right: 24px; }
    .hero-heading { font-size: 48px; }
    .hero-sub { font-size: 16px; }
    .stats { padding: 24px; }
    .stats-inner { grid-template-columns: repeat(2, 1fr); gap: 20px; }
    .what-it-is { padding: 80px 24px; }
    .what-inner { grid-template-columns: 1fr; gap: 32px; }
    .how-it-works { padding: 80px 24px; }
    .how-heading { font-size: 36px; }
    .steps-grid { grid-template-columns: 1fr; gap: 24px; }
    .gallery { padding: 80px 24px 40px; }
    .gallery-heading { font-size: 36px; }
    .gallery-item, .gallery-item.reverse { grid-template-columns: 1fr; }
    .gallery-item.reverse .gallery-screen { order: 0; }
    .gallery-item.reverse .gallery-caption { order: 1; }
    .philosophy { padding: 100px 24px; }
    .philosophy-kanji { font-size: 280px; }
    .philosophy-heading { font-size: 32px; }
    .privacy { padding: 80px 24px; }
    .privacy-inner { grid-template-columns: 1fr; gap: 40px; }
    .pricing { padding: 80px 24px; }
    .pricing-heading { font-size: 36px; }
    .faq { padding: 80px 24px; }
    .faq-heading { font-size: 32px; }
    .faq-cards { grid-template-columns: 1fr; }
    .support { padding: 80px 24px; }
    .footer { padding: 40px 24px; }
    .footer-inner { flex-direction: column; gap: 32px; }
    .footer-cols { gap: 32px; }
  }
</style>
