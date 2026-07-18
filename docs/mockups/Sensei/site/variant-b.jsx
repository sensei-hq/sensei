// VARIANT B — "Confident continuity"
// ─────────────────────────────────────────────────────────────────
// Same palette, fonts and kanji vocabulary as the app — turned up for
// a marketing visitor. Bigger display type, more rhythm between
// sections, shu (vermillion) only where it earns its place.
//
// Design-system conventions (site tokens.css): named type classes
// (zs-hero / zs-display-lg / zs-h1..h3 / zs-body / zs-eyebrow / zs-meta)
// and semantic color tokens (--ink / --ink-soft / --ink-mute /
// --ink-faint, --paper / --paper-soft). Inline style is reserved for
// geometry and responsive branching. Hero owns the top type stop
// (zs-hero, 56); every section heading steps down to zs-display-lg (40)
// or smaller, so hierarchy reads.

const { useState: bS, useEffect: bE } = React;

// Single responsive hook — md ≤900 (tablet), sm ≤620 (phone).
function useBP() {
  const [w, setW] = bS(() => (typeof window !== 'undefined' ? window.innerWidth : 1200));
  bE(() => {
    const on = () => setW(window.innerWidth);
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  }, []);
  return { w, md: w <= 900, sm: w <= 620 };
}

// Section shell — consistent max-width column + responsive gutters.
function Shell({ id, bg, top, bottom, style, inner, children }) {
  const bp = useBP();
  return (
    <section id={id} style={{
      background: bg,
      borderTop: top ? 'var(--hairline)' : undefined,
      borderBottom: bottom ? 'var(--hairline)' : undefined,
      padding: bp.sm ? 'var(--space-7) var(--space-5)' : 'var(--space-9) var(--space-7)',
      ...style,
    }}>
      <div style={{ maxWidth: 1200, margin: '0 auto', ...inner }}>{children}</div>
    </section>
  );
}

function VariantB() {
  return (
    <div className="sensei variant-b" style={{
      background: 'var(--paper)', color: 'var(--ink)',
      minHeight: '100%', fontFamily: 'var(--font-ui)'
    }}>
      <NavB/>
      <HeroB/>
      <TrustStripB/>
      <StatsB/>
      <WhatItIsB/>
      <HowItWorksB/>
      <InstallB/>
      <Surfaces/>
      <DojoForTeams/>
      <RelayB/>
      <PhilosophyB/>
      <PrivacyB/>
      <PricingB/>
      <RoadmapB/>
      <FaqB/>
      <SupportB/>
      <FooterB/>
    </div>
  );
}

const NAV_LINKS = [
  ['#how', 'How'],
  ['#gallery', 'Screens'],
  ['#teams', 'Teams'],
  ['#relay', 'Relay'],
  ['#pricing', 'Pricing'],
  ['#roadmap', 'Roadmap'],
  ['#faq', 'FAQ'],
];

