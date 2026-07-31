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
    <div className="sensei variant-c bg-paper text-ink min-h-full" style={{ fontFamily: 'var(--font-ui)'
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
    <nav className="sticky border-b" style={{ top: 0, zIndex: 10,
 backdropFilter: 'blur(20px)',
 background: 'oklch(0.975 0.008 85 / 0.85)' }}>
      <div style={{
 maxWidth: 1280 }} className="py-4 px-12 mx-auto flex items-center justify-between" >
        <div className="gap-2 flex items-baseline" >
          <span className="kanji text-accent" style={{ fontSize: 22, letterSpacing: '-0.04em' }}>先生</span>
          <span className="display text-ink" style={{ fontSize: 17,
 letterSpacing: '-0.01em' }}>Sensei</span>
        </div>
        <div style={{ fontSize: 13 }} className="gap-6 flex items-center" >
          {[
            ['#how', 'How'],
            ['#gallery', 'Screens'],
            ['#philosophy', 'Philosophy'],
            ['#privacy', 'Privacy'],
            ['#faq', 'FAQ']
          ].map(([href, label]) => (
            <a className="text-ink-2 no-underline" key={href} href={href}
 style={{
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
 background: `radial-gradient(ellipse at 70% 20%, oklch(0.58 0.15 35 / 0.10) 0%, transparent 55%),
 radial-gradient(ellipse at 20% 80%, oklch(0.62 0.08 160 / 0.08) 0%, transparent 50%),
 var(--paper)`
 }} className="pt-16 pb-0 px-12 relative overflow-hidden" >
      <div style={{
 maxWidth: 1280,
 gridTemplateColumns: '1.1fr 1fr' }} className="gap-16 mx-auto pb-24 grid items-center" >
        <div className="relative" >
          <div style={{
 borderRadius: 999,
 fontSize: 11 }} className="gap-2 py-1 px-3 mb-6 inline-flex items-center bg-paper-2 border border-paper-edge text-ink-2" >
            <span className="ink-dot bg-success" style={{ width: 6, height: 6 }}/>
            v0.4.2 · now in public preview
          </div>
          <h1 className="display m-0 font-light" style={{
 fontSize: 56, lineHeight: 1.02,
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
 fontSize: 17,
 lineHeight: 1.6, maxWidth: 520
 }} className="mt-6 text-ink-2" >
            Sensei observes your sessions with AI assistants —
            then surfaces the patterns you're too close to see. Not a
            chatbot. Not a copilot. <em>A patient observer.</em>
          </p>
          <div className="gap-3 mt-8 flex items-center" >
            <DownloadCTAC size="lg"/>
            <a href="#how" style={{
 border: '1px solid var(--edge)',
 borderRadius: 8,
 fontSize: 13 }} className="gap-2 py-3 px-6 inline-flex items-center text-ink bg-paper no-underline" >
              How it works ↓
            </a>
          </div>
          <div style={{
 fontSize: 11 }} className="gap-4 mt-6 flex text-ink-3" >
            <span>✓ Free</span>
            <span>✓ Local-first</span>
            <span>✓ No account</span>
          </div>
        </div>

        {/* Floating screenshot stack */}
        <div className="relative" style={{ height: 540 }}>
          <div className="absolute" style={{ right: -40, top: 0,
 transform: 'rotate(2deg)',
 opacity: 0.95
 }}>
            <MockSessions width={620} height={400}/>
          </div>
          <div className="absolute" style={{ left: -40, bottom: 0,
 transform: 'rotate(-2deg)'
 }}>
            <MockToday width={620} height={400}/>
          </div>
          {/* Big floating kanji as art object */}
          <div style={{ right: -120, top: -60,
 fontSize: 56, lineHeight: 1, opacity: 0.08,
 pointerEvents: 'none',
 letterSpacing: '-0.04em'
 }} className="kanji absolute text-accent">先生</div>
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
 padding: px,
 background: 'linear-gradient(180deg, var(--ink) 0%, oklch(0.18 0.012 50) 100%)',
 borderRadius: 8,
 fontSize: fs,
 boxShadow: '0 8px 24px -8px rgba(20,18,14,0.5), inset 0 1px 0 rgba(255,255,255,0.08)'
 }} className="gap-2 inline-flex items-center text-paper font-medium no-underline" >
      <span className="kanji text-accent" style={{ fontSize: fs + 3 }}>下</span>
      Download for {os}
    </a>
  );
}

// "As featured on" / pretend-press strip — fixture-only
function LogoStripC() {
  const items = ["MCP", "AI assistants", "MCP-compatible tools", "Tauri", "SQLite"];
  return (
    <section className="py-6 px-12 border-t border-b bg-paper-2" >
      <div style={{
 maxWidth: 1280,
 fontSize: 11,
 letterSpacing: '0.12em' }} className="mx-auto flex items-center justify-between text-ink-3 uppercase" >
        <span className="shrink-0" >Works alongside</span>
        <div style={{
 fontSize: 13, fontFamily: 'var(--font-display)',
 letterSpacing: '-0.01em' }} className="gap-8 flex normal-case text-ink-2" >
          {items.map((l, i) => <span key={i}>{l}</span>)}
        </div>
      </div>
    </section>
  );
}

function WhatItIsC() {
  return (
    <section className="py-24 px-12" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div className="mb-16 text-center" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mb-4 text-accent uppercase font-medium" >
            What it is
          </div>
          <h2 className="display mx-auto font-light" style={{
 fontSize: 56, letterSpacing: '-0.025em',
 lineHeight: 1.1, maxWidth: 780
 }}>
            One desktop app. One quiet promise.
          </h2>
        </div>
        <div style={{
 gridTemplateColumns: 'repeat(3, 1fr)'
 }} className="gap-6 grid" >
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
 borderRadius: 16 }} className="py-8 px-6 bg-paper-2 border border-paper-edge relative overflow-hidden" >
              <div style={{ right: -12, top: -16,
 fontSize: 56, lineHeight: 1,
 color: `var(--${f.tone}-soft)`,
 pointerEvents: 'none'
 }} className="kanji absolute">{f.k}</div>
              <div className="kanji mb-4 relative" style={{
 fontSize: 28,
 color: `var(--${f.tone})` }}>
                {f.k}
              </div>
              <h3 className="display mt-0 mb-3 font-normal" style={{
 fontSize: 22,
 letterSpacing: '-0.01em'
 }}>
                {f.title}
              </h3>
              <div className="text-ink-2" style={{ fontSize: 13,
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
 background: `linear-gradient(180deg, var(--paper) 0%, var(--paper-2) 100%)`
 }} className="py-24 px-12 border-t border-b" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div className="mb-16 text-center" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mb-4 text-accent uppercase font-medium" >
            How it works
          </div>
          <h2 className="display m-0 font-light" style={{
 fontSize: 56, letterSpacing: '-0.025em',
 lineHeight: 1.1
 }}>
            Watch → Notice → Adopt
          </h2>
        </div>
        <div style={{
 gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-6 grid relative" >
          {/* connecting line */}
          <div className="absolute" style={{ top: 60,
 left: '16%', right: '16%', height: 1,
 background: 'var(--edge)',
 pointerEvents: 'none' }}/>
          {steps.map((s, i) => (
            <div key={i} style={{
 borderRadius: 16 }} className="py-8 px-8 bg-paper border border-paper-edge relative" >
              <div style={{
 width: 64, height: 64,
 background: `var(--${s.tone}-soft)` }} className="mb-6 rounded-full flex items-center justify-center" >
                <span className="kanji" style={{ fontSize: 28,
                               color: `var(--${s.tone})` }}>{s.kanji}</span>
              </div>
              <div className="mono mb-2" style={{
 fontSize: 11,
                             color: `var(--${s.tone})`
}}>{s.phase}</div>
              <h3 className="display mt-0 mb-3 font-normal" style={{
 fontSize: 22,
 letterSpacing: '-0.01em'
 }}>{s.title}</h3>
              <div style={{
 fontSize: 13,
 lineHeight: 1.65
 }} className="mb-4 text-ink-2" >
                {s.text}
              </div>
              <div style={{
 fontSize: 11 }} className="pt-3 text-ink-3 border-t" >
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
    <section id="gallery" className="pt-24 pb-16 px-12" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div className="mb-16 text-center" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mb-4 text-accent uppercase font-medium" >
            The screens
          </div>
          <h2 className="display m-0 font-light" style={{
 fontSize: 56, letterSpacing: '-0.025em',
 lineHeight: 1.1
 }}>
            Five surfaces, one rhythm.
          </h2>
        </div>
        <div className="flex flex-col" style={{ gap: 96 }}>
          {screens.map((s, i) => (
            <div key={i} style={{
 gridTemplateColumns: i % 2 === 0 ? '1fr 360px' : '360px 1fr' }} className="gap-16 grid items-center" >
              <div className="relative" style={{
 order: i % 2 === 0 ? 0 : 1 }}>
                <div className="absolute" style={{
 inset: -32,
 background: `radial-gradient(ellipse at center, var(--${s.tone}-soft) 0%, transparent 65%)`,
 pointerEvents: 'none',
 zIndex: 0
 }}/>
                <div className="relative" style={{ zIndex: 1 }}>{s.el}</div>
              </div>
              <div style={{ order: i % 2 === 0 ? 1 : 0 }}>
                <div className="mono mb-2" style={{
 fontSize: 13,
                               color: `var(--${s.tone})`,
                               letterSpacing: '0.1em'
}}>
                  0{i + 1} / 05
                </div>
                <div className="display mb-4 font-normal" style={{
 fontSize: 40,
 letterSpacing: '-0.02em'
 }}>{s.caption}</div>
                <div className="text-ink-2" style={{ fontSize: 15,
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
 background: `linear-gradient(180deg, oklch(0.22 0.012 50) 0%, oklch(0.18 0.010 50) 100%)` }} className="py-24 px-12 text-paper relative overflow-hidden" >
      <div style={{ left: '50%', top: '50%',
 transform: 'translate(-50%, -50%)',
 fontSize: 56, lineHeight: 1,
 color: 'oklch(0.58 0.15 35 / 0.08)',
 pointerEvents: 'none'
 }} className="kanji absolute">静</div>
      <div style={{
 maxWidth: 820 }} className="mx-auto text-center relative" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mb-6 text-accent uppercase font-medium" >
          Sei · stillness
        </div>
        <h2 className="display mt-0 mb-8 font-light text-paper" style={{
 fontSize: 56, letterSpacing: '-0.025em',
 lineHeight: 1.18 }}>
          The master observes for a long time before teaching.
        </h2>
        <p style={{
 fontSize: 17, color: 'oklch(0.78 0.008 85)',
                     lineHeight: 1.7
}} className="mt-0 mb-6" >
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
    <section id="privacy" className="py-24 px-12 bg-paper" >
      <div style={{ maxWidth: 1280 }} className="mx-auto" >
        <div className="mb-16 text-center" >
          <span className="kanji text-accent" style={{ fontSize: 56 }}>蔵</span>
          <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mt-3 mb-4 text-accent uppercase font-medium" >
            Privacy & local-first
          </div>
          <h2 className="display m-0 ml-auto mr-auto font-light" style={{
 fontSize: 56, letterSpacing: '-0.025em',
 lineHeight: 1.1, maxWidth: 720
 }}>
            Your sessions stay on your machine.
          </h2>
        </div>
        <div style={{
 gridTemplateColumns: 'repeat(3, 1fr)'
 }} className="gap-6 grid" >
          {[
            { k: "蔵", title: "Local storage only",
              text: "Transcripts, patterns, memories — all in a SQLite file under your home directory. No outbound network requests, ever." },
            { k: "鍵", title: "No telemetry",
              text: "We don't track usage. Updates are checked manually from Help → Check for Updates." },
            { k: "破", title: "Easy to delete",
              text: "One folder. Delete it and sensei forgets everything. Export to JSON anytime." }
          ].map((it, i) => (
            <div key={i} style={{
 borderRadius: 12
 }} className="py-6 px-6 bg-paper-2 border border-paper-edge" >
              <span className="kanji text-ink-2" style={{ fontSize: 28 }}>{it.k}</span>
              <div className="display mt-3 mb-2" style={{
 fontSize: 17,
                             letterSpacing: '-0.01em'
}}>{it.title}</div>
              <div className="text-ink-2" style={{ fontSize: 13,
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
 background: `radial-gradient(ellipse at center, oklch(0.58 0.15 35 / 0.08) 0%, transparent 60%), var(--paper)` }} className="py-24 px-12 border-t border-b text-center" >
      <div style={{ maxWidth: 760 }} className="mx-auto" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mb-4 text-accent uppercase font-medium" >
          Pricing
        </div>
        <h2 className="display mt-0 mb-6 font-light" style={{
 fontSize: 56, letterSpacing: '-0.03em',
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
 fontSize: 17,
 lineHeight: 1.65
 }} className="m-0 text-ink-2" >
          Sensei is free to download and use forever. If it earns a place
          in your daily practice, you can support development below — but
          there's no nag, no trial, no upgrade prompt. Ever.
        </p>
        <div className="mt-12" >
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
    <section id="faq" className="py-24 px-12" >
      <div style={{ maxWidth: 960 }} className="mx-auto" >
        <div className="mb-16 text-center" >
          <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mb-4 text-accent uppercase font-medium" >
            Frequently asked
          </div>
          <h2 className="display m-0 font-light" style={{
 fontSize: 56, letterSpacing: '-0.025em',
 lineHeight: 1.1
 }}>
            Common questions, plain answers.
          </h2>
        </div>
        <div>
          {qs.map((it, i) => (
            <details key={i} style={{
 borderRadius: 10
 }} className="mb-3 py-4 px-6 bg-paper-2 border border-paper-edge" >
              <summary className="cursor-pointer flex justify-between text-ink font-normal" style={{
 listStyle: 'none',
 fontSize: 15,
 fontFamily: 'var(--font-display)' }}>
                <span>{it.q}</span>
                <span className="kanji text-accent" >+</span>
              </summary>
              <div style={{
 fontSize: 13,
 lineHeight: 1.7
 }} className="mt-3 text-ink-2" >
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
 background: `linear-gradient(180deg, var(--paper) 0%, var(--paper-2) 100%)` }} className="py-24 px-12 border-t text-center" >
      <div style={{
 maxWidth: 720,
 borderRadius: 16,
 boxShadow: '0 20px 50px -20px rgba(20,18,14,0.15)'
 }} className="mx-auto py-12 px-8 bg-paper border border-paper-edge" >
        <span className="kanji text-accent" style={{ fontSize: 56 }}>志</span>
        <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mt-3 mb-3 text-ink-3 uppercase" >
          Support development
        </div>
        <h2 className="display mt-0 mb-4 font-normal" style={{
 fontSize: 28, letterSpacing: '-0.015em',
 lineHeight: 1.3
 }}>
          If sensei has earned a place in your practice, help keep it growing.
        </h2>
        <p style={{
 fontSize: 13,
 lineHeight: 1.7
 }} className="mt-0 mb-6 text-ink-2" >
          Built by a small team. GitHub Sponsors keeps the work focused and independent.
        </p>
        <a href="https://github.com/sponsors/sensei-hq" target="_blank" rel="noopener" style={{
 background: 'linear-gradient(180deg, var(--accent) 0%, oklch(0.52 0.16 30) 100%)',
 borderRadius: 8,
 fontSize: 13,
 boxShadow: '0 8px 20px -8px var(--accent)'
 }} className="gap-2 py-3 px-6 inline-flex items-center text-paper font-medium no-underline" >
          ♥ Sponsor on GitHub
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
}} className="pt-16 pb-12 px-12" >
      <div style={{
 maxWidth: 1280 }} className="gap-16 mx-auto flex items-start justify-between flex-wrap" >
        <div style={{ maxWidth: 320 }}>
          <div className="gap-2 mb-3 flex items-baseline" >
            <span className="kanji text-accent" style={{ fontSize: 22, letterSpacing: '-0.04em' }}>先生</span>
            <span className="display text-paper" style={{ fontSize: 17 }}>Sensei</span>
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
        <div className="gap-12 flex flex-wrap" >
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
 fontSize: 11, opacity: 0.6 }} className="mt-8 mb-0 mx-auto pt-6 text-center" >
        © 2025 Sensei. Made with restraint.
      </div>
    </footer>
  );
}

function FooterColC({ title, links }) {
  return (
    <div>
      <div style={{
 fontSize: 11, letterSpacing: '0.22em' }} className="mb-3 text-accent uppercase font-medium" >{title}</div>
      <div className="gap-2 flex flex-col" >
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
