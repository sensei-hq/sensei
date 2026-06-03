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
      background: 'var(--paper)', color: 'var(--ink)',
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
      <div style={{
 maxWidth: 1280,
                     display: 'flex', alignItems: 'center',
                     justifyContent: 'space-between'
}} className="py-4 px-7 mx-auto" >
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
          <span className="kanji" style={{ fontSize: 22,
                         color: 'var(--accent)', letterSpacing: '-0.04em' }}>先生</span>
          <span className="display" style={{ fontSize: 17,
                         letterSpacing: '-0.01em',
                         color: 'var(--ink)' }}>Sensei</span>
        </div>
        <div style={{
 display: 'flex', fontSize: 13,
                       alignItems: 'center'
}} className="gap-5" >
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
      </div>
    </nav>
  );
}

function HeroC() {
  return (
    <section style={{
      position: 'relative',
      overflow: 'hidden',
      background: `radial-gradient(ellipse at 70% 20%, oklch(0.58 0.15 35 / 0.10) 0%, transparent 55%),
                    radial-gradient(ellipse at 20% 80%, oklch(0.62 0.08 160 / 0.08) 0%, transparent 50%),
                    var(--paper)`
}} className="pt-8 pb-0 px-7" >
      <div style={{
 maxWidth: 1280,
                     display: 'grid',
                     gridTemplateColumns: '1.1fr 1fr', alignItems: 'center'
}} className="gap-8 mx-auto pb-9" >
        <div style={{ position: 'relative' }}>
          <div style={{
            display: 'inline-flex', alignItems: 'center',
            background: 'var(--paper-2)',
            border: 'var(--hairline)',
            borderRadius: 999,
            fontSize: 11, color: 'var(--ink-2)'
}} className="gap-2 py-1 px-3 mb-5" >
            <span className="ink-dot" style={{
              background: 'var(--success)', width: 6, height: 6 }}/>
            v0.4.2 · now in public preview
          </div>
          <h1 className="display m-0" style={{
            fontSize: 56, fontWeight: 300, lineHeight: 1.02,
            letterSpacing: '-0.03em'
}}>
            A quiet companion for{' '}
            <span style={{
              background: 'linear-gradient(95deg, var(--accent) 0%, oklch(0.66 0.15 60) 100%)',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
              backgroundClip: 'text'
            }}>AI-assisted work</span>.
          </h1>
          <p style={{
 fontSize: 17, color: 'var(--ink-2)',
                       lineHeight: 1.6, maxWidth: 520
}} className="mt-5" >
            Sensei observes your sessions with AI assistants —
            then surfaces the patterns you're too close to see. Not a
            chatbot. Not a copilot. <em>A patient observer.</em>
          </p>
          <div style={{
 display: 'flex', alignItems: 'center'
}} className="gap-3 mt-6" >
            <DownloadCTAC size="lg"/>
            <a href="#how" style={{
              display: 'inline-flex', alignItems: 'center',
              border: '1px solid var(--edge)',
              borderRadius: 8,
              fontSize: 13,
              color: 'var(--ink)',
              background: 'var(--paper)',
              textDecoration: 'none'
}} className="gap-2 py-3 px-5" >
              How it works ↓
            </a>
          </div>
          <div style={{
 display: 'flex',
                         fontSize: 11, color: 'var(--ink-3)'
}} className="gap-4 mt-5" >
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
            fontSize: 56, lineHeight: 1,
            color: 'var(--accent)', opacity: 0.08,
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
        display: 'inline-flex', alignItems: 'center',
        padding: px,
        background: 'linear-gradient(180deg, var(--ink) 0%, oklch(0.18 0.012 50) 100%)',
        color: 'var(--paper)',
        borderRadius: 8,
        fontSize: fs,
        fontWeight: 500,
        textDecoration: 'none',
        boxShadow: '0 8px 24px -8px rgba(20,18,14,0.5), inset 0 1px 0 rgba(255,255,255,0.08)'
}} className="gap-2" >
      <span className="kanji" style={{ fontSize: fs + 3,
                     color: 'var(--accent)' }}>下</span>
      Download for {os}
    </a>
  );
}