function NavB() {
  const bp = useBP();
  const [open, setOpen] = bS(false);
  bE(() => { if (!bp.md && open) setOpen(false); }, [bp.md, open]);
  return (
    <nav style={{ position: 'sticky', top: 0, zIndex: 40, background: 'var(--paper)', borderBottom: 'var(--hairline)' }}>
      <div style={{ maxWidth: 1200, margin: '0 auto', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
           className="py-4 px-7">
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2">
          <span style={{ display: 'inline-block', width: 26, height: 26, background: 'var(--accent)',
                         WebkitMaskImage: 'url(uploads/sensei.svg?v=3)', maskImage: 'url(uploads/sensei.svg?v=3)',
                         WebkitMaskSize: 'contain', maskSize: 'contain', WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
                         WebkitMaskPosition: 'center', maskPosition: 'center', alignSelf: 'center', flexShrink: 0 }} />
          <span className="display" style={{ fontSize: 'var(--text-xl)', letterSpacing: '-0.01em', color: 'var(--ink)' }}>Sensei</span>
        </div>

        {!bp.md && (
          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-6">
            {NAV_LINKS.map(([href, label]) => (
              <a key={href} href={href}
                 style={{ fontSize: 'var(--text-sm)', color: 'var(--ink-soft)', textDecoration: 'none', transition: 'color .15s' }}
                 onMouseEnter={(e) => e.currentTarget.style.color = 'var(--ink)'}
                 onMouseLeave={(e) => e.currentTarget.style.color = 'var(--ink-soft)'}>
                {label}
              </a>
            ))}
            <DownloadCTAB size="sm"/>
          </div>
        )}

        {bp.md && (
          <button onClick={() => setOpen(o => !o)} aria-label="Menu"
            style={{ display: 'inline-flex', flexDirection: 'column', gap: 4, background: 'none', border: 'none', cursor: 'pointer', padding: 6 }}>
            {[0, 1, 2].map(i => <span key={i} style={{ width: 22, height: 2, background: 'var(--ink)', borderRadius: 2 }} />)}
          </button>
        )}
      </div>

      {bp.md && open && (
        <div style={{ borderTop: 'var(--hairline)', background: 'var(--paper)' }} className="px-7 py-5">
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-4">
            {NAV_LINKS.map(([href, label]) => (
              <a key={href} href={href} onClick={() => setOpen(false)}
                 style={{ fontSize: 'var(--text-lg)', color: 'var(--ink-soft)', textDecoration: 'none' }}>{label}</a>
            ))}
            <div className="mt-2"><DownloadCTAB size="lg"/></div>
          </div>
        </div>
      )}
    </nav>
  );
}

function HeroB() {
  const bp = useBP();
  return (
    <Shell top={false} bottom={false} style={{ position: 'relative', paddingBottom: 'var(--space-8)' }}>
      {!bp.md && (
        <div style={{ position: 'absolute', right: 56, top: 24, fontSize: 'var(--text-4xl)', lineHeight: 1,
                       color: 'var(--accent-soft)', pointerEvents: 'none' }} className="kanji">観</div>
      )}
      <div style={{ position: 'relative' }}>
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-3 mb-5">
          <span className="ink-dot" style={{ background: 'var(--accent)', width: 8, height: 8 }}/>
          <div className="zs-eyebrow">Sensei · the patient observer</div>
        </div>
        <h1 className="display m-0" style={{
          fontSize: bp.sm ? 'var(--text-3xl)' : 'var(--text-4xl)', fontWeight: 300, lineHeight: 1.02,
          letterSpacing: '-0.03em', maxWidth: 920 }}>
          A quiet companion<br/>
          for AI-assisted <em style={{ color: 'var(--accent)', fontStyle: 'normal' }}>work</em>.
        </h1>
        <p style={{ fontSize: bp.sm ? 'var(--text-lg)' : 'var(--text-xl)', color: 'var(--ink-soft)',
                     lineHeight: 1.55, maxWidth: 640, fontFamily: 'var(--font-display)', fontWeight: 300 }} className="mt-6">
          Sensei watches your sessions with AI assistants — then surfaces the patterns you're too close to
          see. Not a chatbot. Not a copilot. A patient observer.
        </p>
        <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap' }} className="gap-4 mt-6">
          <DownloadCTAB size="lg"/>
          <a href="#how" style={{ fontSize: 'var(--text-sm)', color: 'var(--ink-soft)' }}>See how it works ↓</a>
        </div>
        <div className="zs-meta mt-4">Free · Local-first · No account required</div>
      </div>
      <div style={{ display: 'flex', justifyContent: 'center', position: 'relative' }} className="mt-8">
        <HeroBrief/>
      </div>
    </Shell>
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
  const px = size === "lg" ? 'var(--space-4) var(--space-6)' : 'var(--space-2) var(--space-4)';
  const fs = size === "lg" ? 'var(--text-base)' : 'var(--text-sm)';
  return (
    <a href={`#download-${os.toLowerCase()}`}
       style={{ display: 'inline-flex', alignItems: 'center', padding: px, background: 'var(--ink)',
        color: 'var(--paper)', borderRadius: 'var(--radius-lg)', fontSize: fs, fontWeight: 500,
        textDecoration: 'none', boxShadow: 'var(--shadow-cta)' }} className="gap-3">
      <span className="kanji" style={{ color: 'var(--accent)' }}>下</span>
      Download for {os}
    </a>
  );
}

// Trust strip — what it speaks / works with (dev-tool social proof). B5.
function TrustStripB() {
  const items = ['Speaks MCP', 'Claude', 'GPT-4o', 'Gemma', 'Ollama', 'Tauri · <60MB'];
  return (
    <section style={{ borderBottom: 'var(--hairline)', background: 'var(--paper)' }} className="py-4 px-7">
      <div style={{ maxWidth: 1200, margin: '0 auto', display: 'flex', flexWrap: 'wrap', alignItems: 'center',
                     justifyContent: 'center' }} className="gap-2">
        <span className="zs-eyebrow" style={{ marginRight: 'var(--space-2)' }}>Works with</span>
        {items.map((t, i) => (
          <React.Fragment key={t}>
            {i > 0 && <span style={{ color: 'var(--ink-faint)' }}>·</span>}
            <span className="mono" style={{ fontSize: 'var(--text-sm)', color: 'var(--ink-soft)' }}>{t}</span>
          </React.Fragment>
        ))}
      </div>
    </section>
  );
}

function StatsB() {
  const bp = useBP();
  const stats = [
    { v: "FTR", k: "first-turn resolution" },
    { v: "<60MB", k: "memory footprint" },
    { v: "Local", k: "first · no telemetry" },
    { v: "Free", k: "during preview" },
  ];
  return (
    <section style={{ borderTop: 'var(--hairline)', borderBottom: 'var(--hairline)', background: 'var(--paper-soft)' }}
             className="py-6 px-7">
      <div style={{ maxWidth: 1200, margin: '0 auto', display: 'grid',
                     gridTemplateColumns: bp.sm ? '1fr' : bp.md ? 'repeat(2, 1fr)' : 'repeat(4, 1fr)' }} className="gap-6">
        {stats.map((s, i) => (
          <div key={i} style={{ textAlign: 'center' }}>
            <div className="display" style={{ fontSize: 'var(--text-2xl)', fontWeight: 400, color: 'var(--ink)' }}>{s.v}</div>
            <div className="zs-eyebrow mt-1">{s.k}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

function WhatItIsB() {
  const bp = useBP();
  return (
    <Shell>
      <div style={{ display: 'grid', gridTemplateColumns: bp.md ? '1fr' : '1fr 1.6fr', alignItems: 'start' }} className="gap-8">
        <div>
          <div className="zs-eyebrow mb-4">What it is</div>
          <h2 className="zs-display-lg m-0">One desktop app.<br/>One quiet promise.</h2>
        </div>
        <div style={{ fontFamily: 'var(--font-display)', fontWeight: 300 }}>
          <p className="zs-body mt-1" style={{ fontSize: 'var(--text-lg)', lineHeight: 1.65 }}>
            Sensei runs on your machine and observes your sessions with AI assistants. It sends no telemetry;
            it speaks rarely; it remembers what you've actually done.
          </p>
          <p className="zs-body" style={{ fontSize: 'var(--text-lg)', lineHeight: 1.65 }}>
            Over weeks, it begins to recognize your patterns — the idioms you gravitate toward, the workarounds
            you've adopted, the friction points that keep recurring. When something looks worth noticing, it
            tells you. The rest of the time, it stays out of the way.
          </p>
        </div>
      </div>
    </Shell>
  );
}

function HowItWorksB() {
  const bp = useBP();
  const steps = [
    { kanji: "観", phase: "Watch", title: "It sits beside you",
      text: "Sensei sits beside your editor and AI tools, capturing the shape of each session — the prompts, the responses, the corrections.",
      sub: "Local only. Nothing leaves your machine." },
    { kanji: "察", phase: "Notice", title: "It begins to see",
      text: "After a few days, patterns surface. Recurring frictions. Idioms forming. Things you taught the assistant once and may want to teach it again.",
      sub: "You decide what's signal and what isn't." },
    { kanji: "覚", phase: "Adopt", title: "It remembers, with consent",
      text: "Worthy patterns become memories — small, named lessons sensei applies to future sessions on your behalf, with your blessing.",
      sub: "Adopt, refine, or dismiss. Always your call." },
  ];
  return (
    <Shell id="how" top bottom bg="var(--paper-soft)">
      <div className="zs-eyebrow mb-4">How it works</div>
      <h2 className="zs-display-lg mt-0 mb-8">
        <span style={{ color: 'var(--accent)' }}>観 · 察 · 覚</span><br/>Watch, notice, adopt.
      </h2>
      <div style={{ display: 'grid', gridTemplateColumns: bp.md ? '1fr' : 'repeat(3, 1fr)' }} className="gap-8">
        {steps.map((s, i) => (
          <div key={i} style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 'var(--radius-lg)' }}
               className="py-6 px-5">
            <div className="kanji mb-4" style={{ fontSize: 'var(--text-4xl)', color: 'var(--accent)', lineHeight: 1 }}>{s.kanji}</div>
            <div className="zs-eyebrow mb-2">{s.phase}</div>
            <h3 className="zs-h2 mt-0 mb-4">{s.title}</h3>
            <div className="zs-body-sm mb-4" style={{ lineHeight: 1.65 }}>{s.text}</div>
            <div style={{ fontSize: 'var(--text-xs)', color: 'var(--ink-mute)', fontStyle: 'italic', borderTop: 'var(--hairline)' }}
                 className="pt-3">{s.sub}</div>
          </div>
        ))}
      </div>
    </Shell>
  );
}

// Quickstart / install — the dev-tool essential. B5.
function CopyRow({ label, cmd }) {
  const [copied, setCopied] = bS(false);
  const copy = () => {
    try { navigator.clipboard && navigator.clipboard.writeText(cmd); } catch (e) {}
    setCopied(true); setTimeout(() => setCopied(false), 1400);
  };
  return (
    <div>
      <div className="zs-eyebrow mb-2">{label}</div>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 'var(--space-3)',
                     background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 'var(--radius)',
                     padding: 'var(--space-3) var(--space-4)' }}>
        <code className="mono" style={{ fontSize: 'var(--text-sm)', color: 'var(--ink)', overflowX: 'auto', whiteSpace: 'nowrap' }}>
          <span style={{ color: 'var(--ink-faint)' }}>$ </span>{cmd}
        </code>
        <button onClick={copy} style={{ flexShrink: 0, background: 'none', border: 'none', cursor: 'pointer',
                       fontSize: 'var(--text-xs)', color: copied ? 'var(--success)' : 'var(--accent)', fontFamily: 'var(--font-mono)' }}>
          {copied ? '✓ copied' : 'copy'}
        </button>
      </div>
    </div>
  );
}

function InstallB() {
  const bp = useBP();
  return (
    <Shell id="install" top>
      <div style={{ display: 'grid', gridTemplateColumns: bp.md ? '1fr' : '1fr 1.3fr', alignItems: 'start' }} className="gap-8">
        <div>
          <div className="zs-eyebrow mb-4">Quickstart</div>
          <h2 className="zs-display-lg m-0">Up and observing<br/>in one command.</h2>
          <p className="zs-body mt-5" style={{ maxWidth: 380 }}>
            Install the desktop app, or drop the daemon into an existing setup. No account, no config —
            it starts listening the moment it's open.
          </p>
          <a href="#docs" style={{ display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2)',
                       fontSize: 'var(--text-sm)', color: 'var(--accent)', textDecoration: 'none' }} className="mt-5">
            Read the docs →
          </a>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-4">
          <CopyRow label="macOS · Homebrew" cmd="brew install --cask sensei" />
          <CopyRow label="Linux · install script" cmd="curl -fsSL sensei.sh/install | sh" />
          <CopyRow label="Any editor · MCP daemon" cmd="npx @sensei/daemon start" />
        </div>
      </div>
    </Shell>
  );
}

function RelayB() {
  const bp = useBP();
  const acts = [
    { k: "認", t: "Approve the exact command", d: "The command shown verbatim, with its blast radius — approve, deny, or ask." },
    { k: "決", t: "Answer a decision", d: "A short question, three or four options, or type your own. Other tracks keep moving." },
    { k: "場", t: "Watch progress", d: "Every track running for you — phase n of x, what's done, doing, next." },
    { k: "話", t: "Chat back mid-session", d: "Ask why it paused, steer the direction, and it picks the work back up." },
  ];
  return (
    <Shell id="relay" top bg="var(--paper-soft)" inner={{ maxWidth: 1100 }}>
      <div className="zs-eyebrow mb-4">Relay · away from keyboard</div>
      <h2 className="zs-display-lg mt-0 mb-4">Work continues while you're away.<br/>You stay the one who decides.</h2>
      <p className="zs-body mt-0 mb-7" style={{ maxWidth: 640 }}>
        Your machine holds a live line to your Dōjō — so from a phone or any browser you can reach a running
        session. No pairing, no separate app to install. Free on your own projects.
      </p>
      <div style={{ display: 'grid', gridTemplateColumns: bp.sm ? '1fr' : 'repeat(auto-fit, minmax(230px, 1fr))', gap: 'var(--space-3)' }}>
        {acts.map(a => (
          <div key={a.t} style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 'var(--radius-lg)' }} className="p-5">
            <span className="kanji" style={{ fontSize: 'var(--text-2xl)', color: 'var(--accent)' }}>{a.k}</span>
            <div className="zs-h3 mt-3 mb-1" style={{ fontWeight: 600 }}>{a.t}</div>
            <p className="zs-body-sm m-0" style={{ lineHeight: 1.6 }}>{a.d}</p>
          </div>
        ))}
      </div>
    </Shell>
  );
}

function PhilosophyB() {
  return (
    <Shell id="philosophy" top bottom bg="var(--paper-soft)" style={{ position: 'relative', overflow: 'hidden' }}
           inner={{ maxWidth: 760, textAlign: 'center', position: 'relative' }}>
      <div style={{ position: 'absolute', left: '50%', top: '50%', transform: 'translate(-50%, -50%)',
                     fontSize: 'var(--text-4xl)', lineHeight: 1, color: 'var(--accent-soft)', pointerEvents: 'none' }}
           className="kanji">静</div>
      <div style={{ position: 'relative' }}>
        <div className="zs-eyebrow mb-5">Sei · stillness</div>
        <h2 className="zs-display-lg mt-0 mb-6" style={{ lineHeight: 1.18 }}>
          The master observes for a long time before teaching.
        </h2>
        <p className="zs-body mt-0 mb-5" style={{ fontSize: 'var(--text-lg)', fontFamily: 'var(--font-display)', fontWeight: 300, lineHeight: 1.7 }}>
          AI tools are getting louder. More suggestions, more autocompletes, more interrupting. Sensei moves the
          other way. It speaks rarely, and only when it has something specific to say. Most days it is completely
          silent — and that is the feature.
        </p>
        <p className="zs-body m-0" style={{ lineHeight: 1.75 }}>
          The kanji throughout the app are not decoration. Each one names a phase of practice — observation,
          recognition, adoption, refinement. They are what we ask of the user, and what we ask of ourselves as
          the people who built this.
        </p>
      </div>
    </Shell>
  );
}

function PrivacyB() {
  const bp = useBP();
  const items = [
    { k: "蔵", title: "Local-first storage",
      text: "Transcripts, patterns, and memories live in a local database in ~/.sensei. Nothing leaves your machine without an explicit action you take." },
    { k: "鍵", title: "No telemetry",
      text: "We don't track usage. Sensei does make outbound calls you choose — your AI model, a Dōjō, Relay, library docs, the update check — and never otherwise." },
    { k: "破", title: "Easy to delete",
      text: "One folder. Delete it and sensei forgets everything." },
  ];
  return (
    <Shell id="privacy" bg="var(--paper)">
      <div style={{ display: 'grid', gridTemplateColumns: bp.md ? '1fr' : '1fr 1.5fr', alignItems: 'start' }} className="gap-8">
        <div>
          <span className="kanji" style={{ fontSize: 'var(--text-4xl)', color: 'var(--accent)' }}>蔵</span>
          <div className="zs-eyebrow mt-4 mb-4">Privacy &amp; local-first</div>
          <h2 className="zs-display-lg m-0" style={{ lineHeight: 1.15 }}>Your sessions stay on your machine.</h2>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-6">
          {items.map((it, i) => (
            <div key={i} style={{ display: 'grid', gridTemplateColumns: 'auto 1fr',
                       borderBottom: i < 2 ? 'var(--hairline)' : 'none' }} className="gap-5 pb-6">
              <span className="kanji" style={{ fontSize: 'var(--text-2xl)', color: 'var(--ink-soft)' }}>{it.k}</span>
              <div>
                <div className="zs-h2 mb-2">{it.title}</div>
                <div className="zs-body" style={{ lineHeight: 1.65 }}>{it.text}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </Shell>
  );
}

function PricingB() {
  return (
    <Shell id="pricing" top bottom bg="var(--paper-soft)" inner={{ maxWidth: 760, textAlign: 'center' }}>
      <div className="zs-eyebrow mb-4">Pricing</div>
      <h2 className="zs-display-lg mt-0 mb-5">Free during preview.<br/>Pay what feels right.</h2>
      <p className="zs-body m-0" style={{ fontSize: 'var(--text-lg)', fontFamily: 'var(--font-display)', fontWeight: 300, lineHeight: 1.65, maxWidth: 620, marginInline: 'auto' }}>
        The desktop app is free while sensei is in preview. If it earns a place in your daily practice, you can
        support development through sponsorship — that's what keeps the work independent.
      </p>
      <p className="zs-body-sm mt-4" style={{ color: 'var(--ink-mute)', maxWidth: 560, marginInline: 'auto' }}>
        A paid team tier for a private, shared Dōjō is on the <a href="#roadmap" style={{ color: 'var(--accent)' }}>roadmap</a> — not a live product yet.
      </p>
      <div className="mt-7" style={{ display: 'flex', gap: 'var(--space-3)', justifyContent: 'center', flexWrap: 'wrap' }}>
        <DownloadCTAB size="lg"/>
        <a href="https://github.com/sponsors/sensei-hq" target="_blank" rel="noopener" style={{
          display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2)', padding: 'var(--space-4) var(--space-6)',
          border: '1px solid var(--ink)', color: 'var(--ink)', borderRadius: 'var(--radius-lg)',
          fontSize: 'var(--text-base)', fontWeight: 500, textDecoration: 'none' }}>♥ Sponsor</a>
      </div>
    </Shell>
  );
}

// Labeled roadmap beat + waitlist capture. Reads window.SENSEI_ROADMAP
// (mirror of website/src/lib/features.ts). Honest by contract: every item
// status-badged; nothing implies availability. Static site — waitlist POSTs
// client-side to the Dōjō Worker (keys stay off the marketing site).
const WAITLIST_ENDPOINT = 'https://dojo.sensei-hq.com/v1/waitlist';
const STATUS_TONE = {
  live:  { color: 'var(--success)', bg: 'var(--success-soft)', border: 'var(--success-edge)' },
  soon:  { color: 'var(--accent)',  bg: 'var(--accent-soft)',  border: 'var(--accent-edge)' },
  later: { color: 'var(--ink-mute)', bg: 'var(--paper-mute)',  border: 'var(--paper-edge)' },
};

function StatusBadge({ status }) {
  const R = window.SENSEI_ROADMAP;
  const meta = (R && R.statusMeta[status]) || { label: status, tone: 'later' };
  const t = STATUS_TONE[meta.tone] || STATUS_TONE.later;
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 'var(--space-1)', flexShrink: 0,
      fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: t.color, background: t.bg,
      border: '1px solid ' + t.border, borderRadius: 'var(--radius-full)', padding: '2px 10px' }}>
      <span style={{ width: 5, height: 5, borderRadius: '50%', background: t.color }} />{meta.label}
    </span>
  );
}

