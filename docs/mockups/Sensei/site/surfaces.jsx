// ─── The five surfaces — editorial, not screenshots ────────────────
// Modelled on the For-Teams / Dōjō section: eyebrow → display headline →
// lead → varied card motifs. Screens and mocks go stale the moment the UI
// moves; a surface's GOAL ("show me the one thing worth my attention") and
// its FLOW ("noticed → scored → you promote it → it's applied") stay true
// release to release. So we describe what each surface is for, why it's
// shaped that way, and how the flow moves — no captures to keep in sync.
//
// Replaces the screenshot GalleryB. Exported as <Surfaces/>.

const SURF_eyebrow = { fontSize: 11, letterSpacing: '0.22em', color: 'var(--ink-3)', textTransform: 'uppercase' };
const SURF_sub     = { fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)', textTransform: 'uppercase', fontWeight: 600 };

// Shared block frame: identity column (number · kanji · name · role + the
// headline statement) on the left, the lead paragraph + "why this shape"
// note on the right, then the mechanic spanning full width beneath.
function SurfaceBlock({ n, kanji, name, role, headline, lead, why, children }) {
  return (
    <div className="pt-16 pb-16 border-t" >
      <div style={{ gridTemplateColumns: '1fr 1.25fr' }} className="gap-16 grid items-start" >
        {/* identity */}
        <div>
          <div className="gap-3 mb-4 flex items-baseline" >
            <span className="mono text-accent" style={{ fontSize: 12, letterSpacing: '0.06em' }}>{n}</span>
            <span className="kanji text-accent" style={{ fontSize: 34, lineHeight: 1 }}>{kanji}</span>
            <div className="gap-1 flex flex-col" >
              <span className="display font-normal" style={{ fontSize: 28, letterSpacing: '-0.015em', lineHeight: 1 }}>{name}</span>
              <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 9.5, letterSpacing: '0.16em' }}>{role}</span>
            </div>
          </div>
          <h3 className="display m-0 font-light text-ink" style={{ fontSize: 22, letterSpacing: '-0.01em', lineHeight: 1.25, maxWidth: 380 }}>
            {headline}
          </h3>
        </div>
        {/* rationale */}
        <div>
          <p style={{ fontSize: 15, lineHeight: 1.65, fontFamily: 'var(--font-display)' }} className="mt-0 mb-4 text-ink-2 font-light" >
            {lead}
          </p>
          <div style={{ gap: 10, borderLeft: '2px solid var(--accent-soft)' }} className="pl-3 flex items-start" >
            <span style={SURF_sub}>Why</span>
            <span className="text-ink-2 italic" style={{ fontSize: 12.5, lineHeight: 1.55 }}>{why}</span>
          </div>
        </div>
      </div>
      {/* mechanic */}
      <div className="mt-12" >{children}</div>
    </div>
  );
}

function SurfMechLabel({ children }) {
  return <div style={SURF_sub} className="mb-3" >{children}</div>;
}

// Horizontal flow of steps with → connectors (the Dōjō "loop" motif).
function SurfFlow({ steps }) {
  return (
    <div className="flex items-stretch gap-0" >
      {steps.map(([t, who, d], i) => (
        <React.Fragment key={t}>
          {i > 0 && <div className="flex items-center text-accent" style={{ padding: '0 6px', fontFamily: 'var(--font-mono)', fontSize: 13 }}>→</div>}
          <div style={{ borderRadius: 12, gap: 5 }} className="py-4 px-4 flex-1 bg-paper border border-paper-edge flex flex-col" >
            <span className="mono text-accent" style={{ fontSize: 10, letterSpacing: '0.06em' }}>{String(i + 1).padStart(2, '0')}</span>
            <span className="display font-medium text-ink" style={{ fontSize: 16, letterSpacing: '-0.01em' }}>{t}</span>
            <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 9.5, letterSpacing: '0.1em' }}>{who}</span>
            <span className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.5 }}>{d}</span>
          </div>
        </React.Fragment>
      ))}
    </div>
  );
}