// "As featured on" / pretend-press strip — fixture-only
function LogoStripC() {
  const items = ["MCP", "AI assistants", "MCP-compatible tools", "Tauri", "SQLite"];
  return (
    <section style={{
 borderTop: 'var(--hairline)',
                       borderBottom: 'var(--hairline)',
                       background: 'var(--paper-2)'
}} className="py-5 px-7" >
      <div style={{
 maxWidth: 1280,
                     display: 'flex',
                     alignItems: 'center',
                     justifyContent: 'space-between',
                     fontSize: 11, color: 'var(--ink-3)',
                     letterSpacing: '0.12em',
                     textTransform: 'uppercase'
}} className="mx-auto" >
        <span style={{ flexShrink: 0 }}>Works alongside</span>
        <div style={{
 display: 'flex',
                       fontSize: 13, fontFamily: 'var(--font-display)',
                       letterSpacing: '-0.01em',
                       textTransform: 'none',
                       color: 'var(--ink-2)'
}} className="gap-6" >
          {items.map((l, i) => <span key={i}>{l}</span>)}
        </div>
      </div>
    </section>
  );
}

function WhatItIsC() {
  return (
    <section className="py-9 px-7" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div style={{ textAlign: 'center' }} className="mb-8" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--accent)',
                         textTransform: 'uppercase',
                         fontWeight: 500
}} className="mb-4" >
            What it is
          </div>
          <h2 className="display mx-auto" style={{
 fontSize: 56, fontWeight: 300, letterSpacing: '-0.025em',
                         lineHeight: 1.1, maxWidth: 780
}}>
            One desktop app. One quiet promise.
          </h2>
        </div>
        <div style={{
 display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)'
}} className="gap-5" >
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
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 16,
              position: 'relative',
              overflow: 'hidden'
}} className="py-6 px-5" >
              <div style={{
                position: 'absolute', right: -12, top: -16,
                fontSize: 56, lineHeight: 1,
                color: `var(--${f.tone}-soft)`,
                pointerEvents: 'none'
              }} className="kanji">{f.k}</div>
              <div className="kanji mb-4" style={{
 fontSize: 28,
                             color: `var(--${f.tone})`, position: 'relative'
}}>
                {f.k}
              </div>
              <h3 className="display mt-0 mb-3" style={{
 fontSize: 22,
                             fontWeight: 400,
                             letterSpacing: '-0.01em'
}}>
                {f.title}
              </h3>
              <div style={{ fontSize: 13, color: 'var(--ink-2)',
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
      background: `linear-gradient(180deg, var(--paper) 0%, var(--paper-2) 100%)`
}} className="py-9 px-7" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div style={{ textAlign: 'center' }} className="mb-8" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--accent)',
                         textTransform: 'uppercase',
                         fontWeight: 500
}} className="mb-4" >
            How it works
          </div>
          <h2 className="display m-0" style={{
 fontSize: 56, fontWeight: 300, letterSpacing: '-0.025em',
                         lineHeight: 1.1
}}>
            Watch → Notice → Adopt
          </h2>
        </div>
        <div style={{
 display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)', position: 'relative'
}} className="gap-5" >
          {/* connecting line */}
          <div style={{ position: 'absolute', top: 60,
                         left: '16%', right: '16%', height: 1,
                         background: 'var(--edge)',
                         pointerEvents: 'none' }}/>
          {steps.map((s, i) => (
            <div key={i} style={{
              background: 'var(--paper)',
              border: 'var(--hairline)',
              borderRadius: 16,
              position: 'relative'
}} className="py-6 px-6" >
              <div style={{
                width: 64, height: 64,
                borderRadius: '50%',
                background: `var(--${s.tone}-soft)`,
                display: 'flex', alignItems: 'center', justifyContent: 'center'
}} className="mb-5" >
                <span className="kanji" style={{ fontSize: 28,
                               color: `var(--${s.tone})` }}>{s.kanji}</span>
              </div>
              <div className="mono mb-2" style={{
 fontSize: 11,
                             color: `var(--${s.tone})`
}}>{s.phase}</div>
              <h3 className="display mt-0 mb-3" style={{
 fontSize: 22,
                             fontWeight: 400,
                             letterSpacing: '-0.01em'
}}>{s.title}</h3>
              <div style={{
 fontSize: 13, color: 'var(--ink-2)',
                             lineHeight: 1.65
}} className="mb-4" >
                {s.text}
              </div>
              <div style={{
 fontSize: 11, color: 'var(--ink-3)',
                             borderTop: 'var(--hairline)'
}} className="pt-3" >
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
    <section id="gallery" className="pt-9 pb-8 px-7" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div style={{ textAlign: 'center' }} className="mb-8" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--accent)',
                         textTransform: 'uppercase',
                         fontWeight: 500
}} className="mb-4" >
            The screens
          </div>
          <h2 className="display m-0" style={{
 fontSize: 56, fontWeight: 300, letterSpacing: '-0.025em',
                         lineHeight: 1.1
}}>
            Five surfaces, one rhythm.
          </h2>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 96 }}>
          {screens.map((s, i) => (
            <div key={i} style={{
              display: 'grid',
              gridTemplateColumns: i % 2 === 0 ? '1fr 360px' : '360px 1fr', alignItems: 'center'
}} className="gap-8" >
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
                <div className="mono mb-2" style={{
 fontSize: 13,
                               color: `var(--${s.tone})`,
                               letterSpacing: '0.1em'
}}>
                  0{i + 1} / 05
                </div>
                <div className="display mb-4" style={{
 fontSize: 40,
                               fontWeight: 400,
                               letterSpacing: '-0.02em'
}}>{s.caption}</div>
                <div style={{ fontSize: 15, color: 'var(--ink-2)',
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
      background: `linear-gradient(180deg, oklch(0.22 0.012 50) 0%, oklch(0.18 0.010 50) 100%)`,
      color: 'var(--paper)',
      position: 'relative',
      overflow: 'hidden'
}} className="py-9 px-7" >
      <div style={{
        position: 'absolute', left: '50%', top: '50%',
        transform: 'translate(-50%, -50%)',
        fontSize: 56, lineHeight: 1,
        color: 'oklch(0.58 0.15 35 / 0.08)',
        pointerEvents: 'none'
      }} className="kanji">静</div>
      <div style={{
 maxWidth: 820,
                     textAlign: 'center', position: 'relative'
}} className="mx-auto" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--accent)',
                       textTransform: 'uppercase',
                       fontWeight: 500
}} className="mb-5" >
          Sei · stillness
        </div>
        <h2 className="display mt-0 mb-6" style={{
 fontSize: 56, fontWeight: 300, letterSpacing: '-0.025em',
                       lineHeight: 1.18, color: 'var(--paper)'
}}>
          The master observes for a long time before teaching.
        </h2>
        <p style={{
 fontSize: 17, color: 'oklch(0.78 0.008 85)',
                     lineHeight: 1.7
}} className="mt-0 mb-5" >
          AI tools are getting louder. More suggestions, more autocompletes,
          more interrupting. Sensei moves the other way. It speaks rarely,
          and only when it has something specific to say. Most days it is
          completely silent — and that is the feature.
        </p>
        <p style={{
 fontSize: 15, color: 'oklch(0.62 0.010 85)',
                     lineHeight: 1.75
}} className="m-0" >
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
      background: 'var(--paper)'
}} className="py-9 px-7" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div style={{ textAlign: 'center' }} className="mb-8" >
          <span className="kanji" style={{ fontSize: 56,
                         color: 'var(--accent)' }}>蔵</span>
          <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--accent)',
                         textTransform: 'uppercase',
                         fontWeight: 500
}} className="mt-3 mb-4" >
            Privacy & local-first
          </div>
          <h2 className="display m-0 ml-auto mr-auto" style={{
 fontSize: 56, fontWeight: 300, letterSpacing: '-0.025em',
                         lineHeight: 1.1, maxWidth: 720
}}>
            Your sessions stay on your machine.
          </h2>
        </div>
        <div style={{
 display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)'
}} className="gap-5" >
          {[
            { k: "蔵", title: "Local storage only",
              text: "Transcripts, patterns, memories — all in a SQLite file under your home directory. No outbound network requests, ever." },
            { k: "鍵", title: "No telemetry",
              text: "We don't track usage. Updates are checked manually from Help → Check for Updates." },
            { k: "破", title: "Easy to delete",
              text: "One folder. Delete it and sensei forgets everything. Export to JSON anytime." }
          ].map((it, i) => (
            <div key={i} style={{
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 12
}} className="py-5 px-5" >
              <span className="kanji" style={{ fontSize: 28,
                             color: 'var(--ink-2)' }}>{it.k}</span>
              <div className="display mt-3 mb-2" style={{
 fontSize: 17,
                             letterSpacing: '-0.01em'
}}>{it.title}</div>
              <div style={{ fontSize: 13, color: 'var(--ink-2)',
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
      background: `radial-gradient(ellipse at center, oklch(0.58 0.15 35 / 0.08) 0%, transparent 60%), var(--paper)`,
      borderTop: 'var(--hairline)',
      borderBottom: 'var(--hairline)',
      textAlign: 'center'
}} className="py-9 px-7" >
      <div style={{ maxWidth: 760 }} className="mx-auto" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--accent)',
                       textTransform: 'uppercase',
                       fontWeight: 500
}} className="mb-4" >
          Pricing
        </div>
        <h2 className="display mt-0 mb-5" style={{
 fontSize: 56, fontWeight: 300, letterSpacing: '-0.03em',
                       lineHeight: 1
}}>
          Free.<br/>
          <span style={{
            background: 'linear-gradient(95deg, var(--accent) 0%, oklch(0.66 0.15 60) 100%)',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent'
          }}>
            Pay what feels right.
          </span>
        </h2>
        <p style={{
 fontSize: 17, color: 'var(--ink-2)',
                     lineHeight: 1.65
}} className="m-0" >
          Sensei is free to download and use forever. If it earns a place
          in your daily practice, you can support development below — but
          there's no nag, no trial, no upgrade prompt. Ever.
        </p>
        <div className="mt-7" >
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
    <section id="faq" className="py-9 px-7" >
      <div style={{ maxWidth: 960 }} className="mx-auto" >
        <div style={{ textAlign: 'center' }} className="mb-8" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--accent)',
                         textTransform: 'uppercase',
                         fontWeight: 500
}} className="mb-4" >
            Frequently asked
          </div>
          <h2 className="display m-0" style={{
 fontSize: 56, fontWeight: 300, letterSpacing: '-0.025em',
                         lineHeight: 1.1
}}>
            Common questions, plain answers.
          </h2>
        </div>
        <div>
          {qs.map((it, i) => (
            <details key={i} style={{
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 10
}} className="mb-3 py-4 px-5" >
              <summary style={{
                cursor: 'pointer',
                listStyle: 'none',
                display: 'flex', justifyContent: 'space-between',
                fontSize: 15, color: 'var(--ink)',
                fontFamily: 'var(--font-display)', fontWeight: 400
              }}>
                <span>{it.q}</span>
                <span className="kanji" style={{ color: 'var(--accent)' }}>+</span>
              </summary>
              <div style={{
 fontSize: 13, color: 'var(--ink-2)',
                             lineHeight: 1.7
}} className="mt-3" >
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
      background: `linear-gradient(180deg, var(--paper) 0%, var(--paper-2) 100%)`,
      borderTop: 'var(--hairline)',
      textAlign: 'center'
}} className="py-9 px-7" >
      <div style={{
 maxWidth: 720,
                     background: 'var(--paper)',
                     border: 'var(--hairline)',
                     borderRadius: 16,
                     boxShadow: '0 20px 50px -20px rgba(20,18,14,0.15)'
}} className="mx-auto py-7 px-6" >
        <span className="kanji" style={{ fontSize: 56,
                       color: 'var(--accent)' }}>志</span>
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mt-3 mb-3" >
          Support development
        </div>
        <h2 className="display mt-0 mb-4" style={{
 fontSize: 28, fontWeight: 400, letterSpacing: '-0.015em',
                       lineHeight: 1.3
}}>
          If sensei has earned a place in your practice, help keep it growing.
        </h2>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     lineHeight: 1.7
}} className="mt-0 mb-5" >
          Built by a small team. Every coffee buys an hour of focused work.
        </p>
        <a href="#sponsor" style={{
          display: 'inline-flex', alignItems: 'center',
          background: 'linear-gradient(180deg, var(--accent) 0%, oklch(0.52 0.16 30) 100%)',
          color: 'var(--paper)',
          borderRadius: 8,
          fontSize: 13, fontWeight: 500,
          textDecoration: 'none',
          boxShadow: '0 8px 20px -8px var(--accent)'
}} className="gap-2 py-3 px-5" >
          ♥ Buy me a coffee
        </a>
      </div>
    </section>
  );
}

