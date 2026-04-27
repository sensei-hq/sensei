// VARIANT C — "Marketing-forward"
// ─────────────────────────────────────────────────────────────────
// Brief: this variant lets the website become its own thing. The
// app's restraint stays *inside* the screenshots; everything around
// them is more saturated, more contemporary, more web. Gradient
// backdrops, big colored kanji as art objects, hero numbers,
// testimonial-style quotes.
//
// Reasoning:
//   · A "real" marketing site has to fight for attention. Visitors
//     arrive cold, scroll fast, and decide in seconds.
//   · This variant uses visual punch to land the value prop quickly,
//     then cools down inside the gallery to let the actual app
//     screenshots do their quiet work — by contrast they read as
//     "calm island" amid a more energetic page.
//   · Risk: this can feel disconnected from the product. Mitigated
//     by reusing the same kanji vocabulary, the same shu/jade/amber,
//     and the same display font. Only the layout language is louder.

const { useState: cS, useEffect: cE } = React;

function VariantC() {
  return (
    <div className="sensei variant-c" style={{
      background: 'var(--paper)', color: 'var(--sumi)',
      minHeight: '100%', fontFamily: 'var(--font-ui)'
    }}>
      <NavC/>
      <HeroC/>
      <LogoStripC/>
      <WhatItIsC/>
      <HowItWorksC/>
      <GalleryC/>
      <PhilosophyC/>
      <PrivacyC/>
      <PricingC/>
      <FaqC/>
      <SupportC/>
      <FooterC/>
    </div>
  );
}

