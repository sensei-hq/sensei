// VARIANT A — "Same world as the app"
// ─────────────────────────────────────────────────────────────────
// Brief: the website should feel like a quieter wing of the app —
// washi paper, sumi ink, kanji, generous breathing room, small type,
// hairline rules, almost no color. Restraint as marketing.
//
// Color/surface/border come from utility classes (bg-paper, text-ink-soft,
// border-t, …). Only layout/size and dynamic (hover/conditional) styles
// stay inline. The ink ramp reads default → soft → mute → faint.

const { useState: aS, useEffect: aE } = React;

function VariantA() {
  return (
    <div className="sensei variant-a bg-paper text-ink" style={{
      minHeight: '100%'
    }}>
      <NavA/>
      <HeroA/>
      <WhatItIsA/>
      <HowItWorksA/>
      <GalleryA/>
      <PhilosophyA/>
      <PrivacyA/>
      <PricingA/>
      <FaqA/>
      <SupportA/>
      <FooterA/>
    </div>
  );
}

// ─── Nav ────────────────────────────────────────────────────────
function NavA() {
  return (
    <nav style={{
      maxWidth: 1100, margin: '0 auto',
      padding: '28px 48px',
      display: 'flex', alignItems: 'center',
      justifyContent: 'space-between'
    }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span className="kanji text-accent" style={{ fontSize: 20,
                       letterSpacing: '-0.04em' }}>先生</span>
        <span className="display text-ink" style={{ fontSize: 17,
                       letterSpacing: '-0.01em' }}>Sensei</span>
      </div>
      <div style={{ display: 'flex', gap: 28, fontSize: 12 }}>
        {[
          ['#how', 'How it works'],
          ['#gallery', 'Screens'],
          ['#philosophy', 'Philosophy'],
          ['#privacy', 'Privacy'],
          ['#faq', 'FAQ']
        ].map(([href, label]) => (
          <a key={href} href={href} className="text-ink-soft"
             style={{ textDecoration: 'none', transition: 'color .15s' }}
             onMouseEnter={(e) => e.currentTarget.style.color = 'var(--ink)'}
             onMouseLeave={(e) => e.currentTarget.style.color = ''}>
            {label}
          </a>
        ))}
      </div>
    </nav>
  );
}

// ─── Hero ───────────────────────────────────────────────────────
function HeroA() {
  return (
    <section style={{ maxWidth: 1100, margin: '0 auto',
                       padding: '40px 48px 80px' }}>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr',
                     gap: 28, alignItems: 'start' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 12,
                         marginBottom: 24 }}>
            <span className="kanji text-accent" style={{ fontSize: 56,
                           lineHeight: 1 }}>観</span>
            <div className="text-ink-mute" style={{ fontSize: 10,
                           letterSpacing: '0.22em',
                           textTransform: 'uppercase' }}>
              Kan · to observe
            </div>
          </div>
          <h1 className="display" style={{
            fontSize: 56, fontWeight: 300, lineHeight: 1.1,
            letterSpacing: '-0.025em',
            margin: 0, maxWidth: 820
          }}>
            A quiet companion for AI-assisted work.
          </h1>
          <p className="text-ink-soft" style={{ fontSize: 16,
                       lineHeight: 1.6, marginTop: 24, maxWidth: 560 }}>
            Sensei watches your sessions with AI assistants —
            then surfaces the patterns you're too close to see. Not a
            chatbot. Not a copilot. A patient observer.
          </p>
          <div style={{ display: 'flex', gap: 14, alignItems: 'center',
                         marginTop: 36 }}>
            <DownloadCTA/>
            <a href="#how" className="text-ink-soft" style={{ fontSize: 13 }}>
              See how it works ↓
            </a>
          </div>
          <div className="text-ink-mute" style={{ fontSize: 10.5,
                         marginTop: 14 }}>
            Free · Local-first · No account
          </div>
        </div>

        {/* Hero screen — centered, generous margin */}
        <div style={{ marginTop: 24, display: 'flex',
                       justifyContent: 'center' }}>
          <MockToday width={900} height={560}/>
        </div>
      </div>
    </section>
  );
}