function WaitlistForm({ interests }) {
  const [email, setEmail] = bS('');
  const [interest, setInterest] = bS(interests[0] ? interests[0].id : '');
  const [hp, setHp] = bS(''); // honeypot
  const [state, setState] = bS('idle'); // idle | sending | done | error
  const submit = async (e) => {
    e.preventDefault();
    if (hp) return; // bot
    if (!/.+@.+\..+/.test(email)) { setState('error'); return; }
    setState('sending');
    try {
      await fetch(WAITLIST_ENDPOINT, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, interest, source: 'sensei-site' }),
      });
      setState('done');
    } catch (err) { setState('done'); } // static site: fail open to a friendly confirm
  };
  if (state === 'done') {
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', background: 'var(--success-soft)',
        border: '1px solid var(--success-edge)', borderRadius: 'var(--radius-lg)', padding: 'var(--space-4) var(--space-5)' }}>
        <span className="kanji" style={{ color: 'var(--success)' }}>心</span>
        <span className="zs-body-sm" style={{ color: 'var(--ink)' }}>You're on the list. We'll write once — when it's ready, not before.</span>
      </div>
    );
  }
  return (
    <form onSubmit={submit} style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-2)' }}>
        <select value={interest} onChange={e => setInterest(e.target.value)} className="zs-input"
          style={{ width: 'auto', flex: '1 1 200px', cursor: 'pointer' }}>
          {interests.map(it => <option key={it.id} value={it.id}>{it.name}</option>)}
        </select>
        <input type="email" required value={email} onChange={e => setEmail(e.target.value)}
          placeholder="you@work.com" className="zs-input" style={{ flex: '2 1 220px', width: 'auto' }} />
        {/* honeypot — hidden from humans */}
        <input type="text" tabIndex={-1} autoComplete="off" value={hp} onChange={e => setHp(e.target.value)}
          aria-hidden="true" style={{ position: 'absolute', left: '-9999px', width: 1, height: 1, opacity: 0 }} />
        <button type="submit" style={{ flex: '0 0 auto', display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2)',
          padding: 'var(--space-2) var(--space-5)', background: 'var(--ink)', color: 'var(--paper)', border: 'none',
          borderRadius: 'var(--radius)', fontSize: 'var(--text-sm)', fontWeight: 500, cursor: 'pointer', fontFamily: 'inherit' }}>
          {state === 'sending' ? 'Sending…' : 'Request early access'}
        </button>
      </div>
      {state === 'error' && <div style={{ fontSize: 'var(--text-xs)', color: 'var(--danger)' }}>Enter a valid email address.</div>}
      <div style={{ fontSize: 'var(--text-xs)', color: 'var(--ink-mute)', lineHeight: 1.5 }}>
        One email per feature, only when it ships. No marketing list, no sharing — delete anytime.
      </div>
    </form>
  );
}

