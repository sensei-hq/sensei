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
      padding: bp.sm ? 'var(--space-12) var(--space-6)' : 'var(--space-24) var(--space-12)',
      ...style,
    }}>
      <div className="mx-auto" style={{ maxWidth: 1200, ...inner }}>{children}</div>
    </section>
  );
}

function VariantB() {
  return (
    <div className="sensei variant-b bg-paper text-ink min-h-full" style={{ fontFamily: 'var(--font-ui)'
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
    <nav className="sticky bg-paper border-b" style={{ top: 0, zIndex: 40 }}>
      <div style={{ maxWidth: 1200 }}
 className="py-4 px-12 mx-auto flex items-center justify-between">
        <div className="gap-2 flex items-baseline">
          <span className="inline-block bg-accent self-center shrink-0" style={{ width: 26, height: 26,
 WebkitMaskImage: 'url(uploads/sensei.svg?v=3)', maskImage: 'url(uploads/sensei.svg?v=3)',
 WebkitMaskSize: 'contain', maskSize: 'contain', WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
 WebkitMaskPosition: 'center', maskPosition: 'center' }} />
          <span className="display text-xl text-ink" style={{ letterSpacing: '-0.01em' }}>Sensei</span>
        </div>

        {!bp.md && (
          <div className="gap-8 flex items-center">
            {NAV_LINKS.map(([href, label]) => (
              <a className="text-sm text-ink-soft no-underline" key={href} href={href}
 style={{ transition: 'color .15s' }}
 onMouseEnter={(e) => e.currentTarget.style.color = 'var(--ink)'}
 onMouseLeave={(e) => e.currentTarget.style.color = 'var(--ink-soft)'}>
                {label}
              </a>
            ))}
            <DownloadCTAB size="sm"/>
          </div>
        )}

        {bp.md && (
          <button className="inline-flex flex-col border-0 cursor-pointer" onClick={() => setOpen(o => !o)} aria-label="Menu"
 style={{ gap: 4, background: 'none', padding: 6 }}>
            {[0, 1, 2].map(i => <span className="bg-ink" key={i} style={{ width: 22, height: 2, borderRadius: 2 }} />)}
          </button>
        )}
      </div>

      {bp.md && open && (
        <div className="px-12 py-6 border-t bg-paper">
          <div className="gap-4 flex flex-col">
            {NAV_LINKS.map(([href, label]) => (
              <a className="text-lg text-ink-soft no-underline" key={href} href={href} onClick={() => setOpen(false)}
 >{label}</a>
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
    <Shell top={false} bottom={false} style={{ position: 'relative', paddingBottom: 'var(--space-16)' }}>
      {!bp.md && (
        <div style={{ right: 56, top: 24, lineHeight: 1, pointerEvents: 'none' }} className="kanji absolute text-4xl text-accent-soft">観</div>
      )}
      <div className="relative" >
        <div className="gap-3 mb-6 flex items-baseline">
          <span className="ink-dot bg-accent" style={{ width: 8, height: 8 }}/>
          <div className="zs-eyebrow">Sensei · the patient observer</div>
        </div>
        <h1 className="display m-0 font-light" style={{
 fontSize: bp.sm ? 'var(--text-3xl)' : 'var(--text-4xl)', lineHeight: 1.02,
 letterSpacing: '-0.03em', maxWidth: 920 }}>
          A quiet companion<br/>
          for AI-assisted <em className="text-accent not-italic" >work</em>.
        </h1>
        <p style={{ fontSize: bp.sm ? 'var(--text-lg)' : 'var(--text-xl)',
 lineHeight: 1.55, maxWidth: 640, fontFamily: 'var(--font-display)' }} className="mt-8 text-ink-soft font-light">
          Sensei watches your sessions with AI assistants — then surfaces the patterns you're too close to
          see. Not a chatbot. Not a copilot. A patient observer.
        </p>
        <div className="gap-4 mt-8 flex items-center flex-wrap">
          <DownloadCTAB size="lg"/>
          <a className="text-sm text-ink-soft" href="#how" >See how it works ↓</a>
        </div>
        <div className="zs-meta mt-4">Free · Local-first · No account required</div>
      </div>
      <div className="mt-16 flex justify-center relative">
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
  const px = size === "lg" ? 'var(--space-4) var(--space-8)' : 'var(--space-2) var(--space-4)';
  const fs = size === "lg" ? 'var(--text-base)' : 'var(--text-sm)';
  return (
    <a href={`#download-${os.toLowerCase()}`}
 style={{ padding: px, fontSize: fs }} className="gap-3 inline-flex items-center bg-ink text-paper rounded-lg font-medium no-underline shadow-cta">
      <span className="kanji text-accent" >下</span>
      Download for {os}
    </a>
  );
}

// Trust strip — what it speaks / works with (dev-tool social proof). B5.
function TrustStripB() {
  const items = ['Speaks MCP', 'Claude', 'GPT-4o', 'Gemma', 'Ollama', 'Tauri · <60MB'];
  return (
    <section className="py-4 px-12 border-b bg-paper">
      <div style={{ maxWidth: 1200 }} className="gap-2 mx-auto flex flex-wrap items-center justify-center">
        <span className="zs-eyebrow mr-2" >Works with</span>
        {items.map((t, i) => (
          <React.Fragment key={t}>
            {i > 0 && <span className="text-ink-faint" >·</span>}
            <span className="mono text-sm text-ink-soft" >{t}</span>
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
    <section 
 className="py-8 px-12 border-t border-b bg-paper-soft">
      <div style={{ maxWidth: 1200,
 gridTemplateColumns: bp.sm ? '1fr' : bp.md ? 'repeat(2, 1fr)' : 'repeat(4, 1fr)' }} className="gap-8 mx-auto grid">
        {stats.map((s, i) => (
          <div className="text-center" key={i} >
            <div className="display text-2xl font-normal text-ink" >{s.v}</div>
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
      <div style={{ gridTemplateColumns: bp.md ? '1fr' : '1fr 1.6fr' }} className="gap-16 grid items-start">
        <div>
          <div className="zs-eyebrow mb-4">What it is</div>
          <h2 className="zs-display-lg m-0">One desktop app.<br/>One quiet promise.</h2>
        </div>
        <div className="font-light" style={{ fontFamily: 'var(--font-display)' }}>
          <p className="zs-body mt-1 text-lg" style={{ lineHeight: 1.65 }}>
            Sensei runs on your machine and observes your sessions with AI assistants. It sends no telemetry;
            it speaks rarely; it remembers what you've actually done.
          </p>
          <p className="zs-body text-lg" style={{ lineHeight: 1.65 }}>
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
      <h2 className="zs-display-lg mt-0 mb-16">
        <span className="text-accent" >観 · 察 · 覚</span><br/>Watch, notice, adopt.
      </h2>
      <div style={{ gridTemplateColumns: bp.md ? '1fr' : 'repeat(3, 1fr)' }} className="gap-16 grid">
        {steps.map((s, i) => (
          <div key={i} 
 className="py-8 px-6 bg-paper border border-paper-edge rounded-lg">
            <div className="kanji mb-4 text-4xl text-accent" style={{ lineHeight: 1 }}>{s.kanji}</div>
            <div className="zs-eyebrow mb-2">{s.phase}</div>
            <h3 className="zs-h2 mt-0 mb-4">{s.title}</h3>
            <div className="zs-body-sm mb-4" style={{ lineHeight: 1.65 }}>{s.text}</div>
            <div 
 className="pt-3 text-xs text-ink-mute italic border-t">{s.sub}</div>
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
      <div className="flex items-center justify-between gap-3 bg-paper border border-paper-edge rounded py-3 px-4" >
        <code className="mono text-sm text-ink overflow-x-auto whitespace-nowrap" >
          <span className="text-ink-faint" >$ </span>{cmd}
        </code>
        <button className="shrink-0 border-0 cursor-pointer text-xs" onClick={copy} style={{ background: 'none', color: copied ? 'var(--success)' : 'var(--accent)', fontFamily: 'var(--font-mono)' }}>
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
      <div style={{ gridTemplateColumns: bp.md ? '1fr' : '1fr 1.3fr' }} className="gap-16 grid items-start">
        <div>
          <div className="zs-eyebrow mb-4">Quickstart</div>
          <h2 className="zs-display-lg m-0">Up and observing<br/>in one command.</h2>
          <p className="zs-body mt-6" style={{ maxWidth: 380 }}>
            Install the desktop app, or drop the daemon into an existing setup. No account, no config —
            it starts listening the moment it's open.
          </p>
          <a href="#docs" className="mt-6 inline-flex items-center gap-2 text-sm text-accent no-underline">
            Read the docs →
          </a>
        </div>
        <div className="gap-4 flex flex-col">
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
      <p className="zs-body mt-0 mb-12" style={{ maxWidth: 640 }}>
        Your machine holds a live line to your Dōjō — so from a phone or any browser you can reach a running
        session. No pairing, no separate app to install. Free on your own projects.
      </p>
      <div className="grid gap-3" style={{ gridTemplateColumns: bp.sm ? '1fr' : 'repeat(auto-fit, minmax(230px, 1fr))' }}>
        {acts.map(a => (
          <div key={a.t} className="p-6 bg-paper border border-paper-edge rounded-lg">
            <span className="kanji text-2xl text-accent" >{a.k}</span>
            <div className="zs-h3 mt-3 mb-1 font-semibold" >{a.t}</div>
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
      <div style={{ left: '50%', top: '50%', transform: 'translate(-50%, -50%)', lineHeight: 1, pointerEvents: 'none' }}
 className="kanji absolute text-4xl text-accent-soft">静</div>
      <div className="relative" >
        <div className="zs-eyebrow mb-6">Sei · stillness</div>
        <h2 className="zs-display-lg mt-0 mb-8" style={{ lineHeight: 1.18 }}>
          The master observes for a long time before teaching.
        </h2>
        <p className="zs-body mt-0 mb-6 text-lg font-light" style={{ fontFamily: 'var(--font-display)', lineHeight: 1.7 }}>
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
      <div style={{ gridTemplateColumns: bp.md ? '1fr' : '1fr 1.5fr' }} className="gap-16 grid items-start">
        <div>
          <span className="kanji text-4xl text-accent" >蔵</span>
          <div className="zs-eyebrow mt-4 mb-4">Privacy &amp; local-first</div>
          <h2 className="zs-display-lg m-0" style={{ lineHeight: 1.15 }}>Your sessions stay on your machine.</h2>
        </div>
        <div className="gap-8 flex flex-col">
          {items.map((it, i) => (
            <div key={i} style={{ gridTemplateColumns: 'auto 1fr',
 borderBottom: i < 2 ? 'var(--hairline)' : 'none' }} className="gap-6 pb-8 grid">
              <span className="kanji text-2xl text-ink-soft" >{it.k}</span>
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
      <h2 className="zs-display-lg mt-0 mb-6">Free during preview.<br/>Pay what feels right.</h2>
      <p className="zs-body m-0 text-lg font-light" style={{ fontFamily: 'var(--font-display)', lineHeight: 1.65, maxWidth: 620, marginInline: 'auto' }}>
        The desktop app is free while sensei is in preview. If it earns a place in your daily practice, you can
        support development through sponsorship — that's what keeps the work independent.
      </p>
      <p className="zs-body-sm mt-4 text-ink-mute" style={{ maxWidth: 560, marginInline: 'auto' }}>
        A paid team tier for a private, shared Dōjō is on the <a className="text-accent" href="#roadmap" >roadmap</a> — not a live product yet.
      </p>
      <div className="mt-12 flex gap-3 justify-center flex-wrap" >
        <DownloadCTAB size="lg"/>
        <a className="inline-flex items-center gap-2 py-4 px-8 text-ink rounded-lg text-base font-medium no-underline" href="https://github.com/sponsors/sensei-hq" target="_blank" rel="noopener" style={{
 border: '1px solid var(--ink)' }}>♥ Sponsor</a>
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
    <span className="inline-flex items-center gap-1 shrink-0 text-xs rounded-full" style={{
 fontFamily: 'var(--font-mono)', color: t.color, background: t.bg,
 border: '1px solid ' + t.border, padding: '2px 10px' }}>
      <span className="rounded-full" style={{ width: 5, height: 5, background: t.color }} />{meta.label}
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
      <div className="flex items-center gap-2 bg-success-soft rounded-lg py-4 px-6" style={{
 border: '1px solid var(--success-edge)' }}>
        <span className="kanji text-success" >心</span>
        <span className="zs-body-sm text-ink" >You're on the list. We'll write once — when it's ready, not before.</span>
      </div>
    );
  }
  return (
    <form className="flex flex-col gap-3" onSubmit={submit} >
      <div className="flex flex-wrap gap-2" >
        <select value={interest} onChange={e => setInterest(e.target.value)} className="zs-input w-auto cursor-pointer"
 style={{ flex: '1 1 200px' }}>
          {interests.map(it => <option key={it.id} value={it.id}>{it.name}</option>)}
        </select>
        <input type="email" required value={email} onChange={e => setEmail(e.target.value)}
 placeholder="you@work.com" className="zs-input w-auto" style={{ flex: '2 1 220px' }} />
        {/* honeypot — hidden from humans */}
        <input className="absolute" type="text" tabIndex={-1} autoComplete="off" value={hp} onChange={e => setHp(e.target.value)}
 aria-hidden="true" style={{ left: '-9999px', width: 1, height: 1, opacity: 0 }} />
        <button className="inline-flex items-center gap-2 py-2 px-6 bg-ink text-paper border-0 rounded text-sm font-medium cursor-pointer" type="submit" style={{ flex: '0 0 auto', fontFamily: 'inherit' }}>
          {state === 'sending' ? 'Sending…' : 'Request early access'}
        </button>
      </div>
      {state === 'error' && <div className="text-xs text-danger" >Enter a valid email address.</div>}
      <div className="text-xs text-ink-mute" style={{ lineHeight: 1.5 }}>
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
      <p className="zs-body mt-0 mb-16" style={{ maxWidth: 620 }}>
        The local loop is available today. Everything below is labeled honestly — in progress, in beta, or
        planned. Nothing here is available yet; ask to be notified and we'll write the day it ships.
      </p>

      <div className="flex flex-col gap-12" >
        {byPhase.map(({ phase, items }) => (
          <div key={phase.id}>
            <div className="pb-3 mb-4 flex items-baseline gap-3 flex-wrap border-b">
              <h3 className="zs-h2 m-0">{phase.label}</h3>
              <span className="zs-body-sm text-ink-mute" >{phase.blurb}</span>
            </div>
            <div className="grid gap-3" style={{ gridTemplateColumns: bp.md ? '1fr' : 'repeat(2, 1fr)' }}>
              {items.map(f => (
                <div key={f.id} className="p-6 bg-paper-soft border border-paper-edge rounded-lg">
                  <div className="mb-2 flex items-center gap-3">
                    <span className="zs-h3 m-0 flex-1" >{f.name}</span>
                    <StatusBadge status={f.status} />
                  </div>
                  <p className="zs-body-sm m-0" style={{ lineHeight: 1.6 }}>{f.blurb}</p>
                  <div className="zs-meta mt-3 text-ink-faint" >{R.surfaceMeta[f.surface].label}</div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>

      <div className="mt-16 p-8 bg-paper-soft border border-paper-edge rounded-lg">
        <div className="mb-4 flex items-baseline gap-3 flex-wrap">
          <span className="kanji text-2xl text-accent" >待</span>
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
      <h2 className="zs-display-lg mt-0 mb-12">Common questions,<br/>plain answers.</h2>
      <div>
        {qs.map((it, i) => (
          <details key={i} style={{ ...(i === qs.length - 1 ? { borderBottom: 'var(--hairline)' } : {}) }}
 className="py-6 px-0 border-t">
            <summary className="cursor-pointer flex justify-between gap-4 text-lg text-ink font-normal" style={{ listStyle: 'none', fontFamily: 'var(--font-display)' }}>
              <span>{it.q}</span>
              <span className="kanji text-ink-mute" >+</span>
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
      <span className="kanji text-4xl text-accent" >志</span>
      <div className="zs-eyebrow mt-3 mb-3">Support development · shi</div>
      <h2 className="zs-h1 mt-0 mb-6 font-light" style={{ lineHeight: 1.25 }}>
        If sensei has earned a place in your practice, you can help keep it growing.
      </h2>
      <p className="zs-body-sm mt-0 mb-8" style={{ lineHeight: 1.7 }}>
        Sensei is built by a small team. GitHub Sponsors keeps the work focused and independent.
      </p>
      <a href="https://github.com/sponsors/sensei-hq" target="_blank" rel="noopener" className="gap-2 py-3 px-6 inline-flex items-center bg-accent text-paper rounded-lg text-sm font-medium no-underline">
        ♥ Sponsor on GitHub
      </a>
    </Shell>
  );
}

function FooterB() {
  const bp = useBP();
  return (
    <footer className="py-12 px-12 border-t text-ink-mute">
      <div style={{ maxWidth: 1200, flexDirection: bp.md ? 'column' : 'row' }} className="gap-16 mx-auto flex items-start justify-between">
        <div>
          <div className="gap-2 mb-3 flex items-baseline">
            <span className="inline-block bg-accent self-center shrink-0" style={{ width: 22, height: 22,
 WebkitMaskImage: 'url(uploads/sensei.svg?v=3)', maskImage: 'url(uploads/sensei.svg?v=3)',
 WebkitMaskSize: 'contain', maskSize: 'contain', WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
 WebkitMaskPosition: 'center', maskPosition: 'center' }} />
            <span className="display text-base text-ink-soft" >Sensei</span>
          </div>
          <div className="text-xs text-ink-mute" style={{ maxWidth: 280, lineHeight: 1.6 }}>
            A patient observer for AI-assisted work. Built quietly, shipped slowly.
          </div>
          <div className="mono mt-3 text-xs text-ink-faint" >{(typeof window !== 'undefined' && window.__APP_VERSION__) || 'preview'}</div>
        </div>
        <div className="gap-12 flex flex-wrap">
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
      <div className="zs-eyebrow mb-3 text-ink-faint" >{title}</div>
      <div className="gap-2 flex flex-col">
        {links.map((l, i) => (
          <a className="text-sm text-ink-soft" key={i} href={`#${l.toLowerCase()}`} >{l}</a>
        ))}
      </div>
    </div>
  );
}

Object.assign(window, { VariantB });