// Auto-detected OS download button — single CTA per the answers.
function DownloadCTA({ size = "lg" }) {
  const [os, setOs] = aS("macOS");
  aE(() => {
    const ua = navigator.userAgent || "";
    if (/Win/.test(ua))         setOs("Windows");
    else if (/Linux/.test(ua))  setOs("Linux");
    else if (/Mac/.test(ua))    setOs("macOS");
  }, []);
  const px = size === "lg" ? '14px 26px' : '10px 18px';
  const fs = size === "lg" ? 14 : 12;
  return (
    <a href={`#download-${os.toLowerCase()}`} className="bg-ink"
       style={{
        display: 'inline-flex', alignItems: 'center', gap: 10,
        padding: px,
        borderRadius: 6,
        fontSize: fs,
        fontWeight: 500,
        textDecoration: 'none'
      }}>
      <span className="kanji text-accent" style={{ fontSize: fs + 2 }}>下</span>
      Download for {os}
    </a>
  );
}

// ─── What it is ─────────────────────────────────────────────────
function WhatItIsA() {
  return (
    <section className="border-t" style={{ padding: '80px 48px' }}>
      <div style={{ maxWidth: 1100, margin: '0 auto',
                     display: 'grid',
                     gridTemplateColumns: '1fr 1.4fr',
                     gap: 48, alignItems: 'start' }}>
        <div>
          <div className="text-ink-mute" style={{ fontSize: 10,
                         letterSpacing: '0.22em',
                         textTransform: 'uppercase', marginBottom: 12 }}>
            What it is
          </div>
          <h2 className="display" style={{ fontSize: 28, fontWeight: 400,
                         margin: 0, letterSpacing: '-0.015em',
                         lineHeight: 1.25 }}>
            One desktop app. One quiet promise.
          </h2>
        </div>
        <div className="text-ink-soft" style={{ fontSize: 15, lineHeight: 1.7 }}>
          <p style={{ marginTop: 0 }}>
            Sensei runs on your machine and observes your sessions with AI
            assistants. It logs nothing remotely; it speaks rarely; it remembers
            what you've actually done.
          </p>
          <p>
            Over weeks, it begins to recognize your patterns — the
            idioms you gravitate toward, the workarounds you've adopted,
            the friction points that keep recurring. When something
            looks worth noticing, it tells you. The rest of the time, it
            stays out of the way.
          </p>
        </div>
      </div>
    </section>
  );
}

