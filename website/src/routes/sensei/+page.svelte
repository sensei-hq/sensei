<script lang="ts">
  import { browser } from '$app/environment';
  import { base } from '$app/paths';
  import { vibe } from '@rokkit/states';
  import { Button } from '@rokkit/ui';
  import { heroBrief, surfaces } from '$lib/sensei-surfaces-data';
  import type {
    AnatomyRowsMech,
    CardsMech,
    ChipsMech,
    FlowMech,
    HeroBrief,
    LanesMech,
    Mechanic,
    ModesMech,
    PillsMech,
    Surface,
    Tone
  } from '$lib/sensei-surfaces-data';

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
    ['#gallery', 'Surfaces'],
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

  // ── Surface colour maps ─────────────────────────────────────
  // Tone keys → text-<token> classes (literal strings so UnoCSS emits them)
  // and → var(--<token>) for the data-driven accent borders on the lanes.
  const toneText: Record<Tone, string> = {
    accent: 'text-accent',
    success: 'text-success',
    warning: 'text-warning',
    'ink-soft': 'text-ink-soft',
    'ink-mute': 'text-ink-mute',
  };
  const toneVar: Record<Tone, string> = {
    accent: 'var(--accent)',
    success: 'var(--success)',
    warning: 'var(--warning)',
    'ink-soft': 'var(--ink-soft)',
    'ink-mute': 'var(--ink-mute)',
  };
  const priorityTone: Record<'P0' | 'P1' | 'P2', Tone> = {
    P0: 'accent',
    P1: 'warning',
    P2: 'ink-mute',
  };

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
      { label: 'FAQ', href: `${base}/sensei/faq` },
      { label: 'Docs', href: `${base}/sensei/docs` },
      { label: 'Changelog', href: `${GITHUB}/releases` },
    ]},
    { title: 'Legal', links: [
      { label: 'Privacy', href: `${base}/privacy` },
      { label: 'Terms', href: `${base}/terms` },
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
    <div class="nav-inner py-6 px-12">
      <a href="{base}/sensei" class="logo-link gap-2.5">
        <span class="i-brand:sensei text-sensei logo-mark" aria-hidden="true"></span>
        <span class="display logo-text">Sensei</span>
      </a>
      <div class="nav-links gap-8">
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
  <section class="hero pt-16 px-12 pb-8">
    <div class="hero-kanji kanji">観</div>
    <div class="hero-content">
      <div class="hero-tagline gap-3.5">
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
      <div class="hero-actions gap-4">
        <Button href="{RELEASE_BASE}/{dlFile}" variant="primary" size="lg">
          <span class="kanji text-lg" style="line-height: 1">下</span>
          Download for {os}
        </Button>
        <a href="#how" class="hero-link">See how it works ↓</a>
      </div>
      <div class="hero-note">Free preview · Local-first · No account required</div>
    </div>
    <div class="hero-screen">
      {@render heroBriefCard(heroBrief)}
    </div>
  </section>

  <!-- ═══ Stats ═══ -->
  <section class="stats py-8 px-12">
    <div class="stats-inner gap-8">
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
  <section class="what-it-is py-30 px-12">
    <div class="what-inner gap-20">
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
  <section id="how" class="how-it-works py-30 px-12">
    <div class="how-inner">
      <div class="section-tag">How it works</div>
      <h2 class="display how-heading mb-18">
        <span style="color: var(--shu);">観 · 察 · 覚</span><br/>
        Watch, notice, adopt.
      </h2>
      <div class="steps-grid gap-16">
        {#each steps as s}
          <div class="step-card py-8 px-6">
            <div class="kanji step-kanji">{s.kanji}</div>
            <div class="step-phase">{s.phase}</div>
            <h3 class="display step-title mb-4">{s.title}</h3>
            <div class="step-text">{s.text}</div>
            <div class="step-sub">{s.sub}</div>
          </div>
        {/each}
      </div>
    </div>
  </section>

  <!-- ═══ Surfaces (#gallery) ═══ -->
  <section id="gallery" class="gallery pt-30 px-12 pb-16">
    <div class="gallery-inner">
      <div class="section-tag">The screens · 面</div>
      <h2 class="display gallery-heading mb-4">
        Five surfaces,<br/>one rhythm.
      </h2>
      <p class="gallery-sub display mb-8">
        Each surface answers one question and stays quiet otherwise. The pixels will keep
        changing; what they're <em>for</em> won't. So here is what each one does, why it's
        shaped that way, and how its flow moves.
      </p>
      <div class="surfaces">
        {#each surfaces as surf (surf.n)}
          {@render surfaceBlock(surf)}
        {/each}
      </div>
    </div>
  </section>

  <!-- ═══ Philosophy ═══ -->
  <section id="philosophy" class="philosophy py-40 px-12">
    <div class="philosophy-kanji kanji">静</div>
    <div class="philosophy-inner">
      <div class="section-tag">Sei · stillness</div>
      <h2 class="display philosophy-heading mb-9">
        The master observes for a long time before teaching.
      </h2>
      <p class="philosophy-body display mb-5">
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
  <section id="privacy" class="privacy py-30 px-12">
    <div class="privacy-inner gap-16">
      <div class="privacy-left">
        <span class="kanji text-4xl" style="color: var(--shu)">蔵</span>
        <div class="section-tag mt-4">Privacy & local-first</div>
        <h2 class="display privacy-heading">
          Your sessions stay on your machine.
        </h2>
      </div>
      <div class="privacy-items gap-8">
        {#each privacyItems as it, i}
          <div class="privacy-item gap-5" class:bordered={i < 2}>
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
  <section class="pricing py-30 px-12">
    <div class="pricing-inner">
      <div class="section-tag">Pricing</div>
      <h2 class="display pricing-heading mb-6">
        Free during preview.
      </h2>
      <p class="pricing-body display">
        Sensei is in early preview — we're learning what works and what
        doesn't. It's free while we figure that out. If we move to a paid
        tier, early adopters and supporters get a permanent discount.
        No surprises.
      </p>
      <div class="mt-11">
        <Button href="{RELEASE_BASE}/{dlFile}" variant="primary" size="lg">
          <span class="kanji text-lg" style="line-height: 1">下</span>
          Download for {os}
        </Button>
      </div>
    </div>
  </section>

  <!-- ═══ FAQ Summary ═══ -->
  <section id="faq" class="faq py-30 px-12">
    <div class="faq-inner">
      <div class="section-tag">Quick answers</div>
      <h2 class="display faq-heading mb-12">
        The essentials.
      </h2>
      <div class="faq-cards gap-6">
        {#each [
          { q: 'What platforms?', a: 'Claude Code, Cursor, Windsurf, Copilot, Codex, Aider — anything that speaks MCP.' },
          { q: 'What\'s included?', a: '20 commands, 8 agents, skills, hooks, and MCP tools for code search, patterns, and call graph analysis.' },
          { q: 'Will it slow me down?', a: 'Rust daemon, event-driven. Ollama is optional and hardware-aware — it degrades gracefully.' },
          { q: 'Is it free?', a: 'Free during preview. If we move to a paid tier, early adopters get a permanent discount.' },
        ] as card}
          <div class="faq-card p-6">
            <div class="display faq-card-q">{card.q}</div>
            <div class="faq-card-a">{card.a}</div>
          </div>
        {/each}
      </div>
      <a href="{base}/sensei/faq" class="faq-more">All questions & answers →</a>
    </div>
  </section>

  <!-- ═══ Support ═══ -->
  <section class="support py-24 px-12">
    <div class="support-inner">
      <span class="kanji text-4xl" style="color: var(--shu)">志</span>
      <div class="section-tag mt-3.5">Support development</div>
      <h2 class="display support-heading mb-5">
        If sensei has earned a place in your practice, you can help keep it growing.
      </h2>
      <p class="support-body mb-8">
        Sensei is built by a small team. A sponsorship funds the focused hours
        that keep it sharp.
      </p>
      <Button href="https://github.com/sponsors/sensei-hq" target="_blank" rel="noopener" variant="primary" size="md">
        ♥ Sponsor on GitHub
      </Button>
    </div>
  </section>

  <!-- ═══ Footer ═══ -->
  <footer class="footer p-12">
    <div class="footer-inner gap-16">
      <div class="footer-brand">
        <div class="footer-logo gap-2.5">
          <span class="i-brand:sensei text-sensei" style="width:22px;height:22px;flex-shrink:0" aria-hidden="true"></span>
          <span class="display text-base" style="color: var(--sumi-2)">Sensei</span>
        </div>
        <div class="footer-desc">
          A patient observer for AI-assisted work. Built quietly,
          shipped slowly.
        </div>
        <div class="mono footer-version">v{__APP_VERSION__}</div>
      </div>
      <div class="footer-cols gap-14">
        {#each footerCols as col}
          <div>
            <div class="footer-col-title">{col.title}</div>
            <div class="footer-col-links gap-2">
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

<!-- ═══ Hero brief — the product's voice, not a screenshot ═══ -->
{#snippet heroBriefCard(h: HeroBrief)}
  <div class="hero-card w-full py-12 px-16 bg-paper-soft border border-paper-edge rounded-lg shadow-lg">
    <div class="mb-6 flex items-center justify-between">
      <span class="mono text-xs text-ink-mute">{h.meta}</span>
      <span class="surf-eyebrow text-xs text-ink-faint font-semibold">{h.metaRight}</span>
    </div>

    <div class="hero-focal gap-8 items-start">
      <div class="kanji text-accent hero-focal-kanji">{h.focalKanji}</div>
      <div>
        <div class="surf-eyebrow text-xs text-ink-mute mb-2">{h.eyebrow}</div>
        <div class="display text-2xl font-light text-ink hero-headline">{h.headline}</div>
        <p class="mt-3 mb-4 text-base text-ink-soft hero-lead">
          {h.leadBefore}<span class="mono text-sm">{h.leadCode}</span>{h.leadAfter}
        </p>
        <div class="gap-4 flex items-center flex-wrap">
          <span class="inline-flex items-center gap-2 text-sm text-accent">
            <span class="hero-dot rounded-full bg-accent"></span>
            {h.projected}
          </span>
          <span class="flex-1"></span>
          <span class="mono text-xs text-ink-faint">{h.provenance}</span>
        </div>
      </div>
    </div>

    <div class="mt-8 pt-6 border-t border-paper-edge">
      <div class="surf-eyebrow text-xs text-ink-faint font-semibold mb-3">{h.secondaryLabel}</div>
      <div class="grid-3 gap-3">
        {#each h.secondary as s (s.label)}
          <div class="flex items-start gap-2">
            <span class="kanji text-base hero-sec-kanji {toneText[s.tone]}">{s.kanji}</span>
            <div class="min-w-0">
              <div class="surf-eyebrow text-xs text-ink-mute">{s.label}</div>
              <div class="mt-1 text-sm text-ink-soft">{s.text}</div>
              <span class="mono text-xs {toneText[s.tone]}">{s.tag}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>
{/snippet}

<!-- ═══ Surface block — identity → rationale → mechanic ═══ -->
{#snippet surfaceBlock(s: Surface)}
  <div class="surf-block pt-16 pb-16 border-t border-paper-edge">
    <div class="surf-grid gap-16 items-start">
      <div>
        <div class="gap-3 mb-4 flex items-baseline">
          <span class="mono text-accent text-sm surf-n">{s.n}</span>
          <span class="kanji text-accent text-2xl surf-id-kanji">{s.kanji}</span>
          <div class="gap-1 flex flex-col">
            <span class="display font-normal text-2xl surf-name">{s.name}</span>
            <span class="surf-eyebrow text-xs text-ink-mute font-semibold">{s.role}</span>
          </div>
        </div>
        <h3 class="display m-0 font-light text-ink text-xl surf-headline">{s.headline}</h3>
      </div>
      <div>
        <p class="display mt-0 mb-4 text-base text-ink-soft font-light surf-lead">
          {#each s.lead as seg (seg.t)}{#if seg.em}<span class="text-ink">{seg.t}</span>{:else}{seg.t}{/if}{/each}
        </p>
        <div class="surf-why pl-3 flex items-start gap-2.5">
          <span class="surf-eyebrow text-xs text-ink-mute font-semibold">Why</span>
          <span class="text-sm text-ink-soft italic surf-why-note">{s.why}</span>
        </div>
      </div>
    </div>
    <div class="mt-12 flex flex-col gap-6">
      {#each s.mechanics as mech (mech.kind)}
        {@render mechanic(mech)}
      {/each}
    </div>
  </div>
{/snippet}

{#snippet mechLabel(text: string)}
  <div class="surf-eyebrow text-xs text-ink-mute font-semibold mb-3">{text}</div>
{/snippet}

{#snippet mechanic(m: Mechanic)}
  {#if m.kind === 'anatomy-rows'}
    {@render anatomyRows(m)}
  {:else if m.kind === 'lanes'}
    {@render lanes(m)}
  {:else if m.kind === 'chips'}
    {@render chips(m)}
  {:else if m.kind === 'flow'}
    {@render flow(m)}
  {:else if m.kind === 'cards'}
    {@render cards(m)}
  {:else if m.kind === 'pills'}
    {@render pills(m)}
  {:else if m.kind === 'modes'}
    {@render modes(m)}
  {/if}
{/snippet}

{#snippet anatomyRows(m: AnatomyRowsMech)}
  <div>
    {@render mechLabel(m.label)}
    <div class="flex flex-col gap-2">
      {#each m.rows as r (r.label)}
        <div class="today-row gap-4 py-3 px-4 items-baseline bg-paper border border-paper-edge rounded-lg">
          <span class="kanji text-accent text-lg today-row-kanji">{r.kanji}</span>
          <div class="flex items-baseline gap-2">
            <span class="mono font-semibold text-xs {toneText[priorityTone[r.priority]]}">{r.priority}</span>
            <span class="display font-medium text-ink text-base">{r.label}</span>
          </div>
          <span class="text-sm text-ink-soft today-row-desc">{r.desc}</span>
        </div>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet lanes(m: LanesMech)}
  <div>
    {@render mechLabel(m.label)}
    <div class="grid-3 gap-3">
      {#each m.lanes as l (l.title)}
        <div class="lane py-4 px-4 bg-paper rounded-lg" style="border-top-color: {toneVar[l.tone]}">
          <div class="gap-2 mb-2 flex items-baseline">
            <span class="kanji text-lg {toneText[l.tone]}">{l.kanji}</span>
            <span class="surf-eyebrow text-xs text-ink-mute font-semibold">{l.title}</span>
          </div>
          <div class="text-sm text-ink-soft lane-desc">{l.desc}</div>
        </div>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet chips(m: ChipsMech)}
  <div>
    {@render mechLabel(m.label)}
    <div class="flex flex-wrap gap-2">
      {#each m.chips as c (c.title)}
        <div class="chip py-3 px-4 flex items-start gap-2.5 bg-paper border border-paper-edge rounded-lg">
          <span class="kanji text-accent text-base chip-kanji">{c.kanji}</span>
          <div>
            <div class="display font-medium text-ink text-base">{c.title}</div>
            <div class="mt-1 text-xs text-ink-mute chip-desc">{c.desc}</div>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet flow(m: FlowMech)}
  <div>
    {@render mechLabel(m.label)}
    <div class="flex items-stretch">
      {#each m.steps as st, i (st.title)}
        {#if i > 0}
          <div class="flex items-center text-accent mono text-sm px-1.5">→</div>
        {/if}
        <div class="flow-step py-4 px-4 flex-1 bg-paper border border-paper-edge rounded-lg flex flex-col gap-1.5">
          <span class="mono text-accent text-xs">{(i + 1).toString().padStart(2, '0')}</span>
          <span class="display font-medium text-ink text-lg flow-step-title">{st.title}</span>
          <span class="surf-eyebrow text-xs text-ink-mute font-semibold">{st.who}</span>
          <span class="text-xs text-ink-soft flow-step-desc">{st.desc}</span>
        </div>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet cards(m: CardsMech)}
  <div>
    {@render mechLabel(m.label)}
    <div class="grid-3 gap-3">
      {#each m.cards as c (c.title)}
        <div class="py-4 px-4 bg-paper border border-paper-edge rounded-lg">
          <div class="display font-medium text-ink text-base card-title">{c.title}</div>
          <div class="mt-2 text-xs text-ink-mute card-desc">{c.desc}</div>
        </div>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet pills(m: PillsMech)}
  <div>
    {@render mechLabel(m.label)}
    <div class="flex flex-wrap gap-2 items-center">
      {#each m.pills as p (p.label)}
        <span class="py-2 px-4 inline-flex items-center gap-1.5 text-sm text-ink-soft bg-paper border border-paper-edge rounded-full">
          <span class="kanji text-accent text-base">{p.kanji}</span>{p.label}
        </span>
      {/each}
      <span class="pl-2 inline-flex items-center text-xs text-ink-mute italic">{m.note}</span>
    </div>
  </div>
{/snippet}

{#snippet modes(m: ModesMech)}
  <div>
    {@render mechLabel(m.label)}
    <div class="grid-3 gap-3">
      {#each m.modes as md (md.title)}
        <div class="py-6 px-4 bg-paper border border-paper-edge rounded-lg">
          <div class="kanji text-accent text-2xl mode-kanji">{md.kanji}</div>
          <div class="display mt-3 font-medium text-ink text-lg mode-title">{md.title}</div>
          <div class="mt-2 text-sm text-ink-soft mode-desc">{md.desc}</div>
        </div>
      {/each}
    </div>
  </div>
{/snippet}

<style>
  /* ── Base ──────────────────────────────────────────── */
  .site {
    background: var(--paper);
    color: var(--sumi);
    min-height: 100%;
    font-family: var(--font-ui);
  }

  .section-tag {
    font-size: var(--text-xs);
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
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .logo-link {
    display: flex;
    align-items: center;
    text-decoration: none;
  }
  .logo-mark {
    width: 30px;
    height: 30px;
    flex-shrink: 0;
  }
  .logo-text {
    font-size: var(--text-lg);
    letter-spacing: -0.01em;
    color: var(--sumi);
  }
  .nav-links {
    display: flex;
    font-size: var(--text-sm);
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
    margin-bottom: 24px;
  }
  .hero-tag-text {
    font-size: var(--text-xs);
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
    font-size: var(--text-lg);
    color: var(--sumi-2);
    line-height: 1.55;
    margin-top: 32px;
    max-width: 640px;
    font-weight: 300;
  }
  .hero-actions {
    display: flex;
    align-items: center;
    margin-top: 40px;
  }
  .hero-link {
    font-size: var(--text-sm);
    color: var(--sumi-2);
    text-decoration: none;
  }
  .hero-note {
    font-size: var(--text-xs);
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
    background: var(--paper-2);
  }
  .stats-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
  }
  .stat { text-align: center; }
  .stat-value {
    font-size: var(--text-2xl);
    font-weight: 400;
    color: var(--sumi);
  }
  .stat-label {
    font-size: var(--text-xs);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin-top: 4px;
  }

  /* ── What it is ────────────────────────────────────── */
  .what-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1.6fr;
    align-items: start;
  }
  .what-heading {
    font-size: var(--text-3xl);
    font-weight: 300;
    margin: 0;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .what-body {
    font-size: var(--text-lg);
    line-height: 1.65;
    color: var(--sumi-2);
    font-weight: 300;
  }
  .what-body p:first-child { margin-top: 6px; }

  /* ── How it works ──────────────────────────────────── */
  .how-it-works {
    border-top: var(--hairline);
    border-bottom: var(--hairline);
    background: var(--paper-2);
  }
  .how-inner { max-width: 1200px; margin: 0 auto; }
  .how-heading {
    font-size: var(--text-4xl);
    font-weight: 300;
    letter-spacing: -0.025em;
    line-height: 1.05;
  }
  .steps-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }
  .step-card {
    background: var(--paper);
    border: var(--hairline);
    border-radius: 12px;
    position: relative;
  }
  .step-kanji {
    font-size: var(--text-3xl);
    color: var(--shu);
    line-height: 1;
    margin-bottom: 18px;
  }
  .step-phase {
    font-size: var(--text-xs);
    letter-spacing: 0.22em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 10px;
  }
  .step-title {
    font-size: var(--text-xl);
    font-weight: 400;
    letter-spacing: -0.01em;
  }
  .step-text {
    font-size: var(--text-sm);
    color: var(--sumi-2);
    line-height: 1.65;
    margin-bottom: 20px;
  }
  .step-sub {
    font-size: var(--text-xs);
    color: var(--sumi-3);
    font-style: italic;
    padding-top: 12px;
    border-top: var(--hairline);
  }

  /* ── Surfaces (#gallery) ───────────────────────────── */
  .gallery-inner { max-width: 1200px; margin: 0 auto; }
  .gallery-heading {
    font-size: var(--text-4xl);
    font-weight: 300;
    letter-spacing: -0.025em;
    line-height: 1.05;
  }
  .gallery-sub {
    font-size: var(--text-lg);
    color: var(--sumi-2);
    max-width: 640px;
    line-height: 1.6;
    font-weight: 300;
  }

  /* Shared 3-up grid used by lanes / cards / modes / hero secondary. */
  .grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); }

  /* Recurring uppercase tracked label look (roles, mech labels, eyebrows). */
  .surf-eyebrow { text-transform: uppercase; letter-spacing: 0.16em; }

  /* Block frame: identity column + rationale column. */
  .surf-grid { display: grid; grid-template-columns: 1fr 1.25fr; }
  .surf-n { letter-spacing: 0.06em; }
  .surf-id-kanji { line-height: 1; }
  .surf-name { letter-spacing: -0.015em; line-height: 1; }
  .surf-headline { line-height: 1.25; letter-spacing: -0.01em; max-width: 380px; }
  .surf-lead { line-height: 1.65; }
  .surf-why { border-left: 2px solid var(--accent-soft); }
  .surf-why-note { line-height: 1.55; }

  /* Today — priority anatomy rows. */
  .today-row { display: grid; grid-template-columns: auto 200px 1fr; }
  .today-row-kanji { width: 22px; }
  .today-row-desc { line-height: 1.5; }

  /* Sessions — lanes (2px accent top edge, set inline per lane). */
  .lane { border: 1px solid var(--paper-edge); border-top-width: 2px; }
  .lane-desc { line-height: 1.55; }

  /* Sessions — how-a-session-is-read chips. */
  .chip { flex: 1 1 220px; }
  .chip-kanji { line-height: 1.2; }
  .chip-desc { line-height: 1.45; }

  /* Insights — flow steps. */
  .flow-step-title { letter-spacing: -0.01em; }
  .flow-step-desc { line-height: 1.5; }

  /* Memories — anatomy cards. */
  .card-title { letter-spacing: -0.005em; }
  .card-desc { line-height: 1.5; }

  /* Instruments — modes. */
  .mode-kanji { line-height: 1; }
  .mode-title { letter-spacing: -0.01em; }
  .mode-desc { line-height: 1.55; }

  /* Hero brief card. */
  .hero-card { max-width: 920px; }
  .hero-focal { display: grid; grid-template-columns: auto 1fr; }
  .hero-focal-kanji { font-size: 72px; line-height: 0.9; }
  .hero-headline { line-height: 1.15; }
  .hero-lead { line-height: 1.6; max-width: 560px; }
  .hero-dot { width: 5px; height: 5px; }
  .hero-sec-kanji { line-height: 1.2; }

  /* ── Philosophy ────────────────────────────────────── */
  .philosophy {
    border-top: var(--hairline);
    border-bottom: var(--hairline);
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
    font-size: var(--text-3xl);
    font-weight: 300;
    letter-spacing: -0.025em;
    line-height: 1.18;
  }
  .philosophy-body {
    font-size: var(--text-lg);
    color: var(--sumi-2);
    font-weight: 300;
    line-height: 1.7;
  }
  .philosophy-note {
    font-size: var(--text-base);
    color: var(--sumi-2);
    line-height: 1.75;
    margin: 0;
  }

  /* ── Privacy ───────────────────────────────────────── */
  .privacy { background: var(--paper); }
  .privacy-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    align-items: start;
  }
  .privacy-heading {
    font-size: var(--text-3xl);
    font-weight: 300;
    margin: 0;
    letter-spacing: -0.025em;
    line-height: 1.15;
  }
  .privacy-items {
    display: flex;
    flex-direction: column;
  }
  .privacy-item {
    display: grid;
    grid-template-columns: auto 1fr;
    padding-bottom: 32px;
  }
  .privacy-item.bordered { border-bottom: var(--hairline); }
  .privacy-kanji { font-size: var(--text-2xl); color: var(--sumi-2); }
  .privacy-title {
    font-size: var(--text-xl);
    margin-bottom: 10px;
    letter-spacing: -0.01em;
  }
  .privacy-text {
    font-size: var(--text-base);
    color: var(--sumi-2);
    line-height: 1.65;
  }

  /* ── Pricing ───────────────────────────────────────── */
  .pricing {
    border-top: var(--hairline);
    border-bottom: var(--hairline);
    background: var(--paper-2);
    text-align: center;
  }
  .pricing-inner { max-width: 760px; margin: 0 auto; }
  .pricing-heading {
    font-size: var(--text-4xl);
    font-weight: 300;
    letter-spacing: -0.025em;
    line-height: 1.05;
  }
  .pricing-body {
    font-size: var(--text-lg);
    color: var(--sumi-2);
    font-weight: 300;
    line-height: 1.65;
    margin: 0;
  }

  /* ── FAQ ────────────────────────────────────────────── */
  .faq-inner { max-width: 960px; margin: 0 auto; }
  .faq-heading {
    font-size: var(--text-3xl);
    font-weight: 300;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .faq-cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }
  .faq-card {
    border: var(--hairline);
    border-radius: 12px;
    background: var(--paper-2);
  }
  .faq-card-q {
    font-size: var(--text-lg);
    font-weight: 400;
    margin-bottom: 12px;
    letter-spacing: -0.01em;
  }
  .faq-card-a {
    font-size: var(--text-sm);
    color: var(--sumi-2);
    line-height: 1.65;
  }
  .faq-more {
    display: inline-block;
    margin-top: 32px;
    font-size: var(--text-sm);
    color: var(--sumi-2);
    text-decoration: none;
    transition: color 0.15s;
  }
  .faq-more:hover { color: var(--shu); }

  /* ── Support ───────────────────────────────────────── */
  .support {
    border-top: var(--hairline);
    background: var(--paper-2);
    text-align: center;
  }
  .support-inner { max-width: 720px; margin: 0 auto; }
  .support-heading {
    font-size: var(--text-2xl);
    font-weight: 300;
    letter-spacing: -0.02em;
    line-height: 1.25;
  }
  .support-body {
    font-size: var(--text-sm);
    color: var(--sumi-2);
    line-height: 1.7;
  }

  /* ── Footer ────────────────────────────────────────── */
  .footer {
    font-size: var(--text-xs);
    color: var(--sumi-3);
    border-top: var(--hairline);
  }
  .footer-inner {
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }
  .footer-logo {
    display: flex;
    align-items: center;
    margin-bottom: 12px;
  }
  .footer-desc {
    font-size: var(--text-xs);
    color: var(--sumi-3);
    max-width: 280px;
    line-height: 1.6;
  }
  .footer-version {
    font-size: var(--text-xs);
    color: var(--sumi-4);
    margin-top: 14px;
  }
  .footer-cols {
    display: flex;
  }
  .footer-col-title {
    font-size: var(--text-xs);
    letter-spacing: 0.22em;
    color: var(--sumi-4);
    text-transform: uppercase;
    margin-bottom: 12px;
  }
  .footer-col-links {
    display: flex;
    flex-direction: column;
  }
  .footer-link {
    font-size: var(--text-xs);
    color: var(--sumi-2);
    text-decoration: none;
  }
  .footer-link:hover { color: var(--sumi); }

  /* ── Responsive ────────────────────────────────────── */
  @media (max-width: 900px) {
    .nav-inner { padding: 16px 24px; }
    .nav-links { gap: 20px; font-size: var(--text-xs); }
    .hero { padding: 48px 24px 32px; }
    .hero-kanji { font-size: 180px; right: 24px; }
    .hero-heading { font-size: var(--text-3xl); }
    .hero-sub { font-size: var(--text-base); }
    .stats { padding: 24px; }
    .stats-inner { grid-template-columns: repeat(2, 1fr); gap: 20px; }
    .what-it-is { padding: 64px 24px; }
    .what-inner { grid-template-columns: 1fr; gap: 32px; }
    .how-it-works { padding: 64px 24px; }
    .how-heading { font-size: var(--text-3xl); }
    .steps-grid { grid-template-columns: 1fr; gap: 24px; }
    .gallery { padding: 64px 24px 32px; }
    .gallery-heading { font-size: var(--text-3xl); }
    .surf-grid { grid-template-columns: 1fr; gap: 40px; }
    .surf-headline { max-width: none; }
    .today-row { grid-template-columns: 1fr; gap: 8px; }
    .grid-3 { grid-template-columns: 1fr; }
    .hero-focal { grid-template-columns: 1fr; gap: 24px; }
    .hero-focal-kanji { font-size: var(--text-4xl); }
    .philosophy { padding: 96px 24px; }
    .philosophy-kanji { font-size: 280px; }
    .philosophy-heading { font-size: var(--text-2xl); }
    .privacy { padding: 64px 24px; }
    .privacy-inner { grid-template-columns: 1fr; gap: 40px; }
    .pricing { padding: 64px 24px; }
    .pricing-heading { font-size: var(--text-3xl); }
    .faq { padding: 64px 24px; }
    .faq-heading { font-size: var(--text-2xl); }
    .faq-cards { grid-template-columns: 1fr; }
    .support { padding: 64px 24px; }
    .footer { padding: 32px 24px; }
    .footer-inner { flex-direction: column; gap: 32px; }
    .footer-cols { gap: 32px; }
  }
</style>
