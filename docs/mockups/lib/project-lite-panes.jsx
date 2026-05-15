// Simplified, in-context preview panes for the Project window's sidebar.
// Inspired by the "Project window standalone" — each pane is a calm summary
// with one or two highlighted insights and a single primary action.
// These replace the "preview shown elsewhere" placeholder.

const { useState: pLS } = React;

// ───────────────────────────────────────────────────────────
//  Shared bits
// ───────────────────────────────────────────────────────────

function PaneHeader({ kanji, eyebrow, title, accent = "var(--accent)", right }) {
  return (
    <div style={{ display: 'flex', alignItems: 'flex-end', gap: 16, marginBottom: 24 }}>
      <span className="kanji" style={{ fontSize: 56, color: accent, lineHeight: 1 }}>{kanji}</span>
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>
          {eyebrow}
        </div>
        <h1 className="display" style={{ fontSize: 28, fontWeight: 400, margin: 0,
                      letterSpacing: '-0.01em' }}>
          {title}
        </h1>
      </div>
      {right}
    </div>
  );
}

function HeroCard({ kanji, eyebrow, headline, body, action, meta, tone = "var(--accent)" }) {
  return (
    <div style={{
      padding: '24px 24px', marginBottom: 24,
      background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 10,
      display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 24
    }}>
      <div className="kanji" style={{ fontSize: 56, color: tone, lineHeight: 1 }}>{kanji}</div>
      <div>
        <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>
          {eyebrow}
        </div>
        <div className="display" style={{ fontSize: 22, fontWeight: 400,
                      letterSpacing: '-0.01em', lineHeight: 1.3, marginBottom: 8,
                      color: 'var(--ink)' }}>
          {headline}
        </div>
        <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.65, margin: 0 }}>
          {body}
        </p>
        {(action || meta) && (
          <div style={{ display: 'flex', gap: 12, alignItems: 'center', marginTop: 12 }}>
            {action && (
              <button style={{
                padding: '8px 12px', fontSize: 13, background: 'var(--ink)',
                color: 'var(--paper)', borderRadius: 5, border: 'none', cursor: 'pointer'
              }}>{action} →</button>
            )}
            {meta && (
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {meta}
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function StatBlock({ label, value, sub, tone = "var(--ink)" }) {
  return (
    <div style={{ padding: '12px 16px', background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 8 }}>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase', marginBottom: 4 }}>
        {label}
      </div>
      <div className="display" style={{ fontSize: 28, fontWeight: 400, color: tone, lineHeight: 1 }}>
        {value}
      </div>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>{sub}</div>
    </div>
  );
}

function SimpleRow({ left, right, leftSub, rightTone = 'var(--ink-3)' }) {
  return (
    <div style={{
      display: 'grid', gridTemplateColumns: '1fr auto', gap: 12, alignItems: 'baseline',
      padding: '12px 4px', borderBottom: 'var(--hairline)'
    }}>
      <div>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>{left}</div>
        {leftSub && (
          <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
            {leftSub}
          </div>
        )}
      </div>
      <div className="mono" style={{ fontSize: 11, color: rightTone }}>{right}</div>
    </div>
  );
}

function MiniHeading({ kanji, label, right }) {
  return (
    <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                  marginBottom: 12, marginTop: 0 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>{kanji}</span>
        <h2 className="display" style={{ fontSize: 13, fontWeight: 400, margin: 0,
                      letterSpacing: '0.01em', color: 'var(--ink-2)',
                      textTransform: 'uppercase' }}>
          {label}
        </h2>
      </div>
      {right}
    </div>
  );
}

const PADDING = '32px 40px 48px';

// ───────────────────────────────────────────────────────────
//  OVERVIEW · the calm summary the user liked
// ───────────────────────────────────────────────────────────
function ProjOverviewLite({ project, openAction }) {
  const D = window.PROJECT_DATA;
  const topRec = D.recommendations[0];

  return (
    <div style={{ padding: PADDING }}>
      <PaneHeader
        kanji={project.kanji} eyebrow={`Project · ${project.client || "internal"}`}
        title={project.name}
        right={
          <div style={{ textAlign: 'right' }}>
            <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                           textTransform: 'uppercase' }}>FTR · 14d</div>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 4,
                           justifyContent: 'flex-end', marginTop: 4 }}>
              <span className="display"
                     style={{ fontSize: 28, fontWeight: 400, lineHeight: 1,
                               color: project.warn ? 'var(--warning)' : 'var(--ink)' }}>
                {Math.round((project.ftr || 0.78) * 100)}
              </span>
              <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>%</span>
            </div>
          </div>
        }/>

      <HeroCard
        kanji="聴" eyebrow="This project · sensei speaks"
        headline={topRec ? topRec.title : "All quiet — no urgent recommendations."}
        body={topRec ? topRec.why : "Sensei is observing. The next correction or pattern will surface here."}
        action={topRec ? `send to ${topRec.defaultAcp}` : null}
        meta={topRec ? topRec.evidence.join(" · ") : null}
        tone="var(--accent)"/>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 16, marginBottom: 24 }}>
        <StatBlock label="Sessions · 7d" value={project.sessions7d || 28}
                   sub={`${(D.recentSessions || []).filter(s => !s.ftr).length} corrected`}/>
        <StatBlock label="Memories" value="11" sub="2 to share · 1 to merge"/>
        <StatBlock label="Doc drift" value="3" sub="of 18 referenced docs"
                   tone="var(--warning)"/>
      </div>

      <div>
        <MiniHeading kanji="今" label="Recent in this project"/>
        <div>
          {(D.recentSessions || []).slice(0, 4).map(s => (
            <SimpleRow key={s.id}
              left={s.title}
              leftSub={`${s.id} · ${s.duration} · ${s.corrections > 0 ? s.corrections + " corrections" : "first-try right"}`}
              right={s.time}
              rightTone={s.ftr ? 'var(--success)' : 'var(--ink-3)'}/>
          ))}
        </div>
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
//  MEMORIES · what sensei has learned working here
// ───────────────────────────────────────────────────────────
function ProjMemoriesLite({ project }) {
  const memories = [
    { kanji: "覚", title: "Auth handlers always return ApiError, never throw",
      kind: "rule", places: 12, source: "extracted from s-2891 · s-2889",
      status: "active" },
    { kanji: "覚", title: "CRDT ops must emit a Delta and pass commutativity prop-test",
      kind: "rule", places: 14, source: "promoted from pattern p2",
      status: "active" },
    { kanji: "覚", title: "Refresh tokens rotate on every use",
      kind: "decision", places: 3, source: "s-2891 retro",
      status: "active" },
    { kanji: "覚", title: "Use openapi-generator-ts, not openapi-typescript",
      kind: "preference", places: 4, source: "decided in s-2848",
      status: "active" },
    { kanji: "送", title: "“Auth handlers return ApiError” — generalised",
      kind: "ready to share", places: 1, source: "next batch · in 2 days",
      status: "share" },
    { kanji: "送", title: "“CRDT commutativity prop-test” — generalised",
      kind: "ready to share", places: 1, source: "next batch · in 2 days",
      status: "share" }
  ];

  return (
    <div style={{ padding: PADDING }}>
      <PaneHeader kanji="覚" eyebrow="This project · 11 memories" title="Memories"/>

      <HeroCard
        kanji="送" eyebrow="2 ready to share with the collective"
        headline="Two project memories generalised cleanly."
        body="Sensei has rewritten them stack-agnostic — your auth-handler convention and the CRDT commutativity rule. Both are queued for the next sharing batch in 2 days."
        action="review next batch"
        meta="reviewed batches · 47 / 47"
        tone="var(--success)"/>

      <div>
        <MiniHeading kanji="覚" label="Active memories"
          right={<span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            9 active · 2 sharing
          </span>}/>
        {memories.map((m, i) => (
          <div key={i} style={{
            padding: '12px 12px', marginBottom: 8,
            background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 6,
            borderLeft: m.status === 'share' ? '2px solid var(--success)' : '2px solid transparent',
            display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 12, alignItems: 'start'
          }}>
            <span className="kanji" style={{ fontSize: 15,
                  color: m.status === 'share' ? 'var(--success)' : 'var(--accent)' }}>{m.kanji}</span>
            <div>
              <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.4 }}>{m.title}</div>
              <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
                {m.kind} · {m.source}
              </div>
            </div>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {m.places}× used
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
//  TRACEABILITY · doc ↔ symbol drift
// ───────────────────────────────────────────────────────────
function ProjTraceabilityLite({ project }) {
  const drift = [
    { doc: "docs/auth.md § Refresh flow",
      symbol: "lumen-auth/src/refresh.rs::rotate_token",
      lastSync: "21d ago", severity: "high",
      note: "Doc says “tokens rotate every 7d”; code rotates every use since s-2891." },
    { doc: "docs/sync.md § Delta encoding",
      symbol: "lumen-sync/src/delta.rs::encode",
      lastSync: "9d ago", severity: "medium",
      note: "New Delta variant Tombstone added; doc enumerates only 4 variants." },
    { doc: "docs/api.md § Error contract",
      symbol: "lumen-api/src/error.ts::ApiError",
      lastSync: "broken", severity: "high",
      note: "Linked symbol no longer exists — renamed to LumenError." }
  ];

  return (
    <div style={{ padding: PADDING }}>
      <PaneHeader kanji="巻" eyebrow="This project · doc ↔ code drift" title="Traceability"/>

      <HeroCard
        kanji="繕" eyebrow="3 docs drifted · 1 broken"
        headline="docs/auth.md § Refresh flow has drifted from the code."
        body="The doc still says tokens rotate every 7d, but rotate_token has been changing on every use since s-2891. Sensei has a fix-drift prompt ready — it will update the doc and add a regression note."
        action="run fix-drift prompt"
        meta="checked nightly · 18 docs tracked"
        tone="var(--warning)"/>

      <div>
        <MiniHeading kanji="巻" label="Drifted documents"/>
        {drift.map((d, i) => (
          <div key={i} style={{
            padding: '12px 12px', marginBottom: 8,
            background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 6,
            borderLeft: `2px solid ${d.severity === 'high' ? 'var(--accent)' : 'var(--warning)'}`
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between',
                           alignItems: 'baseline', marginBottom: 4 }}>
              <div style={{ fontSize: 13, color: 'var(--ink)' }}>{d.doc}</div>
              <span className="mono" style={{ fontSize: 11,
                    color: d.lastSync === 'broken' ? 'var(--accent)' : 'var(--ink-3)' }}>
                {d.lastSync === 'broken' ? '⚠ broken link' : 'synced ' + d.lastSync}
              </span>
            </div>
            <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)', marginBottom: 4 }}>
              ↪ {d.symbol}
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.5 }}>
              {d.note}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
//  LIBRARIES · what this project uses
// ───────────────────────────────────────────────────────────
function ProjLibrariesLite({ project }) {
  const libs = [
    { kanji: "庫", name: "openapi-3", source: "detected · package.json", uses: 14, hasInstr: true },
    { kanji: "庫", name: "stripe-js", source: "detected · package.json", uses: 6, hasInstr: true },
    { kanji: "庫", name: "postgres", source: "detected · Cargo.toml",   uses: 22, hasInstr: true },
    { kanji: "庫", name: "tailwindcss", source: "detected · package.json", uses: 8, hasInstr: false },
    { kanji: "入", name: "lumen-design-tokens", source: "imported · llms.txt", uses: 11, hasInstr: true }
  ];

  return (
    <div style={{ padding: PADDING }}>
      <PaneHeader kanji="庫" eyebrow="This project · 5 libraries" title="Libraries"/>

      <HeroCard
        kanji="繋" eyebrow="Library coverage"
        headline="One library is unwrapped — tailwindcss."
        body="The other four have sensei-wrapped instruments, so any AI session in this project can ask them structural questions. Wrapping tailwindcss adds the same kind of coverage."
        action="wrap tailwindcss"
        meta="4 of 5 wrapped"
        tone="var(--success)"/>

      <div>
        <MiniHeading kanji="庫" label="In this project"/>
        {libs.map((l, i) => (
          <SimpleRow key={i}
            left={l.name}
            leftSub={l.source}
            right={`${l.uses}× used  ·  ${l.hasInstr ? '具 wrapped' : '— unwrapped'}`}
            rightTone={l.hasInstr ? 'var(--success)' : 'var(--warning)'}/>
        ))}
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
//  INSTRUMENTS · MCP tools scoped to this project
// ───────────────────────────────────────────────────────────
function ProjInstrumentsLite({ project }) {
  const tools = [
    { kanji: "具", name: "openapi.summarize",     scope: "openapi-3",   calls7d: 41, ftr: 1.00 },
    { kanji: "具", name: "openapi.search-paths",  scope: "openapi-3",   calls7d: 28, ftr: 0.96 },
    { kanji: "具", name: "stripe.lookup-event",   scope: "stripe-js",   calls7d: 14, ftr: 0.93 },
    { kanji: "具", name: "postgres.explain",      scope: "postgres",    calls7d: 32, ftr: 1.00 },
    { kanji: "具", name: "patterns.find-similar", scope: "this project", calls7d: 22, ftr: 0.91 },
    { kanji: "具", name: "session.find-similar",  scope: "this project", calls7d: 18, ftr: 1.00 },
    { kanji: "具", name: "memory.recall",         scope: "this project", calls7d: 9,  ftr: 1.00 }
  ];

  return (
    <div style={{ padding: PADDING }}>
      <PaneHeader kanji="具" eyebrow="This project · 7 instruments" title="Instruments"/>

      <HeroCard
        kanji="調" eyebrow="Tool effectiveness"
        headline="patterns.find-similar is your weakest tool here."
        body="91% first-try right across 22 calls in the last 7 days. The two miss-cases both involved the new Repository pattern (p4). One reinforcement pass would close the gap."
        action="open replay for patterns.find-similar"
        meta="164 calls · 7d"
        tone="var(--warning)"/>

      <div>
        <MiniHeading kanji="具" label="Project-scoped tools"/>
        {tools.map((t, i) => (
          <SimpleRow key={i}
            left={t.name}
            leftSub={`scope · ${t.scope}`}
            right={`${t.calls7d} calls  ·  ${Math.round(t.ftr * 100)}% FTR`}
            rightTone={t.ftr >= 0.95 ? 'var(--success)' : 'var(--warning)'}/>
        ))}
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
//  IMPACT · did sensei's advice work here?
// ───────────────────────────────────────────────────────────
function ProjImpactLite({ project }) {
  const verdicts = [
    { kanji: "果", title: "Wrote the auth-test persona (s-2891 retro).",
      acceptedAt: "12d ago",
      before: { ftr: 0.62, label: "FTR 14d before" },
      after:  { ftr: 0.79, label: "FTR 14d after"  },
      verdict: "positive", note: "Auth corrections fell from 3/wk → 0/wk. Persona is being used in 4 of 6 lumen-auth sessions." },
    { kanji: "果", title: "Extracted withAuth(handler) middleware (rec a1).",
      acceptedAt: "8d ago",
      before: { ftr: 0.79, label: "FTR 14d before" },
      after:  { ftr: 0.84, label: "FTR 14d after"  },
      verdict: "positive", note: "Duplicated guards collapsed from 11 sites → 1 wrapper. No regressions in 16 sessions touching auth." },
    { kanji: "果", title: "Promoted Retry-with-backoff pattern (p5).",
      acceptedAt: "3d ago",
      before: null, after: null,
      verdict: "pending", note: "Too soon to read — only 4 sessions since promotion. Verdict due after 14d window closes." }
  ];

  return (
    <div style={{ padding: PADDING }}>
      <PaneHeader kanji="果" eyebrow="This project · 2 verdicts pending" title="Impact"/>

      <HeroCard
        kanji="勝" eyebrow="Loop closed · 2 wins this month"
        headline="Sensei's two accepted recommendations both improved FTR."
        body="The auth-test persona moved FTR from 62% → 79% over 14 days. The withAuth middleware extraction added another 5 points. One more verdict is pending; no negative-impact alerts for this project."
        action="see all impact reports"
        meta="2 positive · 0 negative · 1 pending"
        tone="var(--success)"/>

      <div>
        <MiniHeading kanji="果" label="Accepted recommendations · this project"/>
        {verdicts.map((v, i) => {
          const tone = v.verdict === 'positive' ? 'var(--success)' :
                       v.verdict === 'negative' ? 'var(--accent)'  : 'var(--ink-3)';
          return (
            <div key={i} style={{
              padding: '12px 16px', marginBottom: 8,
              background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 6,
              borderLeft: `2px solid ${tone}`
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between',
                             alignItems: 'baseline', marginBottom: 4 }}>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{v.title}</div>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  accepted {v.acceptedAt}
                </span>
              </div>
              {v.before && v.after && (
                <div style={{ display: 'flex', gap: 16, alignItems: 'baseline', marginBottom: 4 }}>
                  <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                    {v.before.label} <span style={{ color: 'var(--ink)' }}>{Math.round(v.before.ftr * 100)}%</span>
                  </div>
                  <span style={{ fontSize: 13, color: tone }}>→</span>
                  <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                    {v.after.label} <span style={{ color: tone }}>{Math.round(v.after.ftr * 100)}%</span>
                  </div>
                  <span className="mono" style={{ fontSize: 11, color: tone, marginLeft: 'auto' }}>
                    +{Math.round((v.after.ftr - v.before.ftr) * 100)} pts
                  </span>
                </div>
              )}
              <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.5 }}>
                {v.note}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ───────────────────────────────────────────────────────────
//  ABOUT · the original Settings layout, in read mode by default,
//  with a top-right Edit toggle for inline editing.
// ───────────────────────────────────────────────────────────
function ProjAboutPane({ project }) {
  const [editing, setEditing] = pLS(false);
  return (
    <div style={{ position: 'relative', width: '100%', height: '100%',
                  display: 'flex', flexDirection: 'column' }}
         data-editing={editing ? "true" : "false"}>
      {/* Mode bar — minimal, sits above the document */}
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '12px 24px 12px 32px',
        borderBottom: 'var(--hairline)',
        background: editing ? 'var(--paper-2)' : 'var(--paper)'
      }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
          <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>識</span>
          <div className="display" style={{ fontSize: 15, color: 'var(--ink)',
                        letterSpacing: '-0.005em' }}>
            About {project.name}
          </div>
          <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            — identity, stack, repos, links, guidelines, backlog
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          {editing && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--accent)',
                          letterSpacing: '0.14em', textTransform: 'uppercase' }}>
              ● editing
            </span>
          )}
          <button onClick={() => setEditing(e => !e)}
                  style={{
                    padding: '4px 12px', fontSize: 11, borderRadius: 5,
                    border: 'var(--hairline)',
                    background: editing ? 'var(--ink)' : 'transparent',
                    color: editing ? 'var(--paper)' : 'var(--ink-2)',
                    cursor: 'pointer'
                  }}>
            {editing ? "✓  Done" : "✎  Edit"}
          </button>
        </div>
      </div>

      <div style={{ flex: 1, overflow: 'hidden' }}>
        <ProjSettingsV2 project={project}/>
      </div>
    </div>
  );
}

// Hide edit affordances when not in editing mode (small, contained CSS).
// Targets the icon ✎ pip, "+ add" buttons, "change" links, and the
// row-level pencil controls inside ProjSettingsV2.
if (typeof document !== 'undefined' && !document.getElementById('proj-about-edit-css')) {
  const s = document.createElement('style');
  s.id = 'proj-about-edit-css';
  s.textContent = `
    [data-editing="false"] .v2-edit,
    [data-editing="false"] button[title="Change icon"] > span:last-child {
      display: none !important;
    }
    [data-editing="false"] button { pointer-events: none; }
    [data-editing="false"] input,
    [data-editing="false"] textarea { pointer-events: none; user-select: text; }
  `;
  document.head.appendChild(s);
}

// ───────────────────────────────────────────────────────────
//  Export
// ───────────────────────────────────────────────────────────
Object.assign(window, {
  ProjOverviewLite, ProjMemoriesLite, ProjTraceabilityLite,
  ProjLibrariesLite, ProjInstrumentsLite, ProjImpactLite, ProjAboutPane
});