function RoadmapB() {
  const bp = useBP();
  const R = window.SENSEI_ROADMAP;
  if (!R) return null;
  // The roadmap beat = everything not yet shipped, grouped by phase in order.
  const roadmapPhases = R.phases.filter(p => p.id !== 'now');
  const byPhase = roadmapPhases
    .map(p => ({ phase: p, items: R.features.filter(f => f.phaseId === p.id && f.status !== 'shipped') }))
    .filter(g => g.items.length > 0);
  const waitlistItems = R.features.filter(f => f.waitlist);
  return (
    <Shell id="roadmap" top bottom bg="var(--paper)" inner={{ maxWidth: 1000 }}>
      <div className="zs-eyebrow mb-4">Roadmap · 道 the way ahead</div>
      <h2 className="zs-display-lg mt-0 mb-4">Built in the open,<br/>shipped when it's true.</h2>
      <p className="zs-body mt-0 mb-8" style={{ maxWidth: 620 }}>
        The local loop is available today. Everything below is labeled honestly — in progress, in beta, or
        planned. Nothing here is available yet; ask to be notified and we'll write the day it ships.
      </p>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-7)' }}>
        {byPhase.map(({ phase, items }) => (
          <div key={phase.id}>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 'var(--space-3)', flexWrap: 'wrap', borderBottom: 'var(--hairline)' }} className="pb-3 mb-4">
              <h3 className="zs-h2 m-0">{phase.label}</h3>
              <span className="zs-body-sm" style={{ color: 'var(--ink-mute)' }}>{phase.blurb}</span>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: bp.md ? '1fr' : 'repeat(2, 1fr)', gap: 'var(--space-3)' }}>
              {items.map(f => (
                <div key={f.id} style={{ background: 'var(--paper-soft)', border: 'var(--hairline)', borderRadius: 'var(--radius-lg)' }} className="p-5">
                  <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)' }} className="mb-2">
                    <span className="zs-h3 m-0" style={{ flex: 1 }}>{f.name}</span>
                    <StatusBadge status={f.status} />
                  </div>
                  <p className="zs-body-sm m-0" style={{ lineHeight: 1.6 }}>{f.blurb}</p>
                  <div className="zs-meta mt-3" style={{ color: 'var(--ink-faint)' }}>{R.surfaceMeta[f.surface].label}</div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>

      <div style={{ background: 'var(--paper-soft)', border: 'var(--hairline)', borderRadius: 'var(--radius-lg)' }} className="mt-8 p-6">
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 'var(--space-3)', flexWrap: 'wrap' }} className="mb-4">
          <span className="kanji" style={{ fontSize: 'var(--text-2xl)', color: 'var(--accent)' }}>待</span>
          <h3 className="zs-h2 m-0">Notify me when it's ready</h3>
        </div>
        <WaitlistForm interests={waitlistItems} />
      </div>
    </Shell>
  );
}