function NavC() {
  return (
    <nav style={{
      position: 'sticky', top: 0, zIndex: 10,
      backdropFilter: 'blur(20px)',
      background: 'oklch(0.975 0.008 85 / 0.85)',
      borderBottom: 'var(--hairline)'
    }}>
      <div style={{ maxWidth: 1280, margin: '0 auto',
                     padding: '18px 48px',
                     display: 'flex', alignItems: 'center',
                     justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
          <span className="kanji" style={{ fontSize: 22,
                         color: 'var(--shu)', letterSpacing: '-0.04em' }}>先生</span>
          <span className="display" style={{ fontSize: 18,
                         letterSpacing: '-0.01em',
                         color: 'var(--sumi)' }}>Sensei</span>
        </div>
        <div style={{ display: 'flex', gap: 28, fontSize: 13,
                       alignItems: 'center' }}>
          {[
            ['#how', 'How'],
            ['#gallery', 'Screens'],
            ['#philosophy', 'Philosophy'],
            ['#privacy', 'Privacy'],
            ['#faq', 'FAQ']
          ].map(([href, label]) => (
            <a key={href} href={href}
               style={{ color: 'var(--sumi-2)',
                         textDecoration: 'none',
                         transition: 'color .15s' }}
               onMouseEnter={(e) => e.currentTarget.style.color = 'var(--sumi)'}
               onMouseLeave={(e) => e.currentTarget.style.color = 'var(--sumi-2)'}>
              {label}
            </a>
          ))}
        </div>
      </div>
    </nav>
  );
}

function HeroC() {
  return (
    <section style={{
      position: 'relative',
      padding: '80px 48px 0',
      overflow: 'hidden',
      background: `radial-gradient(ellipse at 70% 20%, oklch(0.58 0.15 35 / 0.10) 0%, transparent 55%),
                    radial-gradient(ellipse at 20% 80%, oklch(0.62 0.08 160 / 0.08) 0%, transparent 50%),
                    var(--paper)`
    }}>
      <div style={{ maxWidth: 1280, margin: '0 auto',
                     display: 'grid',
                     gridTemplateColumns: '1.1fr 1fr',
                     gap: 64, alignItems: 'center',
                     paddingBottom: 100 }}>
        <div style={{ position: 'relative' }}>
          <div style={{
            display: 'inline-flex', alignItems: 'center', gap: 8,
            padding: '6px 12px',
            background: 'var(--paper-2)',
            border: 'var(--hairline)',
            borderRadius: 999,
            fontSize: 11, color: 'var(--sumi-2)',
            marginBottom: 28
          }}>
            <span className="ink-dot" style={{
              background: 'var(--jade)', width: 6, height: 6 }}/>
            v0.4.2 · now in public preview
          </div>
          <h1 className="display" style={{
            fontSize: 76, fontWeight: 300, lineHeight: 1.02,
            letterSpacing: '-0.03em',
            margin: 0
          }}>
            A quiet companion for{' '}
            <span style={{
              background: 'linear-gradient(95deg, var(--shu) 0%, oklch(0.66 0.15 60) 100%)',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
              backgroundClip: 'text'
            }}>AI-assisted work</span>.
          </h1>
          <p style={{ fontSize: 18, color: 'var(--sumi-2)',
                       lineHeight: 1.6, marginTop: 28, maxWidth: 520 }}>
            Sensei observes your sessions with AI assistants —
            then surfaces the patterns you're too close to see. Not a
            chatbot. Not a copilot. <em>A patient observer.</em>
          </p>
          <div style={{ display: 'flex', gap: 14, alignItems: 'center',
                         marginTop: 36 }}>
            <DownloadCTAC size="lg"/>
            <a href="#how" style={{
              display: 'inline-flex', alignItems: 'center', gap: 8,
              padding: '14px 22px',
              border: '1px solid var(--paper-edge)',
              borderRadius: 8,
              fontSize: 14,
              color: 'var(--sumi)',
              background: 'var(--paper)',
              textDecoration: 'none'
            }}>
              How it works ↓
            </a>
          </div>
          <div style={{ display: 'flex', gap: 18, marginTop: 22,
                         fontSize: 11, color: 'var(--sumi-3)' }}>
            <span>✓ Free</span>
            <span>✓ Local-first</span>
            <span>✓ No account</span>
          </div>
        </div>

        {/* Floating screenshot stack */}
        <div style={{ position: 'relative', height: 540 }}>
          <div style={{
            position: 'absolute', right: -40, top: 0,
            transform: 'rotate(2deg)',
            opacity: 0.95
          }}>
            <MockSessions width={620} height={400}/>
          </div>
          <div style={{
            position: 'absolute', left: -40, bottom: 0,
            transform: 'rotate(-2deg)'
          }}>
            <MockToday width={620} height={400}/>
          </div>
          {/* Big floating kanji as art object */}
          <div style={{
            position: 'absolute', right: -120, top: -60,
            fontSize: 240, lineHeight: 1,
            color: 'var(--shu)', opacity: 0.08,
            pointerEvents: 'none',
            letterSpacing: '-0.04em'
          }} className="kanji">先生</div>
        </div>
      </div>
    </section>
  );
}

function DownloadCTAC({ size = "lg" }) {
  const [os, setOs] = cS("macOS");
  cE(() => {
    const ua = navigator.userAgent || "";
    if (/Win/.test(ua))         setOs("Windows");
    else if (/Linux/.test(ua))  setOs("Linux");
    else if (/Mac/.test(ua))    setOs("macOS");
  }, []);
  const px = size === "lg" ? '14px 26px' : '8px 16px';
  const fs = size === "lg" ? 14 : 12;
  return (
    <a href={`#download-${os.toLowerCase()}`}
       style={{
        display: 'inline-flex', alignItems: 'center', gap: 10,
        padding: px,
        background: 'linear-gradient(180deg, var(--sumi) 0%, oklch(0.18 0.012 50) 100%)',
        color: 'var(--paper)',
        borderRadius: 8,
        fontSize: fs,
        fontWeight: 500,
        textDecoration: 'none',
        boxShadow: '0 8px 24px -8px rgba(20,18,14,0.5), inset 0 1px 0 rgba(255,255,255,0.08)'
      }}>
      <span className="kanji" style={{ fontSize: fs + 3,
                     color: 'var(--shu)' }}>下</span>
      Download for {os}
    </a>
  );
}

// "As featured on" / pretend-press strip — fixture-only
function LogoStripC() {
  const items = ["MCP", "AI assistants", "MCP-compatible tools", "Tauri", "SQLite"];
  return (
    <section style={{ borderTop: 'var(--hairline)',
                       borderBottom: 'var(--hairline)',
                       padding: '28px 48px',
                       background: 'var(--paper-2)' }}>
      <div style={{ maxWidth: 1280, margin: '0 auto',
                     display: 'flex',
                     alignItems: 'center',
                     justifyContent: 'space-between',
                     fontSize: 11, color: 'var(--sumi-3)',
                     letterSpacing: '0.12em',
                     textTransform: 'uppercase' }}>
        <span style={{ flexShrink: 0 }}>Works alongside</span>
        <div style={{ display: 'flex', gap: 36,
                       fontSize: 14, fontFamily: 'var(--font-display)',
                       letterSpacing: '-0.01em',
                       textTransform: 'none',
                       color: 'var(--sumi-2)' }}>
          {items.map((l, i) => <span key={i}>{l}</span>)}
        </div>
      </div>
    </section>
  );
}

function WhatItIsC() {
  return (
    <section style={{ padding: '120px 48px' }}>
      <div style={{ maxWidth: 1280, margin: '0 auto' }}>
        <div style={{ textAlign: 'center', marginBottom: 80 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--shu)',
                         textTransform: 'uppercase', marginBottom: 16,
                         fontWeight: 500 }}>
            What it is
          </div>
          <h2 className="display" style={{ fontSize: 56, fontWeight: 300,
                         margin: '0 auto', letterSpacing: '-0.025em',
                         lineHeight: 1.1, maxWidth: 780 }}>
            One desktop app. One quiet promise.
          </h2>
        </div>
        <div style={{ display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)',
                       gap: 24 }}>
          {[
            { k: "観", title: "Observes",
              text: "Sensei watches your sessions with AI tools — locally, in real time. Nothing leaves your machine.",
              tone: 'shu' },
            { k: "察", title: "Recognizes",
              text: "Recurring patterns surface as soft signals. Friction points, idioms, lessons forming.",
              tone: 'jade' },
            { k: "覚", title: "Remembers",
              text: "What you adopt becomes a memory — small, named, and applied to future sessions on your terms.",
              tone: 'amber' }
          ].map((f, i) => (
            <div key={i} style={{
              padding: '32px 28px',
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 16,
              position: 'relative',
              overflow: 'hidden'
            }}>
              <div style={{
                position: 'absolute', right: -12, top: -16,
                fontSize: 120, lineHeight: 1,
                color: `var(--${f.tone}-soft)`,
                pointerEvents: 'none'
              }} className="kanji">{f.k}</div>
              <div className="kanji" style={{ fontSize: 32,
                             color: `var(--${f.tone})`,
                             marginBottom: 18, position: 'relative' }}>
                {f.k}
              </div>
              <h3 className="display" style={{ fontSize: 24,
                             fontWeight: 400,
                             margin: '0 0 12px',
                             letterSpacing: '-0.01em' }}>
                {f.title}
              </h3>
              <div style={{ fontSize: 14, color: 'var(--sumi-2)',
                             lineHeight: 1.65 }}>
                {f.text}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function HowItWorksC() {
  const steps = [
    { kanji: "観", phase: "01 · Watch",
      title: "It sits beside you",
      text: "Sensei sits beside your editor and AI tools, capturing the shape of each session — the prompts, the responses, the corrections.",
      sub: "Local only. Nothing leaves your machine.",
      tone: 'shu' },
    { kanji: "察", phase: "02 · Notice",
      title: "It begins to see",
      text: "After a few days, patterns surface. Recurring frictions. Idioms forming. Things you taught the assistant once and may want to teach it again.",
      sub: "You decide what's signal and what isn't.",
      tone: 'jade' },
    { kanji: "覚", phase: "03 · Adopt",
      title: "It remembers, with consent",
      text: "Worthy patterns become memories — small, named lessons sensei applies to future sessions on your behalf, with your blessing.",
      sub: "Adopt, refine, or dismiss. Always your call.",
      tone: 'amber' }
  ];
  return (
    <section id="how" style={{
      borderTop: 'var(--hairline)',
      borderBottom: 'var(--hairline)',
      padding: '120px 48px',
      background: `linear-gradient(180deg, var(--paper) 0%, var(--paper-2) 100%)`
    }}>
      <div style={{ maxWidth: 1280, margin: '0 auto' }}>
        <div style={{ textAlign: 'center', marginBottom: 80 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--shu)',
                         textTransform: 'uppercase', marginBottom: 16,
                         fontWeight: 500 }}>
            How it works
          </div>
          <h2 className="display" style={{ fontSize: 56, fontWeight: 300,
                         margin: 0, letterSpacing: '-0.025em',
                         lineHeight: 1.1 }}>
            Watch → Notice → Adopt
          </h2>
        </div>
        <div style={{ display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)',
                       gap: 24, position: 'relative' }}>
          {/* connecting line */}
          <div style={{ position: 'absolute', top: 60,
                         left: '16%', right: '16%', height: 1,
                         background: 'var(--paper-edge)',
                         pointerEvents: 'none' }}/>
          {steps.map((s, i) => (
            <div key={i} style={{
              padding: '40px 32px',
              background: 'var(--paper)',
              border: 'var(--hairline)',
              borderRadius: 16,
              position: 'relative'
            }}>
              <div style={{
                width: 64, height: 64,
                borderRadius: '50%',
                background: `var(--${s.tone}-soft)`,
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                marginBottom: 22
              }}>
                <span className="kanji" style={{ fontSize: 28,
                               color: `var(--${s.tone})` }}>{s.kanji}</span>
              </div>
              <div className="mono" style={{ fontSize: 11,
                             color: `var(--${s.tone})`,
                             marginBottom: 8 }}>{s.phase}</div>
              <h3 className="display" style={{ fontSize: 24,
                             fontWeight: 400,
                             margin: '0 0 14px',
                             letterSpacing: '-0.01em' }}>{s.title}</h3>
              <div style={{ fontSize: 14, color: 'var(--sumi-2)',
                             lineHeight: 1.65, marginBottom: 18 }}>
                {s.text}
              </div>
              <div style={{ fontSize: 11.5, color: 'var(--sumi-3)',
                             paddingTop: 14,
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

function GalleryC() {
  const screens = [
    { caption: "Today",
      sub: "The morning briefing. One observation that's worth your attention. Everything else stays out of sight.",
      tone: 'shu',
      el: <MockToday width={920} height={580}/> },
    { caption: "Sessions",
      sub: "The week in review. Going well, not going well, things noticed — three lanes, no charts to decode.",
      tone: 'jade',
      el: <MockSessions width={920} height={580}/> },
    { caption: "Insights",
      sub: "What sensei has noticed. Patterns with confidence and provenance. You decide which become memories.",
      tone: 'amber',
      el: <MockInsights width={920} height={580}/> },
    { caption: "Memories",
      sub: "Adopted teachings. Each one named, dated, and traceable to the sessions it came from. No black box.",
      tone: 'shu',
      el: <MockMemory width={920} height={580}/> },
    { caption: "Instruments",
      sub: "Your tools, observed. Try them in isolation, replay what the assistant did, watch toolset health over time.",
      tone: 'jade',
      el: <MockInstruments width={920} height={580}/> }
  ];
  return (
    <section id="gallery" style={{ padding: '120px 48px 60px' }}>
      <div style={{ maxWidth: 1280, margin: '0 auto' }}>
        <div style={{ textAlign: 'center', marginBottom: 80 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--shu)',
                         textTransform: 'uppercase', marginBottom: 16,
                         fontWeight: 500 }}>
            The screens
          </div>
          <h2 className="display" style={{ fontSize: 56, fontWeight: 300,
                         margin: 0, letterSpacing: '-0.025em',
                         lineHeight: 1.1 }}>
            Five surfaces, one rhythm.
          </h2>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 100 }}>
          {screens.map((s, i) => (
            <div key={i} style={{
              display: 'grid',
              gridTemplateColumns: i % 2 === 0 ? '1fr 360px' : '360px 1fr',
              gap: 64, alignItems: 'center'
            }}>
              <div style={{
                order: i % 2 === 0 ? 0 : 1,
                position: 'relative'
              }}>
                <div style={{
                  position: 'absolute',
                  inset: -32,
                  background: `radial-gradient(ellipse at center, var(--${s.tone}-soft) 0%, transparent 65%)`,
                  pointerEvents: 'none',
                  zIndex: 0
                }}/>
                <div style={{ position: 'relative', zIndex: 1 }}>{s.el}</div>
              </div>
              <div style={{ order: i % 2 === 0 ? 1 : 0 }}>
                <div className="mono" style={{ fontSize: 12,
                               color: `var(--${s.tone})`,
                               marginBottom: 10,
                               letterSpacing: '0.1em' }}>
                  0{i + 1} / 05
                </div>
                <div className="display" style={{ fontSize: 36,
                               fontWeight: 400,
                               marginBottom: 16,
                               letterSpacing: '-0.02em' }}>{s.caption}</div>
                <div style={{ fontSize: 15, color: 'var(--sumi-2)',
                               lineHeight: 1.65 }}>{s.sub}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function PhilosophyC() {
  return (
    <section id="philosophy" style={{
      padding: '160px 48px',
      background: `linear-gradient(180deg, oklch(0.22 0.012 50) 0%, oklch(0.18 0.010 50) 100%)`,
      color: 'var(--paper)',
      position: 'relative',
      overflow: 'hidden'
    }}>
      <div style={{
        position: 'absolute', left: '50%', top: '50%',
        transform: 'translate(-50%, -50%)',
        fontSize: 540, lineHeight: 1,
        color: 'oklch(0.58 0.15 35 / 0.08)',
        pointerEvents: 'none'
      }} className="kanji">静</div>
      <div style={{ maxWidth: 820, margin: '0 auto',
                     textAlign: 'center', position: 'relative' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--shu)',
                       textTransform: 'uppercase', marginBottom: 24,
                       fontWeight: 500 }}>
          Sei · stillness
        </div>
        <h2 className="display" style={{ fontSize: 52, fontWeight: 300,
                       margin: '0 0 36px', letterSpacing: '-0.025em',
                       lineHeight: 1.18, color: 'var(--paper)' }}>
          The master observes for a long time before teaching.
        </h2>
        <p style={{ fontSize: 18, color: 'oklch(0.78 0.008 85)',
                     lineHeight: 1.7, margin: '0 0 22px' }}>
          AI tools are getting louder. More suggestions, more autocompletes,
          more interrupting. Sensei moves the other way. It speaks rarely,
          and only when it has something specific to say. Most days it is
          completely silent — and that is the feature.
        </p>
        <p style={{ fontSize: 15, color: 'oklch(0.62 0.010 85)',
                     lineHeight: 1.75, margin: 0 }}>
          The kanji throughout the app are not decoration. Each one names
          a phase of practice — observation, recognition, adoption,
          refinement.
        </p>
      </div>
    </section>
  );
}

function PrivacyC() {
  return (
    <section id="privacy" style={{
      padding: '120px 48px',
      background: 'var(--paper)'
    }}>
      <div style={{ maxWidth: 1280, margin: '0 auto' }}>
        <div style={{ textAlign: 'center', marginBottom: 64 }}>
          <span className="kanji" style={{ fontSize: 56,
                         color: 'var(--shu)' }}>蔵</span>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--shu)',
                         textTransform: 'uppercase',
                         marginTop: 14, marginBottom: 16,
                         fontWeight: 500 }}>
            Privacy & local-first
          </div>
          <h2 className="display" style={{ fontSize: 48, fontWeight: 300,
                         margin: 0, letterSpacing: '-0.025em',
                         lineHeight: 1.1, maxWidth: 720,
                         marginLeft: 'auto', marginRight: 'auto' }}>
            Your sessions stay on your machine.
          </h2>
        </div>
        <div style={{ display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)',
                       gap: 24 }}>
          {[
            { k: "蔵", title: "Local storage only",
              text: "Transcripts, patterns, memories — all in a SQLite file under your home directory. No outbound network requests, ever." },
            { k: "鍵", title: "No telemetry",
              text: "We don't track usage. Updates are checked manually from Help → Check for Updates." },
            { k: "破", title: "Easy to delete",
              text: "One folder. Delete it and sensei forgets everything. Export to JSON anytime." }
          ].map((it, i) => (
            <div key={i} style={{
              padding: '28px 24px',
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 12
            }}>
              <span className="kanji" style={{ fontSize: 28,
                             color: 'var(--sumi-2)' }}>{it.k}</span>
              <div className="display" style={{ fontSize: 18,
                             marginTop: 14, marginBottom: 10,
                             letterSpacing: '-0.01em' }}>{it.title}</div>
              <div style={{ fontSize: 13.5, color: 'var(--sumi-2)',
                             lineHeight: 1.65 }}>{it.text}</div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function PricingC() {
  return (
    <section style={{
      padding: '120px 48px',
      background: `radial-gradient(ellipse at center, oklch(0.58 0.15 35 / 0.08) 0%, transparent 60%), var(--paper)`,
      borderTop: 'var(--hairline)',
      borderBottom: 'var(--hairline)',
      textAlign: 'center'
    }}>
      <div style={{ maxWidth: 760, margin: '0 auto' }}>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--shu)',
                       textTransform: 'uppercase', marginBottom: 18,
                       fontWeight: 500 }}>
          Pricing
        </div>
        <h2 className="display" style={{ fontSize: 64, fontWeight: 300,
                       margin: '0 0 24px', letterSpacing: '-0.03em',
                       lineHeight: 1 }}>
          Free.<br/>
          <span style={{
            background: 'linear-gradient(95deg, var(--shu) 0%, oklch(0.66 0.15 60) 100%)',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent'
          }}>
            Pay what feels right.
          </span>
        </h2>
        <p style={{ fontSize: 17, color: 'var(--sumi-2)',
                     lineHeight: 1.65, margin: 0 }}>
          Sensei is free to download and use forever. If it earns a place
          in your daily practice, you can support development below — but
          there's no nag, no trial, no upgrade prompt. Ever.
        </p>
        <div style={{ marginTop: 44 }}>
          <DownloadCTAC size="lg"/>
        </div>
      </div>
    </section>
  );
}

function FaqC() {
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
    <section id="faq" style={{ padding: '120px 48px' }}>
      <div style={{ maxWidth: 960, margin: '0 auto' }}>
        <div style={{ textAlign: 'center', marginBottom: 64 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--shu)',
                         textTransform: 'uppercase', marginBottom: 16,
                         fontWeight: 500 }}>
            Frequently asked
          </div>
          <h2 className="display" style={{ fontSize: 48, fontWeight: 300,
                         margin: 0, letterSpacing: '-0.025em',
                         lineHeight: 1.1 }}>
            Common questions, plain answers.
          </h2>
        </div>
        <div>
          {qs.map((it, i) => (
            <details key={i} style={{
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 10,
              marginBottom: 12,
              padding: '20px 24px'
            }}>
              <summary style={{
                cursor: 'pointer',
                listStyle: 'none',
                display: 'flex', justifyContent: 'space-between',
                fontSize: 16, color: 'var(--sumi)',
                fontFamily: 'var(--font-display)', fontWeight: 400
              }}>
                <span>{it.q}</span>
                <span className="kanji" style={{ color: 'var(--shu)' }}>+</span>
              </summary>
              <div style={{ fontSize: 14, color: 'var(--sumi-2)',
                             lineHeight: 1.7, marginTop: 14 }}>
                {it.a}
              </div>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}

function SupportC() {
  return (
    <section style={{
      padding: '100px 48px',
      background: `linear-gradient(180deg, var(--paper) 0%, var(--paper-2) 100%)`,
      borderTop: 'var(--hairline)',
      textAlign: 'center'
    }}>
      <div style={{ maxWidth: 720, margin: '0 auto',
                     padding: '48px 32px',
                     background: 'var(--paper)',
                     border: 'var(--hairline)',
                     borderRadius: 16,
                     boxShadow: '0 20px 50px -20px rgba(20,18,14,0.15)' }}>
        <span className="kanji" style={{ fontSize: 48,
                       color: 'var(--shu)' }}>志</span>
        <div style={{ fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--sumi-3)',
                       textTransform: 'uppercase',
                       marginTop: 12, marginBottom: 14 }}>
          Support development
        </div>
        <h2 className="display" style={{ fontSize: 28, fontWeight: 400,
                       margin: '0 0 18px', letterSpacing: '-0.015em',
                       lineHeight: 1.3 }}>
          If sensei has earned a place in your practice, help keep it growing.
        </h2>
        <p style={{ fontSize: 14, color: 'var(--sumi-2)',
                     lineHeight: 1.7, margin: '0 0 28px' }}>
          Built by a small team. Every coffee buys an hour of focused work.
        </p>
        <a href="#sponsor" style={{
          display: 'inline-flex', alignItems: 'center', gap: 10,
          padding: '14px 28px',
          background: 'linear-gradient(180deg, var(--shu) 0%, oklch(0.52 0.16 30) 100%)',
          color: 'var(--paper)',
          borderRadius: 8,
          fontSize: 14, fontWeight: 500,
          textDecoration: 'none',
          boxShadow: '0 8px 20px -8px var(--shu)'
        }}>
          ♥ Buy me a coffee
        </a>
      </div>
    </section>
  );
}

function FooterC() {
  return (
    <footer style={{
      padding: '64px 48px 48px',
      background: 'oklch(0.20 0.010 50)',
      color: 'oklch(0.62 0.010 85)',
      fontSize: 12
    }}>
      <div style={{ maxWidth: 1280, margin: '0 auto',
                     display: 'flex', alignItems: 'flex-start',
                     justifyContent: 'space-between', gap: 64,
                     flexWrap: 'wrap' }}>
        <div style={{ maxWidth: 320 }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 10,
                         marginBottom: 14 }}>
            <span className="kanji" style={{ fontSize: 22,
                           color: 'var(--shu)', letterSpacing: '-0.04em' }}>先生</span>
            <span className="display" style={{ fontSize: 18,
                           color: 'var(--paper)' }}>Sensei</span>
          </div>
          <div style={{ lineHeight: 1.6 }}>
            A patient observer for AI-assisted work. Built quietly,
            shipped slowly.
          </div>
          <div className="mono" style={{ fontSize: 10,
                         marginTop: 16, opacity: 0.7 }}>
            v0.4.2
          </div>
        </div>
        <div style={{ display: 'flex', gap: 56, flexWrap: 'wrap' }}>
          <FooterColC title="Product"
            links={["Download", "Privacy", "FAQ", "Changelog"]}/>
          <FooterColC title="Source"
            links={["GitHub", "MCP", "Roadmap", "Issues"]}/>
          <FooterColC title="Connect"
            links={["Twitter", "Mastodon", "Email", "RSS"]}/>
        </div>
      </div>
      <div style={{ maxWidth: 1280, margin: '40px auto 0',
                     paddingTop: 24,
                     borderTop: '1px solid oklch(0.32 0.010 50)',
                     fontSize: 10.5, opacity: 0.6,
                     textAlign: 'center' }}>
        © 2025 Sensei. Made with restraint.
      </div>
    </footer>
  );
}

function FooterColC({ title, links }) {
  return (
    <div>
      <div style={{ fontSize: 9.5, letterSpacing: '0.22em',
                     color: 'var(--shu)',
                     textTransform: 'uppercase',
                     marginBottom: 14, fontWeight: 500 }}>{title}</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {links.map((l, i) => (
          <a key={i} href={`#${l.toLowerCase()}`}
             style={{ fontSize: 12,
                       color: 'oklch(0.78 0.008 85)' }}>{l}</a>
        ))}
      </div>
    </div>
  );
}

Object.assign(window, { VariantC });
