// VARIANT A — "Same world as the app"
// ─────────────────────────────────────────────────────────────────
// Brief: the website should feel like a quieter wing of the app —
// washi paper, sumi ink, kanji, generous breathing room, small type,
// hairline rules, almost no color. Restraint as marketing.
//
// Reasoning:
//   · The app's distinguishing feature is its restraint. A loud
//     marketing site would betray the product. A loud headline on
//     a quiet app reads as "what they wish they were."
//   · The downside is low marketing punch — visitors can scroll
//     past quiet sites. We compensate with strong typography
//     (Fraunces display) and one striking hero composition.
//   · Vermillion (--shu) is rationed: only the kanji 先生, the active
//     pattern dot, the download CTA. Everything else is sumi on paper.

const { useState: aS, useEffect: aE } = React;

function VariantA() {
  return (
    <div className="sensei variant-a" style={{
      background: 'var(--paper)', color: 'var(--ink)',
      minHeight: '100%', fontFamily: 'var(--font-ui)'
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

// ─── Dōjō · for teams — extracted to site/dojo-for-teams.jsx ──────
// ─── Nav ────────────────────────────────────────────────────────
function NavA() {
  return (
    <nav style={{
      maxWidth: 1100,
      display: 'flex', alignItems: 'center',
      justifyContent: 'space-between'
}} className="py-5 px-7 mx-auto" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
        <span className="kanji" style={{ fontSize: 22,
                       color: 'var(--accent)', letterSpacing: '-0.04em' }}>先生</span>
        <span className="display" style={{ fontSize: 17,
                       letterSpacing: '-0.01em',
                       color: 'var(--ink)' }}>Sensei</span>
      </div>
      <div style={{ display: 'flex', fontSize: 13 }} className="gap-5" >
        {[
          ['#how', 'How it works'],
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

// ─── Hero ───────────────────────────────────────────────────────
function HeroA() {
  return (
    <section style={{
 maxWidth: 1100
}} className="mx-auto pt-6 pb-8 px-7" >
      <div style={{
 display: 'grid', gridTemplateColumns: '1fr', alignItems: 'start'
}} className="gap-5" >
        <div>
          <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-3 mb-5" >
            <span className="kanji" style={{ fontSize: 56,
                           color: 'var(--accent)', lineHeight: 1 }}>観</span>
            <div style={{ fontSize: 11, letterSpacing: '0.22em',
                           color: 'var(--ink-3)',
                           textTransform: 'uppercase' }}>
              Kan · to observe
            </div>
          </div>
          <h1 className="display m-0" style={{
            fontSize: 56, fontWeight: 300, lineHeight: 1.1,
            letterSpacing: '-0.025em', maxWidth: 820
}}>
            A quiet companion for AI-assisted work.
          </h1>
          <p style={{
 fontSize: 15, color: 'var(--ink-2)',
                       lineHeight: 1.6, maxWidth: 560
}} className="mt-5" >
            Sensei watches your sessions with AI assistants —
            then surfaces the patterns you're too close to see. Not a
            chatbot. Not a copilot. A patient observer.
          </p>
          <div style={{
 display: 'flex', alignItems: 'center'
}} className="gap-3 mt-6" >
            <DownloadCTA/>
            <a href="#how" style={{ fontSize: 13,
                           color: 'var(--ink-2)' }}>
              See how it works ↓
            </a>
          </div>
          <div style={{
 fontSize: 11, color: 'var(--ink-3)'
}} className="mt-3" >
            Free · Local-first · No account
          </div>
        </div>

        {/* Hero screen — centered, generous margin */}
        <div style={{
 display: 'flex',
                       justifyContent: 'center'
}} className="mt-5" >
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
    <a href={`#download-${os.toLowerCase()}`}
       style={{
        display: 'inline-flex', alignItems: 'center',
        padding: px,
        background: 'var(--ink)',
        color: 'var(--paper)',
        borderRadius: 6,
        fontSize: fs,
        fontWeight: 500,
        textDecoration: 'none'
}} className="gap-2" >
      <span className="kanji" style={{ fontSize: fs + 2,
                     color: 'var(--accent)' }}>下</span>
      Download for {os}
    </a>
  );
}

// ─── What it is ─────────────────────────────────────────────────
function WhatItIsA() {
  return (
    <section style={{
 borderTop: 'var(--hairline)'
}} className="py-8 px-7" >
      <div style={{
 maxWidth: 1100,
                     display: 'grid',
                     gridTemplateColumns: '1fr 1.4fr', alignItems: 'start'
}} className="gap-7 mx-auto" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-3" >
            What it is
          </div>
          <h2 className="display m-0" style={{
 fontSize: 28, fontWeight: 400, letterSpacing: '-0.015em',
                         lineHeight: 1.25
}}>
            One desktop app. One quiet promise.
          </h2>
        </div>
        <div style={{ fontSize: 15, lineHeight: 1.7,
                       color: 'var(--ink-2)' }}>
          <p className="mt-0" >
            Sensei runs on your machine and observes your sessions with AI
            assistants. It sends no telemetry; it speaks rarely; it remembers
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
    <section id="how" style={{
 borderTop: 'var(--hairline)'
}} className="py-8 px-7" >
      <div style={{ maxWidth: 1100 }} className="mx-auto" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-3" >
          How it works
        </div>
        <h2 className="display mt-0 mb-7" style={{
 fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em'
}}>
          観 · 察 · 覚 — watch, notice, adopt.
        </h2>
        <div style={{
 display: 'grid',
                       gridTemplateColumns: 'repeat(3, 1fr)'
}} className="gap-7" >
          {steps.map((s, i) => (
            <div key={i}>
              <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-3 mb-4" >
                <span className="kanji" style={{ fontSize: 40,
                               color: 'var(--accent)', lineHeight: 1 }}>
                  {s.kanji}
                </span>
                <div style={{ fontSize: 11, letterSpacing: '0.22em',
                               color: 'var(--ink-3)',
                               textTransform: 'uppercase' }}>
                  {s.phase}
                </div>
              </div>
              <div style={{
 fontSize: 13, color: 'var(--ink)',
                             lineHeight: 1.65
}} className="mb-3" >
                {s.text}
              </div>
              <div style={{ fontSize: 11, color: 'var(--ink-3)',
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
      sub: "Try tools in isolation, replay what the assistant did.",
      el: <MockInstruments width={780} height={490}/> }
  ];
  return (
    <section id="gallery" style={{
 borderTop: 'var(--hairline)',
                                    background: 'var(--paper-2)'
}} className="pt-8 pb-6" >
      <div style={{ maxWidth: 1100 }} className="mx-auto px-7" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-3" >
          The screens
        </div>
        <h2 className="display mt-0 mb-3" style={{
 fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em'
}}>
          Five surfaces, one rhythm.
        </h2>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     maxWidth: 560, lineHeight: 1.6
}} className="mt-0 mb-7" >
          Every screen answers one question and stays quiet otherwise.
        </p>
      </div>
      <div style={{
 maxWidth: 1100,
                     display: 'flex', flexDirection: 'column'
}} className="gap-8 mx-auto px-7" >
        {screens.map((s, i) => (
          <div key={i} style={{
            display: 'grid',
            gridTemplateColumns: i % 2 === 0 ? '1fr 320px' : '320px 1fr', alignItems: 'center'
}} className="gap-7" >
            <div style={{ order: i % 2 === 0 ? 0 : 1 }}>{s.el}</div>
            <div style={{ order: i % 2 === 0 ? 1 : 0 }}>
              <div className="display mb-2" style={{
 fontSize: 22, fontWeight: 400
}}>{s.caption}</div>
              <div style={{ fontSize: 13, color: 'var(--ink-2)',
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
    <section id="philosophy" style={{
 borderTop: 'var(--hairline)'
}} className="py-9 px-7" >
      <div style={{ maxWidth: 760, textAlign: 'center' }} className="mx-auto" >
        <span className="kanji" style={{ fontSize: 56,
                       color: 'var(--accent)', lineHeight: 1 }}>静</span>
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mt-3" >
          Sei · stillness
        </div>
        <h2 className="display mt-6 mb-5" style={{
 fontSize: 28, fontWeight: 300, letterSpacing: '-0.02em',
                       lineHeight: 1.3
}}>
          The master observes for a long time before teaching.
        </h2>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     lineHeight: 1.75
}} className="m-0" >
          AI tools are getting louder. More suggestions, more autocompletes,
          more interrupting. Sensei moves the other way. It speaks rarely,
          and only when it has something specific to say. Most days it is
          completely silent — and that is the feature.
        </p>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     lineHeight: 1.75
}} className="mt-4" >
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
    <section id="privacy" style={{
      borderTop: 'var(--hairline)',
      background: 'var(--paper)'
}} className="py-8 px-7" >
      <div style={{
 maxWidth: 1100,
                     display: 'grid',
                     gridTemplateColumns: '1fr 1.4fr', alignItems: 'start'
}} className="gap-7 mx-auto" >
        <div>
          <span className="kanji" style={{ fontSize: 40,
                         color: 'var(--accent)' }}>蔵</span>
          <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                         color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mt-3 mb-3" >
            Privacy & local-first
          </div>
          <h2 className="display m-0" style={{
 fontSize: 28, fontWeight: 400, letterSpacing: '-0.015em',
                         lineHeight: 1.25
}}>
            Your sessions stay on your machine.
          </h2>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-5" >
          {[
            { k: "蔵", title: "Local-first storage",
              text: "Transcripts, patterns, and memories live in a local database in ~/.sensei. Nothing leaves your machine without an explicit action you take." },
            { k: "鍵", title: "No telemetry",
              text: "We don't track usage. Updates are checked manually from Help → Check for Updates." },
            { k: "破", title: "Easy to delete",
              text: "One folder. Delete it and sensei forgets everything." }
          ].map((it, i) => (
            <div key={i} style={{
 display: 'grid',
                       gridTemplateColumns: 'auto 1fr',
                       borderBottom: i < 2 ? 'var(--hairline)' : 'none'
}} className="gap-4 pb-5" >
              <span className="kanji" style={{ fontSize: 22,
                             color: 'var(--ink-2)' }}>{it.k}</span>
              <div>
                <div className="display mb-1" style={{
 fontSize: 15
}}>{it.title}</div>
                <div style={{ fontSize: 13, color: 'var(--ink-2)',
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
    <section style={{
 borderTop: 'var(--hairline)',
                       textAlign: 'center'
}} className="py-8 px-7" >
      <div style={{ maxWidth: 720 }} className="mx-auto" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-3" >
          Pricing
        </div>
        <h2 className="display mt-0 mb-4" style={{
 fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em'
}}>
          Free. Pay what feels right.
        </h2>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     lineHeight: 1.7
}} className="m-0" >
          Sensei is free to download and use forever. If it earns a place
          in your daily practice, you can support development below — but
          there's no nag and no trial while sensei is in preview.
        </p>
        <div className="mt-6" >
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
      a: "It works with Claude Code and Zed today, observing your sessions locally. More assistants as adapters land." },
    { q: "Does sensei see my code?",
      a: "Only what passes through your AI tool's session. It runs locally and stores everything in a local database in ~/.sensei you can inspect or delete at any time." },
    { q: "Will it slow down my machine?",
      a: "Sensei is a Tauri app — small binary, low memory. The observer is event-driven; it only does work when a session happens." },
    { q: "Can I export my memories?",
      a: "Export and import is on the roadmap. Today your data lives in a local database in ~/.sensei you can inspect or copy directly." },
    { q: "Can I act on a session while away from my keyboard?",
      a: "Yes — Relay. Your daemon holds a live line to your Dōjō, so from a phone or any browser you can watch progress, approve the exact command, answer a decision, or chat back to a running session. No pairing, no separate app. Free on your own projects." },
    { q: "Which models does sensei use?",
      a: "Its own gateway ships with embedded Ollama — Gemma 4 by default, fast and local, no keys needed. Point it at a bigger Ollama host or bring your own API keys for Claude, GPT-4o and others. There are no tokens to mark up." },
    { q: "What's the long-term plan?",
      a: "The core promise — quiet, local, observant — never changes, and the app stays free. Teams and orgs pay for a private, shared Dōjō (per active contributor); public, open-source and personal Dōjōs are free forever." }
  ];
  return (
    <section id="faq" style={{
 borderTop: 'var(--hairline)'
}} className="py-8 px-7" >
      <div style={{ maxWidth: 880 }} className="mx-auto" >
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-3" >
          Frequently asked
        </div>
        <h2 className="display mt-0 mb-6" style={{
 fontSize: 28, fontWeight: 400, letterSpacing: '-0.015em'
}}>
          Common questions, plain answers.
        </h2>
        <div>
          {qs.map((it, i) => (
            <details key={i} style={{
              borderTop: 'var(--hairline)',
              ...(i === qs.length - 1 ? { borderBottom: 'var(--hairline)' } : {})
}} className="py-4 px-0" >
              <summary style={{
                cursor: 'pointer',
                listStyle: 'none',
                display: 'flex', justifyContent: 'space-between',
                fontSize: 13, color: 'var(--ink)'
              }}>
                <span>{it.q}</span>
                <span className="kanji" style={{ color: 'var(--ink-3)' }}>+</span>
              </summary>
              <div style={{
 fontSize: 13, color: 'var(--ink-2)',
                             lineHeight: 1.7, maxWidth: 640
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

// ─── Support development ────────────────────────────────────────
function SupportA() {
  return (
    <section style={{
 borderTop: 'var(--hairline)',
                       background: 'var(--paper-2)',
                       textAlign: 'center'
}} className="py-8 px-7" >
      <div style={{ maxWidth: 640 }} className="mx-auto" >
        <span className="kanji" style={{ fontSize: 28,
                       color: 'var(--accent)' }}>志</span>
        <div style={{
 fontSize: 11, letterSpacing: '0.22em',
                       color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mt-2 mb-3" >
          Support development
        </div>
        <h2 className="display mt-0 mb-4" style={{
 fontSize: 22, fontWeight: 400, letterSpacing: '-0.015em',
                       lineHeight: 1.3
}}>
          If sensei has earned a place in your practice, you can help keep it growing.
        </h2>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     lineHeight: 1.7
}} className="mt-0 mb-5" >
          Sensei is built by a small team. GitHub Sponsors keeps the work focused and independent.
        </p>
        <a href="https://github.com/sponsors/sensei-hq" target="_blank" rel="noopener" style={{
          display: 'inline-flex', alignItems: 'center',
          border: '1px solid var(--ink)',
          borderRadius: 6,
          fontSize: 13,
          color: 'var(--ink)',
          textDecoration: 'none'
}} className="gap-2 py-3 px-5" >
          ♥ Sponsor on GitHub
        </a>
      </div>
    </section>
  );
}

// ─── Footer ─────────────────────────────────────────────────────
function FooterA() {
  return (
    <footer style={{
 borderTop: 'var(--hairline)',
                      fontSize: 11, color: 'var(--ink-3)'
}} className="py-6 px-7" >
      <div style={{
 maxWidth: 1100,
                     display: 'flex', alignItems: 'center',
                     justifyContent: 'space-between'
}} className="mx-auto" >
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
          <span className="kanji" style={{ fontSize: 13,
                         color: 'var(--accent)', letterSpacing: '-0.04em' }}>先生</span>
          <span className="display" style={{ fontSize: 13,
                         color: 'var(--ink-2)' }}>Sensei</span>
          <span className="mono ml-3" style={{
 fontSize: 11
}}>{(typeof window !== "undefined" && window.__APP_VERSION__) || "preview"}</span>
        </div>
        <div style={{ display: 'flex' }} className="gap-5" >
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
