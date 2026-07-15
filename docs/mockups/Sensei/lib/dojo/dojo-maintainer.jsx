// Dōjō · Maintainer console — govern the queue.
// Panels: Triage · Candidate · Knowledge. Job: open the queue → evaluate →
// decide (approve / revise / decline) → set distribution → publish & measure,
// and curate the published library.
// Reuses the shared frame from dojo-shared.jsx + primitives (Avatar / EnsoRing).

const { useState: dmS } = React;

const MAINT_NAV = [
  { group: "Govern", items: [
    { id: "triage",    kanji: "門", label: "Triage", badge: 7 },
    { id: "knowledge", kanji: "蔵", label: "Knowledge" },
    { id: "catalog",   kanji: "庫", label: "Catalog" },
  ]},
];

// Every scope has a named owner (set in Admin · Scopes); unowned → fallback.
const SCOPE_OWNERS = {
  "Company":         "Keiko T.",
  "Team · Payments": "Marco D.",
  "Client · Globex": "Sven K.",
  "Stack · React":   "Sven K.",
};
const SCOPE_FALLBACK = "Keiko T.";
const IMPACT_RANK = { high: 0, med: 1, low: 2 };
function ageHours(a) { const n = parseFloat(a) || 0; return /d/.test(a) ? n * 24 : /m/.test(a) ? n / 60 : n; }
function rankCandidates(arr) {
  return [...arr].sort((x, y) => (IMPACT_RANK[x.impact] - IMPACT_RANK[y.impact]) || (ageHours(x.age) - ageHours(y.age)));
}

/* ─── Triage queue ───────────────────────────────────────── */
function DojoTriage({ go, mobile = false }) {
  const D = window.DOJO;
  const groups = {};
  D.queue.forEach(c => { (groups[c.scope] ||= []).push(c); });
  const ordered = Object.entries(groups)
    .map(([scope, items]) => [scope, rankCandidates(items)])
    .sort((a, b) => (IMPACT_RANK[a[1][0].impact] - IMPACT_RANK[b[1][0].impact]) || (ageHours(a[1][0].age) - ageHours(b[1][0].age)));
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="門" eyebrow="Govern · triage" title="What's waiting for review"
        sub="Your scopes by default, ranked by projected impact then age. Every scope has an owner — anything unowned routes to a fallback so nothing sits idle. Nothing publishes without a decision."
        right={<div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 11, fontFamily: "var(--font-mono)",
                        color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid oklch(0.58 0.15 35/.28)",
                        borderRadius: 20, padding: "3px 10px" }}>✓ My scopes</span>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Sort · impact, then age ▾</DojoChip>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "8px 16px 16px" : "8px 28px 28px" }}>
        {ordered.map(([scope, items]) => {
          const owner = SCOPE_OWNERS[scope];
          return (
          <div key={scope} style={{ marginTop: 18 }}>
            <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8, marginBottom: 8 }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--ink-3)" }}>{items[0].scopeKanji}</span>
              <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{scope}</span>
              <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{items.length}</span>
              <span style={{ flex: 1 }} />
              {owner
                ? <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--ink-3)" }}>
                    <Avatar name={owner} size={18} />
                    <span>owner · <span style={{ color: "var(--ink-2)" }}>{owner}</span></span>
                  </span>
                : <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--ink-4)" }}>
                    <span style={{ width: 7, height: 7, borderRadius: "50%", border: "1px dashed var(--ink-4)" }} />
                    <span>unowned → <span style={{ color: "var(--ink-3)" }}>{SCOPE_FALLBACK}</span> · fallback</span>
                  </span>}
            </div>
            <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
              {items.map((c, i) => (
                <button key={c.id} onClick={() => go("triage", c.id)} style={{
                  display: "grid", gridTemplateColumns: mobile ? "auto 1fr auto" : "auto 1fr auto auto auto", gap: mobile ? 11 : 14, alignItems: "center",
                  width: "100%", textAlign: "left", padding: "13px 16px", cursor: "pointer", background: "transparent",
                  border: "none", borderBottom: i < items.length - 1 ? "1px solid var(--edge)" : "none",
                }}>
                  <span className="kanji" style={{ fontSize: 18, color: "var(--accent)", width: 22, textAlign: "center" }}>{c.kanji}</span>
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontSize: 13.5, color: "var(--ink)" }}>{c.title}</div>
                    <div style={{ display: "flex", gap: 8, marginTop: 5, alignItems: "center", flexWrap: "wrap" }}>
                      <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
                      <OriginChip origin={c.origin} />
                      <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>
                        {c.origin === "client" ? "source dropped" : c.by} · {c.evidence} sessions · {c.age}
                      </span>
                      {mobile && <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>· conf {Math.round(c.confidence * 100)}%{c.conflicts > 0 ? ` · ${c.conflicts} conflict` : ""}</span>}
                    </div>
                  </div>
                  {!mobile && <div style={{ display: "flex", gap: 10, alignItems: "center", minWidth: 96, justifyContent: "flex-end" }}>
                    {c.conflicts > 0 && <DojoChip tone="oklch(0.52 0.13 60)" soft="var(--warning-soft)">{c.conflicts} conflict</DojoChip>}
                    {c.dups > 0 && <DojoChip>{c.dups} dup</DojoChip>}
                  </div>}
                  {!mobile && <Confidence v={c.confidence} />}
                  <span style={{ fontSize: 13, color: "var(--ink-4)" }}>→</span>
                </button>
              ))}
            </div>
          </div>
          );
        })}
      </div>
    </div>
  );
}

