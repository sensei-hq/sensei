// VARIANT B — "Confident continuity"
// ─────────────────────────────────────────────────────────────────
// Brief: same palette, same fonts, same kanji vocabulary as the app —
// but turned up. Bigger display type, larger imagery, more shu where
// it earns its place, more rhythm between sections (paper / paper-2
// alternation, taller blocks). Still feels like the app, but louder.
//
// Reasoning:
//   · Variant A is the "honest mirror" of the app. It's beautiful but
//     can read as too quiet for first-time visitors who haven't
//     internalized the product yet.
//   · B keeps the same visual atoms, but uses scale, rhythm, and a
//     more generous use of shu (vermillion accents) to give a
//     marketing visitor more to grab onto without breaking continuity.
//   · The accent rationing is still strict: shu only on kanji that
//     mark phase / identity, on download CTAs, and on one or two
//     stat figures. Never on body copy, never on borders.

const { useState: bS, useEffect: bE } = React;

function VariantB() {
  return (
    <div className="sensei variant-b" style={{
      background: 'var(--paper)', color: 'var(--ink)',
      minHeight: '100%', fontFamily: 'var(--font-ui)'
    }}>
      <NavB/>
      <HeroB/>
      <StatsB/>
      <WhatItIsB/>
      <HowItWorksB/>
      <GalleryB/>
      <PhilosophyB/>
      <PrivacyB/>
      <PricingB/>
      <FaqB/>
      <SupportB/>
      <FooterB/>
    </div>
  );
}

function NavB() {
  return (
    <nav style={{
      maxWidth: 1200, margin: '0 auto',
      padding: '24px 48px',
      display: 'flex', alignItems: 'center',
      justifyContent: 'space-between',
      borderBottom: 'var(--hairline)'
    }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span className="kanji" style={{ fontSize: 22,
                       color: 'var(--accent)', letterSpacing: '-0.04em' }}>先生</span>
        <span className="display" style={{ fontSize: 22,
                       letterSpacing: '-0.01em',
                       color: 'var(--ink)' }}>Sensei</span>
      </div>
      <div style={{ display: 'flex', gap: 32, fontSize: 13,
                     alignItems: 'center' }}>
        {[
          ['#how', 'How'],
          ['#gallery', 'Screens'],
          ['#philosophy', 'Philosophy'],
          ['#privacy', 'Privacy'],
          ['#faq', 'FAQ']
        ].map(([href, label]) => (
          <a key={href} href={href}
             style={{ color: 'var(--ink-2)',
                       textDecoration: 'none',
                       transition: 'color .15s' }}
             onMouseEnter={(e) => e.currentTarget.style.color = 'var(--ink)'}
             onMouseLeave={(e) => e.currentTarget.style.color = 'var(--ink-2)'}>
            {label}
          </a>
        ))}
      </div>
    </nav>
  );
}

function HeroB() {
  return (
    <section style={{ padding: '64px 48px 32px',
                       maxWidth: 1200, margin: '0 auto',
                       position: 'relative' }}>
      {/* Single oversized kanji as backdrop */}
      <div style={{ position: 'absolute', right: 56, top: 24,
                     fontSize: 56, lineHeight: 1,
                     color: 'var(--accent-soft)',
                     pointerEvents: 'none' }}
           className="kanji">
        観
      </div>

      <div style={{ position: 'relative' }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 12,
                       marginBottom: 24 }}>
          <span className="ink-dot" style={{
            background: 'var(--accent)', width: 8, height: 8 }}/>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--ink-3)',
                         textTransform: 'uppercase' }}>
            Sensei · the patient observer
          </div>
        </div>
        <h1 className="display" style={{
          fontSize: 56, fontWeight: 300, lineHeight: 1.02,
          letterSpacing: '-0.03em',
          margin: 0, maxWidth: 920
        }}>
          A quiet companion<br/>
          for AI-assisted <em style={{ color: 'var(--accent)',
                          fontStyle: 'normal' }}>work</em>.
        </h1>
        <p style={{ fontSize: 22, color: 'var(--ink-2)',
                     lineHeight: 1.55, marginTop: 32, maxWidth: 640,
                     fontFamily: 'var(--font-display)', fontWeight: 300 }}>
          Sensei watches your sessions with AI assistants —
          then surfaces the patterns you're too close to see. Not a
          chatbot. Not a copilot. A patient observer.
        </p>
        <div style={{ display: 'flex', gap: 16, alignItems: 'center',
                       marginTop: 32 }}>
          <DownloadCTAB size="lg"/>
          <a href="#how" style={{ fontSize: 13,
                         color: 'var(--ink-2)' }}>
            See how it works ↓
          </a>
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)',
                       marginTop: 16,
                       letterSpacing: '0.05em' }}>
          Free · Local-first · No account required
        </div>
      </div>

      {/* Hero screen */}
      <div style={{ marginTop: 64, display: 'flex',
                     justifyContent: 'center', position: 'relative' }}>
        <MockToday width={1040} height={620}/>
      </div>
    </section>
  );
}