function FooterC() {
  return (
    <footer style={{
      background: 'oklch(0.20 0.010 50)',
      color: 'oklch(0.62 0.010 85)',
      fontSize: 13
}} className="pt-8 pb-7 px-7" >
      <div style={{
 maxWidth: 1280,
                     display: 'flex', alignItems: 'flex-start',
                     justifyContent: 'space-between',
                     flexWrap: 'wrap'
}} className="gap-8 mx-auto" >
        <div style={{ maxWidth: 320 }}>
          <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2 mb-3" >
            <span className="kanji" style={{ fontSize: 22,
                           color: 'var(--accent)', letterSpacing: '-0.04em' }}>先生</span>
            <span className="display" style={{ fontSize: 17,
                           color: 'var(--paper)' }}>Sensei</span>
          </div>
          <div style={{ lineHeight: 1.6 }}>
            A patient observer for AI-assisted work. Built quietly,
            shipped slowly.
          </div>
          <div className="mono mt-4" style={{
 fontSize: 11, opacity: 0.7
}}>
            v0.4.2
          </div>
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-7" >
          <FooterColC title="Product"
            links={["Download", "Privacy", "FAQ", "Changelog"]}/>
          <FooterColC title="Source"
            links={["GitHub", "MCP", "Roadmap", "Issues"]}/>
          <FooterColC title="Connect"
            links={["Twitter", "Mastodon", "Email", "RSS"]}/>
        </div>
      </div>
      <div style={{
 maxWidth: 1280,
                     borderTop: '1px solid oklch(0.32 0.010 50)',
                     fontSize: 11, opacity: 0.6,
                     textAlign: 'center'
}} className="mt-6 mb-0 mx-auto pt-5" >
        © 2025 Sensei. Made with restraint.
      </div>
    </footer>
  );
}

function FooterColC({ title, links }) {
  return (
    <div>
      <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                     color: 'var(--accent)',
                     textTransform: 'uppercase', fontWeight: 500
}} className="mb-3" >{title}</div>
      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
        {links.map((l, i) => (
          <a key={i} href={`#${l.toLowerCase()}`}
             style={{ fontSize: 13,
                       color: 'oklch(0.78 0.008 85)' }}>{l}</a>
        ))}
      </div>
    </div>
  );
}

Object.assign(window, { VariantC });