// ─── 01 · Today ───────────────────────────────────────────────────
function SurfaceToday() {
  const anatomy = [
    ["聴", "The focal teaching", "P0", "The single most important pattern right now — phrased as a koan, with its projected impact and the sessions it came from."],
    ["繰", "Also worth noticing", "P1", "At most three secondary signals: a pattern recurring, a teaching adopted, a doc quietly drifting."],
    ["昇", "System has learned", "P2", "Rules sensei has already adopted on your behalf — each with its scope and the session it came from."],
    ["診", "First-Try-Right · 14d", "P2", "The one number that says whether you and your assistant are getting better together."],
  ];
  const ptone = { P0: 'var(--accent)', P1: 'oklch(0.55 0.13 60)', P2: 'var(--ink-3)' };
  return (
    <SurfaceBlock
      n="01" kanji="観" name="Today" role="家 · the morning brief"
      headline="One thing worth your attention. The rest stays silent."
      lead={<>Out of everything sensei watched overnight, Today elevates a <span className="text-ink" >single</span> focal observation and keeps the rest out of sight. Most mornings it is nearly empty — that emptiness is earned, and it is the point.</>}
      why="A dashboard of twelve widgets teaches you to ignore all twelve. One ranked item forces a decision and respects a morning.">
      <SurfMechLabel>What it surfaces · in priority order</SurfMechLabel>
      <div className="gap-2 flex flex-col" >
        {anatomy.map(([k, label, p, d]) => (
          <div key={label} style={{ gridTemplateColumns: 'auto 200px 1fr', borderRadius: 10 }} className="gap-4 py-3 px-4 grid items-baseline bg-paper border border-paper-edge" >
            <span className="kanji text-accent" style={{ fontSize: 18, width: 22 }}>{k}</span>
            <div className="flex items-baseline" style={{ gap: 8 }}>
              <span className="mono font-semibold" style={{ fontSize: 9, color: ptone[p] }}>{p}</span>
              <span className="display font-medium text-ink" style={{ fontSize: 15, letterSpacing: '-0.005em' }}>{label}</span>
            </div>
            <span className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.5 }}>{d}</span>
          </div>
        ))}
      </div>
    </SurfaceBlock>
  );
}

// ─── 02 · Sessions ────────────────────────────────────────────────
function SurfaceSessions() {
  const lanes = [
    { k: "良", title: "Going well", accent: 'var(--success)',
      d: "Compound refactors, consistent naming, clean test boundaries — the habits worth keeping." },
    { k: "破", title: "Not going well", accent: 'var(--accent)',
      d: "Tests skipped, useEffect overreach, PRs that sit too long — friction caught while it's small." },
    { k: "観", title: "Insights", accent: 'var(--ink-2)',
      d: "New idioms forming, recurring shapes in error handling — things sensei noticed but won't judge yet." },
  ];
  return (
    <SurfaceBlock
      n="02" kanji="録" name="Sessions" role="録 · the week in review"
      headline="The week, read back to you in three honest lanes."
      lead={<>Every session sensei witnessed, digested into a retrospective you didn't have to run. Not a log to scroll — a verdict, in plain language, with the trend underneath.</>}
      why="Charts you have to decode get ignored. Three lanes and a single trend line don't.">
      <SurfMechLabel>The retro · three lanes</SurfMechLabel>
      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3 mb-6 grid" >
        {lanes.map(l => (
          <div key={l.title} style={{ borderRadius: 10,
 borderTop: `2px solid ${l.accent}` }} className="py-4 px-4 bg-paper border border-paper-edge" >
            <div className="gap-2 mb-2 flex items-baseline" >
              <span className="kanji" style={{ fontSize: 16, color: l.accent }}>{l.k}</span>
              <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{l.title}</span>
            </div>
            <div className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.55 }}>{l.d}</div>
          </div>
        ))}
      </div>
      <SurfMechLabel>How a session is read</SurfMechLabel>
      <div className="gap-2 flex flex-wrap" >
        {[
          ["録", "Captured", "every turn, edit, test and correction"],
          ["診", "Scored", "first-try-right, or corrected — and how many times"],
          ["印", "Checkpointed", "marks where an adopted rule changed the trajectory"],
        ].map(([k, t, d]) => (
          <div key={t} style={{ flex: '1 1 220px', gap: 10, borderRadius: 10 }} className="py-3 px-4 flex items-start bg-paper border border-paper-edge" >
            <span className="kanji text-accent" style={{ fontSize: 15, lineHeight: 1.2 }}>{k}</span>
            <div>
              <div className="display font-medium text-ink" style={{ fontSize: 14 }}>{t}</div>
              <div style={{ fontSize: 11.5, lineHeight: 1.45 }} className="mt-1 text-ink-3" >{d}</div>
            </div>
          </div>
        ))}
      </div>
    </SurfaceBlock>
  );
}