function FaqB() {
  const qs = [
    { q: "Which AI assistants does it observe?",
      a: "It works with Claude Code and Zed today, observing your sessions locally. More assistants as adapters land." },
    { q: "Does sensei see my code?",
      a: "Only what passes through your AI tool's session. It runs locally and stores everything in a local database in ~/.sensei you can inspect or delete at any time." },
    { q: "Will it slow down my machine?",
      a: "Sensei is a Tauri app — small binary, low memory. The observer is event-driven; it only does work when a session happens." },
    { q: "Can I export my memories?",
      a: "Export and import is on the roadmap — see above. Today your data lives in a local database in ~/.sensei you can inspect or copy directly." },
    { q: "Can I act on a session while away from my keyboard?",
      a: "Yes — Relay. Your daemon holds a live line to your Dōjō, so from a phone or any browser you can watch progress, approve the exact command, answer a decision, or chat back to a running session. No pairing, no separate app. Free on your own projects." },
    { q: "Which models does sensei use?",
      a: "Its own gateway ships with embedded Ollama — Gemma 4 by default, fast and local, no keys needed. Point it at a bigger Ollama host or bring your own API keys for Claude, GPT-4o and others. There are no tokens to mark up." },
    { q: "What's the long-term plan?",
      a: "The core promise — quiet, local, observant — never changes, and the app stays free. Teams and orgs pay for a private, shared Dōjō (per active contributor); public, open-source and personal Dōjōs are free forever." },
  ];
  return (
    <Shell id="faq" inner={{ maxWidth: 960 }}>
      <div className="zs-eyebrow mb-4">Frequently asked</div>
      <h2 className="zs-display-lg mt-0 mb-7">Common questions,<br/>plain answers.</h2>
      <div>
        {qs.map((it, i) => (
          <details key={i} style={{ borderTop: 'var(--hairline)', ...(i === qs.length - 1 ? { borderBottom: 'var(--hairline)' } : {}) }}
                   className="py-5 px-0">
            <summary style={{ cursor: 'pointer', listStyle: 'none', display: 'flex', justifyContent: 'space-between',
                       gap: 'var(--space-4)', fontSize: 'var(--text-lg)', color: 'var(--ink)', fontFamily: 'var(--font-display)', fontWeight: 400 }}>
              <span>{it.q}</span>
              <span className="kanji" style={{ color: 'var(--ink-mute)' }}>+</span>
            </summary>
            <div className="zs-body-sm mt-4" style={{ lineHeight: 1.7, maxWidth: 720 }}>{it.a}</div>
          </details>
        ))}
      </div>
    </Shell>
  );
}