function DownloadCTAB({ size = "lg" }) {
  const [os, setOs] = bS("macOS");
  bE(() => {
    const ua = navigator.userAgent || "";
    if (/Win/.test(ua))         setOs("Windows");
    else if (/Linux/.test(ua))  setOs("Linux");
    else if (/Mac/.test(ua))    setOs("macOS");
  }, []);
  const px = size === "lg" ? '16px 28px' : '8px 16px';
  const fs = size === "lg" ? 15 : 12;
  return (
    <a href={`#download-${os.toLowerCase()}`}
       style={{
        display: 'inline-flex', alignItems: 'center', gap: 12,
        padding: px,
        background: 'var(--ink)',
        color: 'var(--paper)',
        borderRadius: 8,
        fontSize: fs,
        fontWeight: 500,
        textDecoration: 'none',
        boxShadow: '0 6px 20px -8px rgba(20,18,14,0.4)'
      }}>
      <span className="kanji" style={{ fontSize: fs + 4,
                     color: 'var(--accent)' }}>下</span>
      Download for {os}
    </a>
  );
}

// New for B: a stats row that gives the eye something concrete
function StatsB() {
  return (
    <section style={{ borderTop: 'var(--hairline)',
                       borderBottom: 'var(--hairline)',
                       padding: '32px 48px',
                       background: 'var(--paper-2)' }}>
      <div style={{ maxWidth: 1200, margin: '0 auto',
                     display: 'grid',
                     gridTemplateColumns: 'repeat(4, 1fr)',
                     gap: 32 }}>
        {[
          { v: "0", k: "external requests" },
          { v: "<60MB", k: "memory footprint" },
          { v: "MCP", k: "open protocol" },
          { v: "Free", k: "forever" }
        ].map((s, i) => (
          <div key={i} style={{ textAlign: 'center' }}>
            <div className="display" style={{ fontSize: 28,
                           fontWeight: 400,
                           color: 'var(--ink)' }}>
              {s.v}
            </div>
            <div style={{ fontSize: 11,
                           letterSpacing: '0.16em',
                           textTransform: 'uppercase',
                           color: 'var(--ink-3)',
                           marginTop: 4 }}>{s.k}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

function WhatItIsB() {
  return (
    <section style={{ padding: '96px 48px' }}>
      <div style={{ maxWidth: 1200, margin: '0 auto',
                     display: 'grid',
                     gridTemplateColumns: '1fr 1.6fr',
                     gap: 64, alignItems: 'start' }}>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 16 }}>
            What it is
          </div>
          <h2 className="display" style={{ fontSize: 40, fontWeight: 300,
                         margin: 0, letterSpacing: '-0.025em',
                         lineHeight: 1.1 }}>
            One desktop app.<br/>
            One quiet promise.
          </h2>
        </div>
        <div style={{ fontSize: 17, lineHeight: 1.65,
                       color: 'var(--ink-2)',
                       fontFamily: 'var(--font-display)',
                       fontWeight: 300 }}>
          <p style={{ marginTop: 4 }}>
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
  );
}

function HowItWorksB() {
  const steps = [
    { kanji: "観", phase: "Watch",
      title: "It sits beside you",
      text: "Sensei sits beside your editor and AI tools, capturing the shape of each session — the prompts, the responses, the corrections.",
      sub: "Local only. Nothing leaves your machine." },
    { kanji: "察", phase: "Notice",
      title: "It begins to see",
      text: "After a few days, patterns surface. Recurring frictions. Idioms forming. Things you taught the assistant once and may want to teach it again.",
      sub: "You decide what's signal and what isn't." },
    { kanji: "覚", phase: "Adopt",
      title: "It remembers, with consent",
      text: "Worthy patterns become memories — small, named lessons sensei applies to future sessions on your behalf, with your blessing.",
      sub: "Adopt, refine, or dismiss. Always your call." }
  ];
  return (
    <section id="how" style={{
      borderTop: 'var(--hairline)',
      borderBottom: 'var(--hairline)',
      padding: '96px 48px',
      background: 'var(--paper-2)'
    }}>
      <div style={{ maxWidth: 1200, margin: '0 auto' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 16 }}>
          How it works
        </div>
        <h2 className="display" style={{ fontSize: 56, fontWeight: 300,
                       margin: '0 0 64px', letterSpacing: '-0.025em',
                       lineHeight: 1.05 }}>
          <span style={{ color: 'var(--accent)' }}>観 · 察 · 覚</span><br/>
          Watch, notice, adopt.
        </h2>
        <div style={{ display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)',
                       gap: 64 }}>
          {steps.map((s, i) => (
            <div key={i} style={{
              padding: '32px 24px',
              background: 'var(--paper)',
              border: 'var(--hairline)',
              borderRadius: 12,
              position: 'relative'
            }}>
              <div className="kanji" style={{ fontSize: 56,
                             color: 'var(--accent)', lineHeight: 1,
                             marginBottom: 16 }}>{s.kanji}</div>
              <div style={{ fontSize: 11, letterSpacing: '0.22em',
                             color: 'var(--ink-3)',
                             textTransform: 'uppercase',
                             marginBottom: 8 }}>{s.phase}</div>
              <h3 className="display" style={{ fontSize: 22,
                             fontWeight: 400,
                             margin: '0 0 16px',
                             letterSpacing: '-0.01em' }}>{s.title}</h3>
              <div style={{ fontSize: 13, color: 'var(--ink-2)',
                             lineHeight: 1.65, marginBottom: 16 }}>
                {s.text}
              </div>
              <div style={{ fontSize: 11, color: 'var(--ink-3)',
                             fontStyle: 'italic',
                             paddingTop: 12,
                             borderTop: 'var(--hairline)' }}>
                {s.sub}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function GalleryB() {
  const screens = [
    { caption: "Today",
      sub: "The morning briefing. One observation that's worth your attention. Everything else stays out of sight.",
      el: <MockToday width={920} height={580}/> },
    { caption: "Sessions",
      sub: "The week in review. Going well, not going well, things noticed — three lanes, no charts to decode.",
      el: <MockSessions width={920} height={580}/> },
    { caption: "Insights",
      sub: "What sensei has noticed. Patterns with confidence and provenance. You decide which become memories.",
      el: <MockInsights width={920} height={580}/> },
    { caption: "Memories",
      sub: "Adopted teachings. Each one named, dated, and traceable to the sessions it came from. No black box.",
      el: <MockMemory width={920} height={580}/> },
    { caption: "Instruments",
      sub: "Your tools, observed. Try them in isolation, replay what the assistant did, watch toolset health over time.",
      el: <MockInstruments width={920} height={580}/> }
  ];
  return (
    <section id="gallery" style={{ padding: '96px 48px 64px' }}>
      <div style={{ maxWidth: 1200, margin: '0 auto' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 16 }}>
          The screens
        </div>
        <h2 className="display" style={{ fontSize: 56, fontWeight: 300,
                       margin: '0 0 16px', letterSpacing: '-0.025em',
                       lineHeight: 1.05 }}>
          Five surfaces,<br/>
          one rhythm.
        </h2>
        <p style={{ fontSize: 17, color: 'var(--ink-2)',
                     maxWidth: 600, lineHeight: 1.6,
                     fontFamily: 'var(--font-display)', fontWeight: 300,
                     margin: '0 0 64px' }}>
          Every screen answers one question and stays quiet otherwise.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 96 }}>
          {screens.map((s, i) => (
            <div key={i} style={{
              display: 'grid',
              gridTemplateColumns: i % 2 === 0 ? '1fr 320px' : '320px 1fr',
              gap: 64, alignItems: 'center'
            }}>
              <div style={{ order: i % 2 === 0 ? 0 : 1 }}>{s.el}</div>
              <div style={{ order: i % 2 === 0 ? 1 : 0 }}>
                <div className="mono" style={{ fontSize: 11,
                               color: 'var(--accent)',
                               marginBottom: 8 }}>0{i + 1}</div>
                <div className="display" style={{ fontSize: 28,
                               fontWeight: 400,
                               marginBottom: 12,
                               letterSpacing: '-0.015em' }}>{s.caption}</div>
                <div style={{ fontSize: 13, color: 'var(--ink-2)',
                               lineHeight: 1.65 }}>{s.sub}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function PhilosophyB() {
  return (
    <section id="philosophy" style={{
      borderTop: 'var(--hairline)',
      borderBottom: 'var(--hairline)',
      padding: '96px 48px',
      background: 'var(--paper-2)',
      position: 'relative',
      overflow: 'hidden'
    }}>
      <div style={{ position: 'absolute', left: '50%', top: '50%',
                     transform: 'translate(-50%, -50%)',
                     fontSize: 56, lineHeight: 1,
                     color: 'var(--accent-soft)',
                     pointerEvents: 'none' }}
           className="kanji">
        静
      </div>
      <div style={{ maxWidth: 760, margin: '0 auto',
                     textAlign: 'center', position: 'relative' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 24 }}>
          Sei · stillness
        </div>
        <h2 className="display" style={{ fontSize: 56, fontWeight: 300,
                       margin: '0 0 32px', letterSpacing: '-0.025em',
                       lineHeight: 1.18 }}>
          The master observes for a long time before teaching.
        </h2>
        <p style={{ fontSize: 17, color: 'var(--ink-2)',
                     fontFamily: 'var(--font-display)', fontWeight: 300,
                     lineHeight: 1.7, margin: '0 0 24px' }}>
          AI tools are getting louder. More suggestions, more autocompletes,
          more interrupting. Sensei moves the other way. It speaks rarely,
          and only when it has something specific to say. Most days it is
          completely silent — and that is the feature.
        </p>
        <p style={{ fontSize: 15, color: 'var(--ink-2)',
                     lineHeight: 1.75, margin: 0 }}>
          The kanji throughout the app are not decoration. Each one names
          a phase of practice — observation, recognition, adoption,
          refinement. They are what we ask of the user, and what we ask
          of ourselves as the people who built this.
        </p>
      </div>
    </section>
  );
}

function PrivacyB() {
  return (
    <section id="privacy" style={{
      padding: '96px 48px',
      background: 'var(--paper)'
    }}>
      <div style={{ maxWidth: 1200, margin: '0 auto',
                     display: 'grid',
                     gridTemplateColumns: '1fr 1.5fr',
                     gap: 64, alignItems: 'start' }}>
        <div>
          <span className="kanji" style={{ fontSize: 56,
                         color: 'var(--accent)' }}>蔵</span>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--ink-3)',
                         textTransform: 'uppercase',
                         marginTop: 16, marginBottom: 16 }}>
            Privacy & local-first
          </div>
          <h2 className="display" style={{ fontSize: 40, fontWeight: 300,
                         margin: 0, letterSpacing: '-0.025em',
                         lineHeight: 1.15 }}>
            Your sessions stay on your machine.
          </h2>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 32 }}>
          {[
            { k: "蔵", title: "Local storage only",
              text: "Transcripts, patterns, memories — all stored in a SQLite file under your home directory. Sensei never makes outbound network requests." },
            { k: "鍵", title: "No telemetry",
              text: "We don't track usage. Updates are checked manually from Help → Check for Updates." },
            { k: "破", title: "Easy to delete",
              text: "One folder. Delete it and sensei forgets everything. Export to JSON anytime." }
          ].map((it, i) => (
            <div key={i} style={{ display: 'grid',
                       gridTemplateColumns: 'auto 1fr', gap: 24,
                       paddingBottom: 32,
                       borderBottom: i < 2 ? 'var(--hairline)' : 'none' }}>
              <span className="kanji" style={{ fontSize: 28,
                             color: 'var(--ink-2)' }}>{it.k}</span>
              <div>
                <div className="display" style={{ fontSize: 22,
                               marginBottom: 8,
                               letterSpacing: '-0.01em' }}>{it.title}</div>
                <div style={{ fontSize: 15, color: 'var(--ink-2)',
                               lineHeight: 1.65 }}>{it.text}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function PricingB() {
  return (
    <section style={{ borderTop: 'var(--hairline)',
                       borderBottom: 'var(--hairline)',
                       padding: '96px 48px',
                       background: 'var(--paper-2)',
                       textAlign: 'center' }}>
      <div style={{ maxWidth: 760, margin: '0 auto' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 16 }}>
          Pricing
        </div>
        <h2 className="display" style={{ fontSize: 56, fontWeight: 300,
                       margin: '0 0 24px', letterSpacing: '-0.025em',
                       lineHeight: 1.05 }}>
          Free.<br/>
          Pay what feels right.
        </h2>
        <p style={{ fontSize: 17, color: 'var(--ink-2)',
                     fontFamily: 'var(--font-display)', fontWeight: 300,
                     lineHeight: 1.65, margin: 0 }}>
          Sensei is free to download and use forever. If it earns a place
          in your daily practice, you can support development below — but
          there's no nag, no trial, no upgrade prompt. Ever.
        </p>
        <div style={{ marginTop: 48 }}>
          <DownloadCTAB size="lg"/>
        </div>
      </div>
    </section>
  );
}

function FaqB() {
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
    <section id="faq" style={{ padding: '96px 48px' }}>
      <div style={{ maxWidth: 960, margin: '0 auto' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 16 }}>
          Frequently asked
        </div>
        <h2 className="display" style={{ fontSize: 40, fontWeight: 300,
                       margin: '0 0 48px', letterSpacing: '-0.025em',
                       lineHeight: 1.1 }}>
          Common questions,<br/>
          plain answers.
        </h2>
        <div>
          {qs.map((it, i) => (
            <details key={i} style={{
              borderTop: 'var(--hairline)',
              padding: '24px 0',
              ...(i === qs.length - 1 ? { borderBottom: 'var(--hairline)' } : {})
            }}>
              <summary style={{
                cursor: 'pointer',
                listStyle: 'none',
                display: 'flex', justifyContent: 'space-between',
                fontSize: 17, color: 'var(--ink)',
                fontFamily: 'var(--font-display)',
                fontWeight: 400
              }}>
                <span>{it.q}</span>
                <span className="kanji" style={{ color: 'var(--ink-3)' }}>+</span>
              </summary>
              <div style={{ fontSize: 13, color: 'var(--ink-2)',
                             lineHeight: 1.7, marginTop: 16, maxWidth: 720 }}>
                {it.a}
              </div>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}

function SupportB() {
  return (
    <section style={{
      borderTop: 'var(--hairline)',
      background: 'var(--paper-2)',
      padding: '96px 48px',
      textAlign: 'center'
    }}>
      <div style={{ maxWidth: 720, margin: '0 auto' }}>
        <span className="kanji" style={{ fontSize: 56,
                       color: 'var(--accent)' }}>志</span>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase',
                       marginTop: 12, marginBottom: 12 }}>
          Support development · shi
        </div>
        <h2 className="display" style={{ fontSize: 28, fontWeight: 300,
                       margin: '0 0 24px', letterSpacing: '-0.02em',
                       lineHeight: 1.25 }}>
          If sensei has earned a place in your practice, you can help keep it growing.
        </h2>
        <p style={{ fontSize: 13, color: 'var(--ink-2)',
                     lineHeight: 1.7, margin: '0 0 32px' }}>
          Sensei is built by a small team. Every coffee buys an hour of focused work.
        </p>
        <a href="#sponsor" style={{
          display: 'inline-flex', alignItems: 'center', gap: 8,
          padding: '12px 24px',
          background: 'var(--accent)',
          color: 'var(--paper)',
          borderRadius: 8,
          fontSize: 13, fontWeight: 500,
          textDecoration: 'none'
        }}>
          ♥ Buy me a coffee
        </a>
      </div>
    </section>
  );
}

function FooterB() {
  return (
    <footer style={{
      padding: '48px 48px',
      fontSize: 13, color: 'var(--ink-3)',
      borderTop: 'var(--hairline)'
    }}>
      <div style={{ maxWidth: 1200, margin: '0 auto',
                     display: 'flex', alignItems: 'flex-start',
                     justifyContent: 'space-between', gap: 64 }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8,
                         marginBottom: 12 }}>
            <span className="kanji" style={{ fontSize: 17,
                           color: 'var(--accent)', letterSpacing: '-0.04em' }}>先生</span>
            <span className="display" style={{ fontSize: 15,
                           color: 'var(--ink-2)' }}>Sensei</span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--ink-3)',
                         maxWidth: 280, lineHeight: 1.6 }}>
            A patient observer for AI-assisted work. Built quietly,
            shipped slowly.
          </div>
          <div className="mono" style={{ fontSize: 11,
                         color: 'var(--ink-4)', marginTop: 12 }}>
            v0.4.2
          </div>
        </div>
        <div style={{ display: 'flex', gap: 48 }}>
          <FooterCol title="Product"
            links={["Download", "Privacy", "FAQ", "Changelog"]}/>
          <FooterCol title="Source"
            links={["GitHub", "MCP", "Roadmap", "Issues"]}/>
          <FooterCol title="Connect"
            links={["Twitter", "Mastodon", "Email", "RSS"]}/>
        </div>
      </div>
    </footer>
  );
}

function FooterCol({ title, links }) {
  return (
    <div>
      <div style={{ fontSize: 11, letterSpacing: '0.22em',
                     color: 'var(--ink-4)',
                     textTransform: 'uppercase',
                     marginBottom: 12 }}>{title}</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {links.map((l, i) => (
          <a key={i} href={`#${l.toLowerCase()}`}
             style={{ fontSize: 13,
                       color: 'var(--ink-2)' }}>{l}</a>
        ))}
      </div>
    </div>
  );
}

Object.assign(window, { VariantB });