// ─── 03 · Insights ────────────────────────────────────────────────
function SurfaceInsights() {
  return (
    <SurfaceBlock
      n="03" kanji="今" name="Insights" role="今 · candidates for triage"
      headline="Candidate patterns, scored — you decide which become memory."
      lead={<>The patterns sensei is tracking but hasn't adopted: each with a confidence, the number of projects it's appeared in, and how long it's been forming. Sensei proposes; you dispose.</>}
      why="A system that adopts on its own becomes a black box you have to fight. Keeping the promotion in your hands keeps the trust intact.">
      <SurfMechLabel>The flow · noticed to adopted</SurfMechLabel>
      <SurfFlow steps={[
        ["Noticed", "Sensei", "A shape repeats across sessions and gets logged as a candidate."],
        ["Scored", "Sensei", "Confidence rises with corroboration; spread and recency are tracked."],
        ["Reviewed", "You", "Surfaced here with its evidence. You read the provenance, not a verdict."],
        ["Promoted", "You", "A nod turns it into a memory — or you dismiss it and it stops asking."],
      ]}/>
    </SurfaceBlock>
  );
}

// ─── 04 · Memories ────────────────────────────────────────────────
function SurfaceMemories() {
  const anatomy = [
    ["When to apply", "The precise condition the lesson holds under — and when to lift it instead."],
    ["Examples watched", "The real sessions it was drawn from, dated and linked. Receipts, not assertions."],
    ["Provenance", "Seen across N sessions · first observed · confidence · adopted by whom."],
  ];
  const acts = [["採", "Adopt"], ["磨", "Refine"], ["捨", "Dismiss"]];
  return (
    <SurfaceBlock
      n="04" kanji="覚" name="Memories" role="覚 · adopted teachings"
      headline="Named, dated, and traceable to the sessions they came from."
      lead={<>An adopted memory is a small, named lesson sensei applies to future matching sessions — with your blessing. Every one is auditable down to the moments that formed it.</>}
      why="A learning system you can't inspect is a liability. Every memory shows its receipts, and every adoption is reversible.">
      <SurfMechLabel>Anatomy of a memory</SurfMechLabel>
      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3 mb-6 grid" >
        {anatomy.map(([t, d]) => (
          <div key={t} style={{ borderRadius: 10 }} className="py-4 px-4 bg-paper border border-paper-edge" >
            <div className="display font-medium text-ink" style={{ fontSize: 14.5, letterSpacing: '-0.005em' }}>{t}</div>
            <div style={{ fontSize: 11.5, lineHeight: 1.5 }} className="mt-2 text-ink-3" >{d}</div>
          </div>
        ))}
      </div>
      <SurfMechLabel>Always your call</SurfMechLabel>
      <div className="gap-2 flex flex-wrap" >
        {acts.map(([k, n]) => (
          <span key={n} style={{ gap: 7, fontSize: 12.5, borderRadius: 20 }} className="py-2 px-4 inline-flex items-center text-ink-2 bg-paper border border-paper-edge" >
            <span className="kanji text-accent" style={{ fontSize: 14 }}>{k}</span>{n}
          </span>
        ))}
        <span style={{ fontSize: 11.5 }} className="pl-2 inline-flex items-center text-ink-3 italic" >
          adopt, refine, or dismiss — nothing is permanent.
        </span>
      </div>
    </SurfaceBlock>
  );
}

// ─── 05 · Instruments ─────────────────────────────────────────────
function SurfaceInstruments() {
  const modes = [
    { k: "具", t: "Playground", d: "Run any MCP tool in isolation, with real arguments, and read the raw result. Sensei watches each call to learn which tools earn their place." },
    { k: "録", t: "Replay", d: "Step back through exactly what the assistant called during a session — the order, the inputs, the latency — so a surprise becomes legible." },
  ];
  return (
    <SurfaceBlock
      n="05" kanji="具" name="Instruments" role="具 · your tools, observed"
      headline="Your tools, observed — tried in isolation, replayed in full."
      lead={<>An assistant is only as good as the instruments it can reach. Instruments lets you exercise each MCP tool in isolation and replay exactly what was called during a session.</>}
      why="When a session goes wrong, the tool is as likely a culprit as the model. You can't trust what you can't watch.">
      <SurfMechLabel>Two ways to look · one toolset</SurfMechLabel>
      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3 grid" >
        {modes.map(m => (
          <div key={m.t} style={{ borderRadius: 10 }} className="py-6 px-4 bg-paper border border-paper-edge" >
            <div className="kanji text-accent" style={{ fontSize: 28, lineHeight: 1 }}>{m.k}</div>
            <div className="display mt-3 font-medium text-ink" style={{ fontSize: 16, letterSpacing: '-0.01em' }}>{m.t}</div>
            <div style={{ fontSize: 12, lineHeight: 1.55 }} className="mt-2 text-ink-2" >{m.d}</div>
          </div>
        ))}
      </div>
    </SurfaceBlock>
  );
}

// ─── Hero centerpiece — the product's voice, not a screenshot ──────
// What Today actually produces is a single teaching; the hero shows that
// artifact directly (chrome-less, clearly "an example morning"), plus a
// dimmed hint of what stays out of sight. No app window to keep in sync.
function HeroBrief() {
  const secondary = [
    ["繰", "Pattern recurring", "cache invalidation missed again", "3rd time", 'var(--warning)'],
    ["昇", "Teaching adopted", "canvas smoothing promoted to a rule", "+7% FTR", 'var(--success)'],
    ["探", "Drift detected", "brand-tokens README is 47 days old", "low", 'var(--ink-3)'],
  ];
  return (
    <div style={{ maxWidth: 920, borderRadius: 16,
 boxShadow: '0 30px 60px -28px rgba(20,18,14,0.18)'
 }} className="py-12 px-16 w-full bg-paper-2 border border-paper-edge" >
      {/* meta row */}
      <div className="mb-6 flex items-center justify-between" >
        <span className="mono text-ink-3" style={{ fontSize: 11, letterSpacing: '0.04em' }}>
          sensei speaks · 09:12
        </span>
        <span className="uppercase text-ink-4 font-semibold" style={{ fontSize: 10, letterSpacing: '0.16em' }}>
          an example morning
        </span>
      </div>

      {/* the focal teaching */}
      <div style={{ gridTemplateColumns: 'auto 1fr' }} className="gap-8 grid items-start" >
        <div className="kanji text-accent" style={{ fontSize: 72, lineHeight: 0.9 }}>聴</div>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 uppercase text-ink-3" >
            The one thing worth noticing
          </div>
          <div className="display font-light text-ink" style={{ fontSize: 32, letterSpacing: '-0.02em', lineHeight: 1.15 }}>
            The AI does not know your auth.
          </div>
          <p style={{ fontSize: 15, lineHeight: 1.6, maxWidth: 560 }} className="mt-3 mb-4 text-ink-2" >
            Three sessions corrected this week in <span className="mono" style={{ fontSize: 13 }}>lumen-auth</span> —
            all touched refresh or device flow. There's no integration-test persona for this module yet.
          </p>
          <div className="gap-4 flex items-center flex-wrap" >
            <span className="inline-flex items-center text-accent" style={{ gap: 7, fontSize: 13 }}>
              <span className="rounded-full bg-accent" style={{ width: 5, height: 5 }}/>
              Projected First-Try-Right +14% in Lumen Cloud
            </span>
            <span className="flex-1" />
            <span className="mono text-ink-4" style={{ fontSize: 11 }}>from s-2891 · s-2889 · s-2886</span>
          </div>
        </div>
      </div>

      {/* what stays out of sight */}
      <div className="mt-8 pt-6 border-t" >
        <div style={{ fontSize: 10, letterSpacing: '0.16em' }} className="mb-3 uppercase text-ink-4 font-semibold" >
          And quietly, today — kept out of the way
        </div>
        <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-3 grid" >
          {secondary.map(([k, label, text, tag, tone]) => (
            <div className="flex items-start" key={label} style={{ gap: 9, opacity: 0.78 }}>
              <span className="kanji" style={{ fontSize: 15, color: tone, lineHeight: 1.2 }}>{k}</span>
              <div className="min-w-0" >
                <div className="uppercase text-ink-3" style={{ fontSize: 10.5, letterSpacing: '0.1em' }}>{label}</div>
                <div style={{ fontSize: 12, lineHeight: 1.4 }} className="mt-1 text-ink-2" >{text}</div>
                <span className="mono" style={{ fontSize: 10, color: tone }}>{tag}</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function Surfaces() {
  return (
    <section id="gallery" className="pt-24 pb-16 px-12" >
      <div style={{ maxWidth: 1200 }} className="mx-auto" >
        <div style={SURF_eyebrow} className="mb-4" >The screens · 面</div>
        <h2 className="display mt-0 mb-4 font-light" style={{ fontSize: 56, letterSpacing: '-0.025em', lineHeight: 1.05 }}>
          Five surfaces,<br/>one rhythm.
        </h2>
        <p style={{ fontSize: 17, maxWidth: 640, lineHeight: 1.6,
 fontFamily: 'var(--font-display)' }} className="mt-0 mb-2 text-ink-2 font-light" >
          Each surface answers one question and stays quiet otherwise. The pixels will keep
          changing; what they're <em>for</em> won't. So here is what each one does, why it's
          shaped that way, and how its flow moves.
        </p>
        <div className="mt-8" >
          <SurfaceToday/>
          <SurfaceSessions/>
          <SurfaceInsights/>
          <SurfaceMemories/>
          <SurfaceInstruments/>
        </div>
      </div>
    </section>
  );
}

Object.assign(window, { Surfaces, HeroBrief });