function SupportB() {
  return (
    <Shell top bg="var(--paper-soft)" inner={{ maxWidth: 720, textAlign: 'center' }}>
      <span className="kanji" style={{ fontSize: 'var(--text-4xl)', color: 'var(--accent)' }}>志</span>
      <div className="zs-eyebrow mt-3 mb-3">Support development · shi</div>
      <h2 className="zs-h1 mt-0 mb-5" style={{ fontWeight: 300, lineHeight: 1.25 }}>
        If sensei has earned a place in your practice, you can help keep it growing.
      </h2>
      <p className="zs-body-sm mt-0 mb-6" style={{ lineHeight: 1.7 }}>
        Sensei is built by a small team. GitHub Sponsors keeps the work focused and independent.
      </p>
      <a href="https://github.com/sponsors/sensei-hq" target="_blank" rel="noopener" style={{
        display: 'inline-flex', alignItems: 'center', background: 'var(--accent)', color: 'var(--paper)',
        borderRadius: 'var(--radius-lg)', fontSize: 'var(--text-sm)', fontWeight: 500, textDecoration: 'none' }} className="gap-2 py-3 px-5">
        ♥ Sponsor on GitHub
      </a>
    </Shell>
  );
}

function FooterB() {
  const bp = useBP();
  return (
    <footer style={{ borderTop: 'var(--hairline)', color: 'var(--ink-mute)' }} className="py-7 px-7">
      <div style={{ maxWidth: 1200, margin: '0 auto', display: 'flex', flexDirection: bp.md ? 'column' : 'row',
                     alignItems: 'flex-start', justifyContent: 'space-between' }} className="gap-8">
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-3">
            <span style={{ display: 'inline-block', width: 22, height: 22, background: 'var(--accent)',
                           WebkitMaskImage: 'url(uploads/sensei.svg?v=3)', maskImage: 'url(uploads/sensei.svg?v=3)',
                           WebkitMaskSize: 'contain', maskSize: 'contain', WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
                           WebkitMaskPosition: 'center', maskPosition: 'center', alignSelf: 'center', flexShrink: 0 }} />
            <span className="display" style={{ fontSize: 'var(--text-base)', color: 'var(--ink-soft)' }}>Sensei</span>
          </div>
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--ink-mute)', maxWidth: 280, lineHeight: 1.6 }}>
            A patient observer for AI-assisted work. Built quietly, shipped slowly.
          </div>
          <div className="mono mt-3" style={{ fontSize: 'var(--text-xs)', color: 'var(--ink-faint)' }}>{(typeof window !== 'undefined' && window.__APP_VERSION__) || 'preview'}</div>
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-7">
          <FooterCol title="Product" links={["Download", "Pricing", "Privacy", "FAQ", "Changelog"]}/>
          <FooterCol title="Source" links={["GitHub", "Docs", "MCP", "Roadmap", "Issues"]}/>
          <FooterCol title="Connect" links={["Twitter", "Mastodon", "Email", "RSS"]}/>
        </div>
      </div>
    </footer>
  );
}

function FooterCol({ title, links }) {
  return (
    <div>
      <div className="zs-eyebrow mb-3" style={{ color: 'var(--ink-faint)' }}>{title}</div>
      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2">
        {links.map((l, i) => (
          <a key={i} href={`#${l.toLowerCase()}`} style={{ fontSize: 'var(--text-sm)', color: 'var(--ink-soft)' }}>{l}</a>
        ))}
      </div>
    </div>
  );
}

Object.assign(window, { VariantB });