/* ─── Candidate detail · evaluate → decide ──────────────────── */
function DojoCandidate({ id, go }) {
  const D = window.DOJO;
  const c = D.queue.find(x => x.id === id) || D.queue[0];
  const o = DOJO_ORIGIN[c.origin] || DOJO_ORIGIN.employer;
  const REACH = { "Company": { repos: 134, devs: 48 }, "Team · Payments": { repos: 6, devs: 14 },
    "Client · Globex": { repos: 3, devs: 7 }, "Stack · React": { repos: 22, devs: 19 }, "Stack · Postgres": { repos: 17, devs: 12 } };
  const r = REACH[c.scope] || { repos: 4, devs: 8 };
  const Block = ({ kanji, label, children }) => (
    <div style={{ marginBottom: 18 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 6 }}>
        <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: 10.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{label}</span>
      </div>
      <div style={{ fontSize: 13.5, color: "var(--ink)", lineHeight: 1.6 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "12px 28px", borderBottom: "var(--hairline)", flexShrink: 0 }}>
        <button onClick={() => go("triage")} className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>← Triage</button>
        <span style={{ fontSize: 11, color: "var(--ink-4)" }}>/</span>
        <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{c.scope}</span>
      </div>
      <div style={{ flex: 1, display: "grid", gridTemplateColumns: "1fr 332px", minHeight: 0 }}>
        <div style={{ overflow: "auto", padding: "24px 28px 32px" }}>
          <div style={{ display: "flex", gap: 9, marginBottom: 12, flexWrap: "wrap" }}>
            <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
            <DojoChip tone="var(--ink-2)">{c.scopeKanji} {c.scope}</DojoChip>
            <OriginChip origin={c.origin} />
            <DojoChip tone={c.impact === "high" ? "var(--accent)" : "var(--ink-3)"}>{c.impact} impact</DojoChip>
          </div>
          <h1 className="display" style={{ fontSize: 27, fontWeight: 300, letterSpacing: "-0.015em", lineHeight: 1.18, margin: "0 0 20px", color: "var(--ink)" }}>{c.title}</h1>
          <Block kanji="芽" label="The learning">{c.learning}</Block>
          <Block kanji="因" label="The cause">{c.cause}</Block>
          <Block kanji="周" label="The context — where it applies">{c.context}</Block>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 10, padding: "13px 16px", marginBottom: 16 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--ink-3)" }}>証</span>
              <span style={{ fontSize: 13, color: "var(--ink)" }}><b style={{ fontWeight: 600 }}>{c.evidence} sessions</b> support this</span>
              <span style={{ flex: 1 }} />
              <button className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>view evidence →</button>
            </div>
          </div>
          {c.origin === "client" && (
            <div style={{ background: "var(--accent-soft)", border: "1px solid oklch(0.58 0.15 35/.25)", borderRadius: 10, padding: "13px 16px", marginBottom: 16 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>盾</span>
                <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)", fontWeight: 600 }}>Source dereferenced automatically</span>
              </div>
              <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
                The lesson, its cause and context are kept; the source reference was dropped before it reached you — so this can be published anywhere safely.
              </div>
              <div style={{ display: "flex", gap: 6, marginTop: 9, flexWrap: "wrap" }}>
                {c.dereferenced.map(d => <DojoChip key={d} tone="var(--ink-3)" soft="var(--paper)">dropped · {d}</DojoChip>)}
              </div>
            </div>
          )}
          {(c.conflicts > 0 || c.dups > 0) && (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {c.conflicts > 0 && (
                <div style={{ background: "var(--warning-soft)", borderRadius: 10, padding: "13px 15px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 9 }}>
                    <span className="kanji" style={{ fontSize: 13, color: "oklch(0.52 0.13 60)" }}>衝</span>
                    <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "oklch(0.52 0.13 60)", fontWeight: 600 }}>Conflicts with a published rule</span>
                  </div>
                  <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: 8, overflow: "hidden" }}>
                    <div style={{ display: "flex", gap: 9, padding: "8px 12px", borderBottom: "1px solid var(--edge)", fontFamily: "var(--font-mono)", fontSize: 11.5 }}>
                      <span style={{ width: 10, flexShrink: 0, color: "oklch(0.52 0.13 60)", fontWeight: 700 }}>−</span>
                      <span style={{ color: "var(--ink-2)" }}><span style={{ color: "var(--ink-4)" }}>Company</span> · “Retry freely on transient errors”</span>
                    </div>
                    <div style={{ display: "flex", gap: 9, padding: "8px 12px", fontFamily: "var(--font-mono)", fontSize: 11.5 }}>
                      <span style={{ width: 10, flexShrink: 0, color: "var(--success)", fontWeight: 700 }}>+</span>
                      <span style={{ color: "var(--ink)" }}><span style={{ color: "var(--ink-4)" }}>{c.scope}</span> · “{c.title}”</span>
                    </div>
                  </div>
                  <div style={{ fontSize: 12, color: "var(--ink-2)", lineHeight: 1.5, marginTop: 9 }}>The more specific scope wins — approving lets <b style={{ fontWeight: 600 }}>{c.scope}</b> supersede the Company rule on money-moving paths, leaving it intact everywhere else.</div>
                  <div style={{ display: "flex", gap: 8, marginTop: 11 }}>
                    <button style={{ padding: "6px 12px", borderRadius: 7, border: "var(--hairline)", background: "var(--paper)", color: "var(--ink-2)", fontSize: 12, cursor: "pointer", fontFamily: "inherit" }}>View rule</button>
                    <button style={{ padding: "6px 12px", borderRadius: 7, border: "1px solid oklch(0.52 0.13 60/.4)", background: "var(--paper)", color: "oklch(0.5 0.13 60)", fontSize: 12, cursor: "pointer", fontFamily: "inherit" }}>Supersede on approve</button>
                  </div>
                </div>
              )}
              {c.dups > 0 && (() => {
                const dupsList = c.dups >= 2
                  ? [{ t: "Persona: drafts integration tests for auth", s: 0.86 }, { t: "Persona: refresh-flow test author", s: 0.81 }]
                  : [{ t: "Never write secrets to stdout in handlers", s: 0.78 }];
                return (
                <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 10, padding: "13px 15px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 9 }}>
                    <span className="kanji" style={{ fontSize: 13, color: "var(--ink-3)" }}>双</span>
                    <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{c.dups} near-duplicate{c.dups > 1 ? "s" : ""} · merge suggested</span>
                  </div>
                  {dupsList.map((d, i) => (
                    <div key={i} style={{ display: "flex", alignItems: "center", gap: 10, padding: "7px 0", borderBottom: i < dupsList.length - 1 ? "1px solid var(--edge)" : "none" }}>
                      <span style={{ fontSize: 12.5, color: "var(--ink)", flex: 1 }}>{d.t}</span>
                      <span className="mono" style={{ fontSize: 11, color: d.s >= 0.9 ? "var(--accent)" : "var(--ink-3)" }}>sim {d.s.toFixed(2)}</span>
                    </div>
                  ))}
                  <div style={{ marginTop: 10, fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
                    <span style={{ color: "var(--ink-3)" }}>Auto-dedupe · </span>≥&nbsp;0.90 merges automatically; <b style={{ fontWeight: 600 }}>0.75–0.90 is flagged here</b> for you to confirm.
                  </div>
                  <div style={{ display: "flex", gap: 8, marginTop: 11 }}>
                    <button style={{ padding: "6px 12px", borderRadius: 7, border: "1px solid oklch(0.58 0.15 35/.4)", background: "var(--accent-soft)", color: "var(--accent)", fontSize: 12, cursor: "pointer", fontFamily: "inherit" }}>Merge into canonical</button>
                    <button style={{ padding: "6px 12px", borderRadius: 7, border: "var(--hairline)", background: "var(--paper)", color: "var(--ink-2)", fontSize: 12, cursor: "pointer", fontFamily: "inherit" }}>Keep separate</button>
                  </div>
                </div>
                );
              })()}
            </div>
          )}
        </div>
        <div style={{ borderLeft: "var(--hairline)", background: "var(--paper-2)", overflow: "auto", padding: "22px 20px", display: "flex", flexDirection: "column", gap: 18 }}>
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6 }}>
            <EnsoRing progress={c.confidence} size={104} stroke={8} color="var(--accent)" label={Math.round(c.confidence * 100)} />
            <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)" }}>Confidence</span>
          </div>
          <div>
            <div style={{ fontSize: 10.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600, marginBottom: 6 }}>Attribution</div>
            <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.5 }}>
              {c.origin === "client" ? "Source-dereferenced — no contributor or client identity attached." : <>Named to <b style={{ color: "var(--ink)", fontWeight: 600 }}>{c.by}</b>, {c.origin === "community" ? "from the community" : "org-internal"}.</>}
            </div>
          </div>
          <div>
            <div style={{ display: "flex", alignItems: "center", marginBottom: 6 }}>
              <span style={{ fontSize: 10.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600 }}>Distribute to</span>
              <span style={{ flex: 1 }} />
              <button style={{ display: "inline-flex", alignItems: "center", gap: 5, background: "transparent", border: "none", cursor: "pointer", fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--ink-3)" }}>preset · team default ▾</button>
            </div>
            <button style={{ width: "100%", display: "flex", alignItems: "center", gap: 8, background: "var(--paper)", border: "var(--hairline)", borderRadius: 7, padding: "9px 11px", cursor: "pointer", textAlign: "left" }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>{c.scopeKanji}</span>
              <span style={{ fontSize: 13, color: "var(--ink)", flex: 1 }}>{c.scope}</span>
              <span className="mono" style={{ fontSize: 9, color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: ".06em" }}>binding</span>
              <span style={{ fontSize: 9, color: "var(--ink-3)" }}>▾</span>
            </button>
            <div style={{ marginTop: 8, background: "var(--paper)", border: "var(--hairline)", borderRadius: 8, padding: "9px 11px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
                <span className="kanji" style={{ fontSize: 12, color: "var(--ink-3)" }}>誰</span>
                <span style={{ fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Who gets this</span>
                <span style={{ flex: 1 }} />
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-2)" }}>{r.repos} repos · {r.devs} devs</span>
              </div>
              <button className="mono" style={{ marginTop: 6, fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0 }}>Preview recipients →</button>
            </div>
            <div style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 7, lineHeight: 1.45 }}>Inherits the contribution's binding. Narrowing is free; broadening beyond it asks you to confirm.</div>
          </div>
          <div style={{ borderTop: "var(--hairline)", paddingTop: 16, marginTop: "auto", display: "flex", flexDirection: "column", gap: 8 }}>
            {c.impact === "high" && (
              <div style={{ background: "var(--accent-soft)", border: "1px solid oklch(0.58 0.15 35/.25)", borderRadius: 8, padding: "10px 11px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 5 }}>
                  <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>検</span>
                  <span style={{ fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--accent)", fontWeight: 600 }}>Second approval required</span>
                </div>
                <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.45 }}>High-impact items need a second maintainer. Threshold set per scope in Scopes &amp; policies.</div>
                <button style={{ width: "100%", marginTop: 8, display: "flex", alignItems: "center", gap: 8, background: "var(--paper)", border: "var(--hairline)", borderRadius: 7, padding: "7px 10px", cursor: "pointer", textAlign: "left" }}>
                  <Avatar name="Sven K." size={18} />
                  <span style={{ fontSize: 12, color: "var(--ink-2)", flex: 1 }}>Sven K. · suggested approver</span>
                  <span style={{ fontSize: 9, color: "var(--ink-3)" }}>▾</span>
                </button>
              </div>
            )}
            <button style={{ width: "100%", padding: "11px", borderRadius: 8, border: "none", cursor: "pointer",
                background: "var(--ink)", color: "var(--paper)", fontSize: 13, fontWeight: 500, fontFamily: "inherit",
                display: "inline-flex", alignItems: "center", justifyContent: "center", gap: 7 }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>決</span> {c.impact === "high" ? "Approve & request 2nd" : "Approve & publish"}
            </button>
            <div style={{ display: "flex", gap: 8 }}>
              <button style={{ flex: 1, padding: "9px", borderRadius: 8, border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-2)", fontSize: 12.5, fontFamily: "inherit" }}>Revise</button>
              <button style={{ flex: 1, padding: "9px", borderRadius: 8, border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-2)", fontSize: 12.5, fontFamily: "inherit" }}>Decline</button>
            </div>
            <div style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 2, textAlign: "center" }}>{c.impact === "high" ? "Two named approvals, with notes, land in the audit trail." : "A named decision, with a note, lands in the audit trail."}</div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Knowledge · published library ──────────────────────── */
function DojoKnowledge() {
  const active = [
    { k: "守", title: "Never log refresh tokens, even at debug level", scope: "Company",     used: "142 repos", age: "8mo" },
    { k: "紋", title: "Idempotency key on money-moving mutations",      scope: "Team · Payments", used: "6 repos", age: "5mo" },
    { k: "盾", title: "Validate webhook signatures before parsing",     scope: "Stack",        used: "23 repos", age: "3mo" },
    { k: "理", title: "Public APIs stay backward-compatible two minors", scope: "Company",     used: "31 repos", age: "6mo" },
  ];
  const disabled = [
    { k: "技", title: "Skill: explain a slow query plan",          scope: "Stack · Postgres", reason: "superseded by a newer skill", left: 18 },
    { k: "問", title: "Persona: integration-test author (auth)",   scope: "Stack · React",    reason: "deprecated — flows changed",  left: 4 },
  ];
  const [pruneDays, setPruneDays] = dmS("30");
  const HeadCard = ({ children }) => (
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "16px 18px" }}>{children}</div>
  );
  const Row = ({ it, tone }) => (
    <div style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 13, alignItems: "center",
                  padding: "13px 16px", borderBottom: "1px solid var(--edge)" }}>
      <span className="kanji" style={{ fontSize: 17, color: tone || "var(--accent)", width: 20, textAlign: "center" }}>{it.k}</span>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 13.5, color: "var(--ink)" }}>{it.title}</div>
        <div style={{ display: "flex", gap: 8, marginTop: 4, alignItems: "center", flexWrap: "wrap" }}>
          <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-3)" }}>{it.scope}</span>
          {it.used && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>· {it.used}</span>}
          {it.reason && <span style={{ fontSize: 11, color: "var(--ink-3)" }}>· {it.reason}</span>}
        </div>
      </div>
      {it.age && <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{it.age}</span>}
      {it.left != null && (
        <span className="mono" style={{ fontSize: 11, color: it.left <= 7 ? "var(--accent)" : "var(--ink-3)" }}>evicted in {it.left}d</span>
      )}
    </div>
  );
  return (
    <div style={{ height: "100%", overflow: "auto" }}>
      <div style={{ display: "flex", alignItems: "flex-start", gap: 16, padding: "22px 28px 18px", borderBottom: "var(--hairline)" }}>
        <span className="kanji" style={{ fontSize: 38, color: "var(--accent)", lineHeight: 1, flexShrink: 0 }}>蔵</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: ".18em", textTransform: "uppercase", color: "var(--ink-3)", marginBottom: 4 }}>Govern · published library</div>
          <h1 className="display" style={{ fontSize: 23, fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: 1.05 }}>Knowledge</h1>
          <p style={{ fontSize: 13, color: "var(--ink-2)", lineHeight: 1.55, margin: "6px 0 0", maxWidth: 720 }}>
            The Dōjō holds only active, derived intelligence — guards, patterns, principles, skills and personas. As practice
            moves on, a maintainer disables what's gone redundant; disabled knowledge is then evicted automatically.
          </p>
        </div>
      </div>
      <div style={{ padding: 28 }}>
        <HeadCard>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <span className="kanji" style={{ fontSize: 18, color: "var(--accent)" }}>時</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: 14, color: "var(--ink)" }}>Prune disabled knowledge</div>
              <div style={{ fontSize: 12, color: "var(--ink-3)", marginTop: 2, lineHeight: 1.5 }}>
                How long an item stays recoverable after a maintainer disables it. After the window it's evicted from the library for good.
              </div>
            </div>
            <select value={pruneDays} onChange={e => setPruneDays(e.target.value)}
                    style={{ fontSize: 13, border: "var(--hairline)", borderRadius: 5, background: "var(--paper)",
                             color: "var(--ink)", cursor: "pointer", fontFamily: "inherit", padding: "6px 10px" }}>
              <option value="7">7 days</option>
              <option value="30">30 days · default</option>
              <option value="90">90 days</option>
              <option value="never">Never · keep disabled</option>
            </select>
          </div>
        </HeadCard>
        <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, margin: "22px 0 10px" }}>
          Active · {active.length}
        </div>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
          {active.map((it, i) => <Row key={i} it={it}/>)}
        </div>
        <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, margin: "22px 0 10px", display: "flex", alignItems: "center", gap: 8 }}>
          Disabled · pending pruning
          <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", letterSpacing: 0, textTransform: "none" }}>
            {pruneDays === "never" ? "retained — pruning off" : `evicting ${pruneDays}d after disable`}
          </span>
        </div>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden", opacity: 0.92 }}>
          {disabled.map((it, i) => <Row key={i} it={it} tone="var(--ink-4)"/>)}
        </div>
      </div>
    </div>
  );
}

/* ─── the maintainer console ─────────────────────────────── */
function DojoMaintainerConsole({ initial = "triage", initialCandidate = null, mobile = false, relayStart = null }) {
  const [section, setSection] = dmS(initial);
  const [candidate, setCandidate] = dmS(initialCandidate);
  const go = (sec, cand = null) => { setSection(sec); setCandidate(cand); };
  let screen;
  if (section === "triage" && candidate) screen = <DojoCandidate id={candidate} go={go} mobile={mobile} />;
  else if (section === "knowledge") screen = <DojoKnowledge mobile={mobile} />;
  else if (section === "catalog") screen = <DojoExtensions />;
  else screen = <DojoTriage go={go} mobile={mobile} />;
  return (
    <DojoRoleShell label="Dōjō · Maintainer console" role={{ kanji: "先", label: "Maintainer" }}
      nav={MAINT_NAV} active={candidate ? "triage" : section} setActive={(s) => go(s)} mobile={mobile} relayStart={relayStart}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoMaintainerConsole, DojoTriage, DojoCandidate, DojoKnowledge });