// ─── How it works (Watch → Notice → Adopt) ──────────────────────
function HowItWorksA() {
  const steps = [
    { kanji: "観", phase: "Watch",
      text: "Sensei sits beside your editor and AI tools, capturing the shape of each session — the prompts, the responses, the corrections.",
      sub: "Local only. Nothing leaves your machine." },
    { kanji: "察", phase: "Notice",
      text: "After a few days, patterns begin to surface. Recurring frictions. Idioms forming. Things you taught the assistant once and may want to teach it again.",
      sub: "You decide what's signal and what isn't." },
    { kanji: "覚", phase: "Adopt",
      text: "Worthy patterns become memories — small, named lessons sensei can apply to future sessions on your behalf, with your blessing.",
      sub: "Adopt, refine, or dismiss. Always your call." }
  ];
  return (
    <section id="how" className="border-t" style={{ padding: '80px 48px' }}>
      <div style={{ maxWidth: 1100, margin: '0 auto' }}>
        <div className="text-ink-mute" style={{ fontSize: 10,
                       letterSpacing: '0.22em',
                       textTransform: 'uppercase', marginBottom: 12 }}>
          How it works
        </div>
        <h2 className="display" style={{ fontSize: 36, fontWeight: 300,
                       margin: '0 0 56px', letterSpacing: '-0.02em' }}>
          観 · 察 · 覚 — watch, notice, adopt.
        </h2>
        <div style={{ display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)',
                       gap: 56 }}>
          {steps.map((s, i) => (
            <div key={i}>
              <div style={{ display: 'flex', alignItems: 'baseline',
                             gap: 12, marginBottom: 18 }}>
                <span className="kanji text-accent" style={{ fontSize: 38,
                               lineHeight: 1 }}>
                  {s.kanji}
                </span>
                <div className="text-ink-mute" style={{ fontSize: 10,
                               letterSpacing: '0.22em',
                               textTransform: 'uppercase' }}>
                  {s.phase}
                </div>
              </div>
              <div className="text-ink" style={{ fontSize: 14,
                             lineHeight: 1.65, marginBottom: 14 }}>
                {s.text}
              </div>
              <div className="text-ink-mute" style={{ fontSize: 11,
                             fontStyle: 'italic' }}>
                {s.sub}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── Gallery ────────────────────────────────────────────────────
function GalleryA() {
  const screens = [
    { caption: "Today — the morning briefing",
      sub: "Sensei surfaces one observation that's worth your attention. Everything else stays out of sight.",
      el: <MockToday width={780} height={490}/> },
    { caption: "Sessions — the week in review",
      sub: "Going well, not going well, things noticed. Three lanes, no charts to decode.",
      el: <MockSessions width={780} height={490}/> },
    { caption: "Insights — what sensei has noticed",
      sub: "Patterns sensei is tracking, with confidence and provenance. You decide which become memories.",
      el: <MockInsights width={780} height={490}/> },
    { caption: "Memories — adopted teachings",
      sub: "Each memory is named, dated, and traceable to the sessions it came from. No black box.",
      el: <MockMemory width={780} height={490}/> },
    { caption: "Instruments — your tools, observed",
      sub: "Try MCP tools in isolation, replay what the assistant did, watch toolset health over time.",
      el: <MockInstruments width={780} height={490}/> }
  ];
  return (
    <section id="gallery" className="border-t bg-paper-soft"
             style={{ padding: '80px 0 40px' }}>
      <div style={{ maxWidth: 1100, margin: '0 auto', padding: '0 48px' }}>
        <div className="text-ink-mute" style={{ fontSize: 10,
                       letterSpacing: '0.22em',
                       textTransform: 'uppercase', marginBottom: 12 }}>
          The screens
        </div>
        <h2 className="display" style={{ fontSize: 36, fontWeight: 300,
                       margin: '0 0 12px', letterSpacing: '-0.02em' }}>
          Five surfaces, one rhythm.
        </h2>
        <p className="text-ink-soft" style={{ fontSize: 14,
                     maxWidth: 560, lineHeight: 1.6, margin: '0 0 56px' }}>
          Every screen answers one question and stays quiet otherwise.
        </p>
      </div>
      <div style={{ maxWidth: 1100, margin: '0 auto', padding: '0 48px',
                     display: 'flex', flexDirection: 'column', gap: 80 }}>
        {screens.map((s, i) => (
          <div key={i} style={{
            display: 'grid',
            gridTemplateColumns: i % 2 === 0 ? '1fr 320px' : '320px 1fr',
            gap: 56, alignItems: 'center'
          }}>
            <div style={{ order: i % 2 === 0 ? 0 : 1 }}>{s.el}</div>
            <div style={{ order: i % 2 === 0 ? 1 : 0 }}>
              <div className="display" style={{ fontSize: 22, fontWeight: 400,
                             marginBottom: 10 }}>{s.caption}</div>
              <div className="text-ink-soft" style={{ fontSize: 13,
                             lineHeight: 1.65 }}>{s.sub}</div>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

// ─── Philosophy ─────────────────────────────────────────────────
function PhilosophyA() {
  return (
    <section id="philosophy" className="border-t" style={{ padding: '120px 48px' }}>
      <div style={{ maxWidth: 760, margin: '0 auto', textAlign: 'center' }}>
        <span className="kanji text-accent" style={{ fontSize: 80,
                       lineHeight: 1 }}>静</span>
        <div className="text-ink-mute" style={{ fontSize: 10,
                       letterSpacing: '0.22em',
                       textTransform: 'uppercase', marginTop: 14 }}>
          Sei · stillness
        </div>
        <h2 className="display" style={{ fontSize: 30, fontWeight: 300,
                       margin: '40px 0 28px', letterSpacing: '-0.02em',
                       lineHeight: 1.3 }}>
          The master observes for a long time before teaching.
        </h2>
        <p className="text-ink-soft" style={{ fontSize: 14,
                     lineHeight: 1.75, margin: 0 }}>
          AI tools are getting louder. More suggestions, more autocompletes,
          more interrupting. Sensei moves the other way. It speaks rarely,
          and only when it has something specific to say. Most days it is
          completely silent — and that is the feature.
        </p>
        <p className="text-ink-soft" style={{ fontSize: 14,
                     lineHeight: 1.75, marginTop: 18 }}>
          The kanji throughout the app are not decoration. Each one names
          a phase of practice — observation, recognition, adoption,
          refinement. They are what we ask of the user, and what we ask
          of ourselves as the people who built this.
        </p>
      </div>
    </section>
  );
}

// ─── Privacy ────────────────────────────────────────────────────
function PrivacyA() {
  return (
    <section id="privacy" className="border-t bg-paper"
             style={{ padding: '80px 48px' }}>
      <div style={{ maxWidth: 1100, margin: '0 auto',
                     display: 'grid',
                     gridTemplateColumns: '1fr 1.4fr',
                     gap: 48, alignItems: 'start' }}>
        <div>
          <span className="kanji text-accent" style={{ fontSize: 36 }}>蔵</span>
          <div className="text-ink-mute" style={{ fontSize: 10,
                         letterSpacing: '0.22em',
                         textTransform: 'uppercase',
                         marginTop: 12, marginBottom: 12 }}>
            Privacy & local-first
          </div>
          <h2 className="display" style={{ fontSize: 28, fontWeight: 400,
                         margin: 0, letterSpacing: '-0.015em',
                         lineHeight: 1.25 }}>
            Your sessions stay on your machine.
          </h2>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 22 }}>
          {[
            { k: "蔵", title: "Local storage only",
              text: "Transcripts, patterns, memories — all stored in a SQLite file under your home directory. Sensei never makes outbound network requests." },
            { k: "鍵", title: "No telemetry",
              text: "We don't track usage. Updates are checked manually from Help → Check for Updates." },
            { k: "破", title: "Easy to delete",
              text: "One folder. Delete it and sensei forgets everything. Export to JSON anytime." }
          ].map((it, i) => (
            <div key={i} style={{ display: 'grid',
                       gridTemplateColumns: 'auto 1fr', gap: 16,
                       paddingBottom: 22,
                       borderBottom: i < 2 ? 'var(--hairline)' : 'none' }}>
              <span className="kanji text-ink-soft" style={{ fontSize: 22 }}>{it.k}</span>
              <div>
                <div className="display" style={{ fontSize: 16,
                               marginBottom: 6 }}>{it.title}</div>
                <div className="text-ink-soft" style={{ fontSize: 13,
                               lineHeight: 1.6 }}>{it.text}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── Pricing ────────────────────────────────────────────────────
function PricingA() {
  return (
    <section className="border-t" style={{ padding: '80px 48px',
                       textAlign: 'center' }}>
      <div style={{ maxWidth: 720, margin: '0 auto' }}>
        <div className="text-ink-mute" style={{ fontSize: 10,
                       letterSpacing: '0.22em',
                       textTransform: 'uppercase', marginBottom: 14 }}>
          Pricing
        </div>
        <h2 className="display" style={{ fontSize: 36, fontWeight: 300,
                       margin: '0 0 16px', letterSpacing: '-0.02em' }}>
          Free. Pay what feels right.
        </h2>
        <p className="text-ink-soft" style={{ fontSize: 14,
                     lineHeight: 1.7, margin: 0 }}>
          Sensei is free to download and use forever. If it earns a place
          in your daily practice, you can support development below — but
          there's no nag, no trial, no upgrade prompt. Ever.
        </p>
        <div style={{ marginTop: 36 }}>
          <DownloadCTA/>
        </div>
      </div>
    </section>
  );
}

// ─── FAQ ────────────────────────────────────────────────────────
function FaqA() {
  const qs = [
    { q: "Which AI assistants does it observe?",
      a: "Any AI assistant that speaks the Model Context Protocol. The list grows as MCP grows." },
    { q: "Does sensei see my code?",
      a: "Only what passes through your AI tool's session. It runs locally and stores everything in a SQLite file you can inspect or delete at any time." },
    { q: "Will it slow down my machine?",
      a: "Sensei is a Tauri app — small binary, low memory. The observer is event-driven; it only does work when a session happens." },
    { q: "Can I export my memories?",
      a: "Yes. Settings → Export gives you a JSON dump of every pattern, memory, and adopted teaching. Import is also supported." },
    { q: "What's the long-term plan?",
      a: "Sensei stays local-first and free. We may add an optional paid tier later for cross-machine sync, but the core promise — quiet, local, observant — never changes." }
  ];
  return (
    <section id="faq" className="border-t" style={{ padding: '80px 48px' }}>
      <div style={{ maxWidth: 880, margin: '0 auto' }}>
        <div className="text-ink-mute" style={{ fontSize: 10,
                       letterSpacing: '0.22em',
                       textTransform: 'uppercase', marginBottom: 12 }}>
          Frequently asked
        </div>
        <h2 className="display" style={{ fontSize: 28, fontWeight: 400,
                       margin: '0 0 40px', letterSpacing: '-0.015em' }}>
          Common questions, plain answers.
        </h2>
        <div>
          {qs.map((it, i) => (
            <details key={i} className="border-t" style={{
              padding: '20px 0',
              ...(i === qs.length - 1 ? { borderBottom: 'var(--hairline)' } : {})
            }}>
              <summary className="text-ink" style={{
                cursor: 'pointer',
                listStyle: 'none',
                display: 'flex', justifyContent: 'space-between',
                fontSize: 14
              }}>
                <span>{it.q}</span>
                <span className="kanji text-ink-mute">+</span>
              </summary>
              <div className="text-ink-soft" style={{ fontSize: 13,
                             lineHeight: 1.7, marginTop: 14, maxWidth: 640 }}>
                {it.a}
              </div>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── Support development ────────────────────────────────────────
function SupportA() {
  return (
    <section className="border-t bg-paper-soft" style={{ padding: '80px 48px',
                       textAlign: 'center' }}>
      <div style={{ maxWidth: 640, margin: '0 auto' }}>
        <span className="kanji text-accent" style={{ fontSize: 32 }}>志</span>
        <div className="text-ink-mute" style={{ fontSize: 10,
                       letterSpacing: '0.22em',
                       textTransform: 'uppercase',
                       marginTop: 10, marginBottom: 12 }}>
          Support development
        </div>
        <h2 className="display" style={{ fontSize: 24, fontWeight: 400,
                       margin: '0 0 18px', letterSpacing: '-0.015em',
                       lineHeight: 1.3 }}>
          If sensei has earned a place in your practice, you can help keep it growing.
        </h2>
        <p className="text-ink-soft" style={{ fontSize: 13,
                     lineHeight: 1.7, margin: '0 0 28px' }}>
          Sensei is built by a small team. Every coffee buys an hour of focused work.
        </p>
        <a href="#sponsor" className="border-ink text-ink" style={{
          display: 'inline-flex', alignItems: 'center', gap: 8,
          padding: '12px 22px',
          borderRadius: 6,
          fontSize: 13,
          textDecoration: 'none'
        }}>
          ♥ Buy me a coffee
        </a>
      </div>
    </section>
  );
}

// ─── Footer ─────────────────────────────────────────────────────
function FooterA() {
  return (
    <footer className="border-t text-ink-mute" style={{ padding: '40px 48px',
                      fontSize: 11 }}>
      <div style={{ maxWidth: 1100, margin: '0 auto',
                     display: 'flex', alignItems: 'center',
                     justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
          <span className="kanji text-accent" style={{ fontSize: 13,
                         letterSpacing: '-0.04em' }}>先生</span>
          <span className="display text-ink-soft" style={{ fontSize: 13 }}>Sensei</span>
          <span className="mono" style={{ fontSize: 10,
                         marginLeft: 12 }}>v0.4.2</span>
        </div>
        <div style={{ display: 'flex', gap: 22 }}>
          <a href="#privacy">Privacy</a>
          <a href="#faq">FAQ</a>
          <a href="#github">GitHub</a>
          <a href="#twitter">Twitter</a>
        </div>
      </div>
    </footer>
  );
}

Object.assign(window, { VariantA });
