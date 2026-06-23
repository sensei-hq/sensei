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

// ─── Dōjō · for teams (governance) ──────────────────────────────
function DojoForTeams() {
  const eyebrow = { fontSize: 11, letterSpacing: '0.22em', color: 'var(--ink-3)', textTransform: 'uppercase' };
  const sub = { fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)', textTransform: 'uppercase', fontWeight: 600 };
  const loop = [
    ["01","Contribute","Individual","A dev marks a memory, pattern or guard to share, scoped to where it's true."],
    ["02","Accumulate","The hive","Contributions pool on the company server, clustered and deduped against existing knowledge."],
    ["03","Triage","Maintainer","Candidates are scored, conflicts surfaced, near-duplicates merged — the trust step."],
    ["04","Approve","Maintainer","A named approval publishes it at its scope, with attribution and a regression note."],
    ["05","Distribute","Everyone","Approved practice lands automatically in every matching scope's Today / Upgrades."],
  ];
  const artifacts = [
    ["理","Guiding principles","Durable engineering values and ‘how we build here’ statements."],
    ["紋","Patterns","Constructive shapes promoted to rules — and the anti-patterns to avoid."],
    ["問","Prompts","Vetted prompt templates and personas for recurring tasks."],
    ["守","Guards","Lints, checks and safety rails that gate risky changes."],
    ["技","Skills","Packaged capabilities an assistant can pick up for a task."],
    ["使","Agents","Configured agents with tools and scope, shared ready-to-run."],
  ];
  const members = [["社","Employer"],["客","Clients"],["群","Communities"],["己","Personal"]];
  const ladders = [
    { k:"守", name:"Security", rungs:[["P0","Never log secrets"],["P1","Auth needs a test persona"],["P2","Parameterize queries"]] },
    { k:"構", name:"Architecture", rungs:[["P0","APIs stay compatible"],["P1","Depend inward"],["P2","Co-locate state"]] },
    { k:"験", name:"Testing", rungs:[["P0","Hold the coverage floor"],["P1","Bug fix ships a test"],["P2","Table-driven tests"]] },
  ];
  const ptone = { P0:'var(--accent)', P1:'oklch(0.55 0.13 60)', P2:'var(--ink-3)' };
  const vs = {
    left:  { k:"群", h:"Collective", sub:"global · public commons", rows:[
      ["Hosting","Sensei community cloud"],
      ["Audience","All users · anonymized"],
      ["Membership","One public commons"],
      ["Scope","Stack-agnostic, generalised"],
      ["Trust","Reputation + aggregate signal"],
      ["Direction","Opt-in share · pull upgrades"],
      ["Control","Per-category filters, cadence"],
    ]},
    right: { k:"結", h:"Dōjō", sub:"private · governed · company-hosted", rows:[
      ["Hosting","Your infrastructure / VPC"],
      ["Audience","Your orgs · attributed"],
      ["Membership","Many — employer · clients · communities"],
      ["Scope","Company → team → project → repo → stack"],
      ["Trust","Triage + named approval"],
      ["Direction","Governed upstream + downstream loop"],
      ["Control","Roles, policies, approval gates"],
    ]},
  };
  return (
    <section id="teams" style={{ borderTop: 'var(--hairline)', background: 'var(--paper-2)' }} className="py-9 px-7" >
      <div style={{ maxWidth: 1200 }} className="mx-auto" >
        <div style={eyebrow} className="mb-3" >For teams · 結 Dōjō</div>
        <h2 className="display mt-0 mb-4" style={{ fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em' }}>
          Your team's hard-won lessons — shared, governed, routed.
        </h2>
        <p style={{ fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.65, maxWidth: 620 }} className="mb-8" >
          Sensei is local-first for one developer. <span style={{ color: 'var(--ink)' }}>Dōjō</span> is its company-hosted counterpart — a private, governed collective that turns what individuals learn into team practice, with nothing leaking.
        </p>

        {/* the loop — lifecycle cards + distributed-back return */}
        <div style={sub} className="mb-3" >The loop</div>
        <div style={{ display: 'flex', alignItems: 'stretch', gap: 0 }}>
          {loop.map(([n, t, who, d], i) => (
            <React.Fragment key={n}>
              {i > 0 && <div style={{ display: 'flex', alignItems: 'center', padding: '0 6px', color: 'var(--accent)', fontFamily: 'var(--font-mono)', fontSize: 13 }}>→</div>}
              <div style={{ flex: 1, background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 12, padding: '14px 14px', display: 'flex', flexDirection: 'column', gap: 5 }}>
                <span className="mono" style={{ fontSize: 10, color: 'var(--accent)', letterSpacing: '0.06em' }}>{n}</span>
                <span className="display" style={{ fontSize: 16, fontWeight: 500, letterSpacing: '-0.01em', color: 'var(--ink)' }}>{t}</span>
                <span style={{ fontSize: 9.5, letterSpacing: '0.1em', textTransform: 'uppercase', color: 'var(--ink-3)', fontWeight: 600 }}>{who}</span>
                <span style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.5 }}>{d}</span>
              </div>
            </React.Fragment>
          ))}
        </div>
        <div style={{ position: 'relative', height: 36, marginTop: 4 }} className="mb-8" >
          <svg viewBox="0 0 1000 36" preserveAspectRatio="none" style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', overflow: 'visible' }}>
            <path d="M 962 2 C 962 28, 942 32, 900 32 L 100 32 C 58 32, 38 28, 38 4" fill="none" stroke="var(--accent)" strokeWidth="1.4" strokeDasharray="5 4"/>
            <path d="M 38 1 L 33 11 L 43 11 Z" fill="var(--accent)"/>
          </svg>
          <div style={{ position: 'absolute', left: '50%', top: 9, transform: 'translateX(-50%)', background: 'var(--paper-2)', padding: '0 12px', fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--accent)', letterSpacing: '0.04em' }}>↑ distributed back · to every matching scope</div>
        </div>

        {/* what flows — artifact cards with descriptions */}
        <div style={sub} className="mb-3" >What flows through it</div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3 mb-8" >
          {artifacts.map(([k, n, d]) => (
            <div key={n} style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 10, padding: '15px 16px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)', lineHeight: 1 }} >{k}</span>
                <span className="display" style={{ fontSize: 14.5, fontWeight: 500, letterSpacing: '-0.01em', color: 'var(--ink)' }}>{n}</span>
              </div>
              <div style={{ fontSize: 11.5, color: 'var(--ink-3)', lineHeight: 1.5 }} className="mt-2" >{d}</div>
            </div>
          ))}
        </div>

        {/* membership & routing */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', alignItems: 'start' }} className="gap-8 mb-8" >
          <div>
            <div style={sub} className="mb-3" >One developer, many orgs</div>
            <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6 }} className="mb-3" >
              A dev can belong to several at once. Every project is bound to exactly one — so findings route only where they belong and never pollute an unrelated hive-mind.
            </p>
            <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-2" >
              {members.map(([k, n]) => (
                <span key={n} style={{ display: 'inline-flex', alignItems: 'center', gap: 6, fontSize: 12, color: 'var(--ink-2)',
                              background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 20, padding: '5px 12px' }}>
                  <span className="kanji" style={{ color: 'var(--accent)', fontSize: 13 }}>{k}</span>{n}
                </span>
              ))}
            </div>
          </div>
          <div>
            <div style={sub} className="mb-3" >Ranked by priority, not hierarchy</div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-2" >
              {ladders.map(l => (
                <div key={l.name} style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 10, padding: '11px 11px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }} className="mb-2" >
                    <span className="kanji" style={{ fontSize: 14, color: 'var(--accent)' }}>{l.k}</span>
                    <span style={{ fontSize: 12, color: 'var(--ink)', fontWeight: 500 }}>{l.name}</span>
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
                    {l.rungs.map(([p, t]) => (
                      <div key={p} style={{ display: 'flex', gap: 6, alignItems: 'baseline' }}>
                        <span className="mono" style={{ fontSize: 9, fontWeight: 600, color: ptone[p] }}>{p}</span>
                        <span style={{ fontSize: 10.5, color: 'var(--ink-2)', lineHeight: 1.35 }}>{t}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', fontStyle: 'italic' }} className="mt-2" >When principles compete, the higher rung wins.</div>
          </div>
        </div>

        {/* collective vs dojo */}
        <div style={sub} className="mb-3" >The global Collective vs. your Dōjō</div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', border: 'var(--hairline)', borderRadius: 14, overflow: 'hidden' }}>
          {[vs.left, vs.right].map((col, ci) => (
            <div key={ci} style={{ borderLeft: ci === 1 ? 'var(--hairline)' : 'none',
                          background: ci === 1 ? 'color-mix(in srgb, var(--accent) 5%, var(--paper))' : 'var(--paper)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 11, padding: '16px 20px 13px', borderBottom: 'var(--hairline)' }}>
                <span className="kanji" style={{ fontSize: 24, color: ci === 1 ? 'var(--accent)' : 'var(--ink-3)', lineHeight: 1 }}>{col.k}</span>
                <div>
                  <div className="display" style={{ fontSize: 17, fontWeight: 500, letterSpacing: '-0.01em', color: 'var(--ink)' }}>{col.h}</div>
                  <div className="mono" style={{ fontSize: 10.5, color: 'var(--ink-3)' }}>{col.sub}</div>
                </div>
              </div>
              {col.rows.map(([l, v], i) => (
                <div key={l} style={{ padding: '11px 20px', borderBottom: i < col.rows.length - 1 ? '1px solid var(--edge)' : 'none' }}>
                  <div style={{ fontSize: 9.5, letterSpacing: '0.1em', textTransform: 'uppercase', color: 'var(--ink-4)', fontWeight: 600, marginBottom: 3 }}>{l}</div>
                  <div style={{ fontSize: 12.5, color: ci === 1 ? 'var(--ink)' : 'var(--ink-2)' }}>{v}</div>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

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
      sub: "Try MCP tools in isolation, replay what the assistant did, watch toolset health over time.",
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
            { k: "蔵", title: "Local storage only",
              text: "Transcripts, patterns, memories — all stored in a SQLite file under your home directory. Sensei never makes outbound network requests." },
            { k: "鍵", title: "No telemetry",
              text: "We don't track usage. Updates are checked manually from Help → Check for Updates." },
            { k: "破", title: "Easy to delete",
              text: "One folder. Delete it and sensei forgets everything. Export to JSON anytime." }
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
          there's no nag, no trial, no upgrade prompt. Ever.
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
}}>v0.4.2</span>
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
