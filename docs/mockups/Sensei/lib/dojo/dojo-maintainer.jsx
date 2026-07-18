// Dōjō · Maintainer console — govern the queue.
// Panels: Triage · Candidate · Knowledge. Job: open the queue → evaluate →
// decide (approve / revise / decline) → set distribution → publish & measure,
// and curate the published library.
// Reuses the shared frame from dojo-shared.jsx + primitives (Avatar / EnsoRing).

const { useState: dmS } = React;

const MAINT_NAV = [
  { group: "Govern", items: [
    { id: "triage",    kanji: "門", label: "Triage", badge: 7 },
    { id: "approvals", kanji: "承", label: "Approvals", badge: 2 },
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
        right={<div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", fontFamily: "var(--font-mono)",
                        color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid var(--accent-edge)",
                        borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-2)" }}>✓ My scopes</span>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Sort · impact, then age ▾</DojoChip>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-2) var(--space-4) var(--space-4)" : "var(--space-2) var(--space-5) var(--space-5)" }}>
        {ordered.map(([scope, items]) => {
          const owner = SCOPE_OWNERS[scope];
          return (
          <div key={scope} style={{ marginTop: "var(--space-4)" }}>
            <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>{items[0].scopeKanji}</span>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{scope}</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{items.length}</span>
              <span style={{ flex: 1 }} />
              {owner
                ? <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
                    <Avatar name={owner} size={18} />
                    <span>owner · <span style={{ color: "var(--ink-soft)" }}>{owner}</span></span>
                  </span>
                : <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>
                    <span style={{ width: 7, height: 7, borderRadius: "50%", border: "1px dashed var(--ink-faint)" }} />
                    <span>unowned → <span style={{ color: "var(--ink-mute)" }}>{SCOPE_FALLBACK}</span> · fallback</span>
                  </span>}
            </div>
            <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
              {items.map((c, i) => (
                <button key={c.id} onClick={() => go("triage", c.id)} style={{
                  display: "grid", gridTemplateColumns: mobile ? "auto 1fr auto" : "auto 1fr auto auto auto", gap: mobile ? "var(--space-3)" : "var(--space-3)", alignItems: "center",
                  width: "100%", textAlign: "left", padding: "var(--space-3) var(--space-4)", cursor: "pointer", background: "transparent",
                  border: "none", borderBottom: i < items.length - 1 ? "1px solid var(--paper-edge)" : "none",
                }}>
                  <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 22, textAlign: "center" }}>{c.kanji}</span>
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{c.title}</div>
                    <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-1)", alignItems: "center", flexWrap: "wrap" }}>
                      <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
                      <OriginChip origin={c.origin} />
                      <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>
                        {c.origin === "client" ? "anonymized" : c.by} · {c.evidence} sessions · {c.age}
                      </span>
                      {mobile && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>· conf {Math.round(c.confidence * 100)}%{c.conflicts > 0 ? ` · ${c.conflicts} conflict` : ""}</span>}
                    </div>
                  </div>
                  {!mobile && <div style={{ display: "flex", gap: "var(--space-2)", alignItems: "center", minWidth: 96, justifyContent: "flex-end" }}>
                    {c.conflicts > 0 && <DojoChip tone="var(--warning)" soft="var(--warning-soft)">{c.conflicts} conflict</DojoChip>}
                    {c.dups > 0 && <DojoChip>{c.dups} dup</DojoChip>}
                  </div>}
                  {!mobile && <Confidence v={c.confidence} />}
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-faint)" }}>→</span>
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
function DojoCandidate({ id, go, mobile = false }) {
  const [revising, setRevising] = dmS(false);
  const [revision, setRevision] = dmS(null); // { title, learning } once saved
  const [showRecipients, setShowRecipients] = dmS(false);
  const revTitleRef = React.useRef(null);
  const revLearnRef = React.useRef(null);
  const D = window.DOJO;
  const c = D.queue.find(x => x.id === id) || D.queue[0];
  const o = DOJO_ORIGIN[c.origin] || DOJO_ORIGIN.employer;
  const REACH = { "Company": { repos: 134, devs: 48 }, "Team · Payments": { repos: 6, devs: 14 },
    "Client · Globex": { repos: 3, devs: 7 }, "Stack · React": { repos: 22, devs: 19 }, "Stack · Postgres": { repos: 17, devs: 12 } };
  const r = REACH[c.scope] || null;
  const secondApprover = SCOPE_OWNERS[c.scope] || SCOPE_FALLBACK;
  const Block = ({ kanji, label, children }) => (
    <div style={{ marginBottom: "var(--space-4)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{label}</span>
      </div>
      <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", lineHeight: 1.6 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-5)", borderBottom: "var(--hairline)", flexShrink: 0 }}>
        <button onClick={() => go("triage")} className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>← Triage</button>
        <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>/</span>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{c.scope}</span>
      </div>
      <div style={{ flex: 1, display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 332px", minHeight: 0 }}>
        <div style={{ overflow: "auto", padding: "var(--space-5) var(--space-5) var(--space-6)" }}>
          {revising && (
            <div style={{ background: "var(--paper-soft)", border: "1px solid var(--accent)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)", marginBottom: "var(--space-4)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>筆</span>
                <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)", fontWeight: 700 }}>Revise before publishing</span>
                <span style={{ flex: 1 }} />
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>edits are recorded in the audit trail</span>
              </div>
              <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-1)" }}>Title</div>
              <div ref={revTitleRef} contentEditable suppressContentEditableWarning style={{ fontSize: "var(--text-sm)", color: "var(--ink)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-3)", marginBottom: "var(--space-3)", outline: "none" }}>{revision ? revision.title : c.title}</div>
              <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-1)" }}>The learning</div>
              <div ref={revLearnRef} contentEditable suppressContentEditableWarning style={{ fontSize: "var(--text-sm)", color: "var(--ink)", lineHeight: 1.6, background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-3)", minHeight: 70, outline: "none" }}>{revision ? revision.learning : c.learning}</div>
              <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-3)" }}>
                <button onClick={() => { setRevision({ title: (revTitleRef.current?.textContent || c.title).trim(), learning: (revLearnRef.current?.textContent || c.learning).trim() }); setRevising(false); }} style={{ padding: "var(--space-2) var(--space-4)", borderRadius: "var(--radius-lg)", border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-sm)", fontWeight: 500, fontFamily: "inherit", display: "inline-flex", alignItems: "center", gap: "var(--space-2)" }}><span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>筆</span> Save revision</button>
                <button onClick={() => setRevising(false)} style={{ padding: "var(--space-2) var(--space-4)", borderRadius: "var(--radius-lg)", border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-sm)", fontFamily: "inherit" }}>Cancel</button>
              </div>
            </div>
          )}
          <div style={{ display: "flex", gap: "var(--space-2)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
            <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
            <DojoChip tone="var(--ink-soft)">{c.scopeKanji} {c.scope}</DojoChip>
            <OriginChip origin={c.origin} />
            <DojoChip tone={c.impact === "high" ? "var(--accent)" : "var(--ink-mute)"}>{c.impact} impact</DojoChip>
          </div>
          <h1 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, letterSpacing: "-0.015em", lineHeight: 1.18, margin: "0 0 var(--space-2)", color: "var(--ink)" }}>{revision ? revision.title : c.title}</h1>
          {revision && <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
            <DojoChip tone="var(--accent)" soft="var(--accent-soft)">筆 revised · recorded in audit trail</DojoChip>
            <button className="mono" onClick={() => setRevision(null)} style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "none", border: "none", cursor: "pointer" }}>revert</button>
          </div>}
          <Block kanji="芽" label="The learning">{revision ? revision.learning : c.learning}</Block>
          <Block kanji="因" label="The cause">{c.cause}</Block>
          <Block kanji="周" label="The context — where it applies">{c.context}</Block>
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>証</span>
              <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}><b style={{ fontWeight: 600 }}>{c.evidence} sessions</b> support this</span>
              <span style={{ flex: 1 }} />
              <button className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>view evidence →</button>
            </div>
          </div>
          {c.origin === "client" && (
            <div style={{ background: "var(--accent-soft)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>盾</span>
                <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--accent)", fontWeight: 600 }}>Source anonymized automatically</span>
              </div>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>
                The lesson, its cause and context are kept; the source reference was dropped before it reached you — so this can be published anywhere safely.
              </div>
              <div style={{ display: "flex", gap: "var(--space-1)", marginTop: "var(--space-2)", flexWrap: "wrap" }}>
                {c.anonymized.map(d => <DojoChip key={d} tone="var(--ink-mute)" soft="var(--paper)">dropped · {d}</DojoChip>)}
              </div>
            </div>
          )}
          {(c.conflicts > 0 || c.dups > 0) && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              {c.conflicts > 0 && (
                <div style={{ background: "var(--warning-soft)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--warning)" }}>衝</span>
                    <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--warning)", fontWeight: 600 }}>Conflicts with a published rule</span>
                  </div>
                  <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
                    <div style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-2) var(--space-3)", borderBottom: "1px solid var(--paper-edge)", fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)" }}>
                      <span style={{ width: 10, flexShrink: 0, color: "var(--warning)", fontWeight: 700 }}>−</span>
                      <span style={{ color: "var(--ink-soft)" }}><span style={{ color: "var(--ink-faint)" }}>Company</span> · “Retry freely on transient errors”</span>
                    </div>
                    <div style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-2) var(--space-3)", fontFamily: "var(--font-mono)", fontSize: "var(--text-xs)" }}>
                      <span style={{ width: 10, flexShrink: 0, color: "var(--success)", fontWeight: 700 }}>+</span>
                      <span style={{ color: "var(--ink)" }}><span style={{ color: "var(--ink-faint)" }}>{c.scope}</span> · “{c.title}”</span>
                    </div>
                  </div>
                  <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", lineHeight: 1.5, marginTop: "var(--space-2)" }}>The more specific scope wins — approving lets <b style={{ fontWeight: 600 }}>{c.scope}</b> supersede the Company rule on money-moving paths, leaving it intact everywhere else.</div>
                  <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-3)" }}>
                    <button style={{ padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)", border: "var(--hairline)", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-xs)", cursor: "pointer", fontFamily: "inherit" }}>View rule</button>
                    <button style={{ padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)", border: "1px solid var(--danger-edge)", background: "var(--paper)", color: "var(--danger)", fontSize: "var(--text-xs)", cursor: "pointer", fontFamily: "inherit" }}>Supersede on approve</button>
                  </div>
                </div>
              )}
              {c.dups > 0 && (() => {
                const dupsList = c.dups >= 2
                  ? [{ t: "Persona: drafts integration tests for auth", s: 0.86 }, { t: "Persona: refresh-flow test author", s: 0.81 }]
                  : [{ t: "Never write secrets to stdout in handlers", s: 0.78 }];
                return (
                <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>双</span>
                    <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{c.dups} near-duplicate{c.dups > 1 ? "s" : ""} · merge suggested</span>
                  </div>
                  {dupsList.map((d, i) => (
                    <div key={i} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-2) 0", borderBottom: i < dupsList.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                      <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", flex: 1 }}>{d.t}</span>
                      <span className="mono" style={{ fontSize: "var(--text-xs)", color: d.s >= 0.9 ? "var(--accent)" : "var(--ink-mute)" }}>sim {d.s.toFixed(2)}</span>
                    </div>
                  ))}
                  <div style={{ marginTop: "var(--space-2)", fontSize: "var(--text-xs)", color: "var(--ink-soft)", lineHeight: 1.55 }}>
                    <span style={{ color: "var(--ink-mute)" }}>Auto-dedupe · </span>≥&nbsp;0.90 merges automatically; <b style={{ fontWeight: 600 }}>0.75–0.90 is flagged here</b> for you to confirm.
                  </div>
                  <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-3)" }}>
                    <button style={{ padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)", border: "1px solid var(--accent-edge)", background: "var(--accent-soft)", color: "var(--accent)", fontSize: "var(--text-xs)", cursor: "pointer", fontFamily: "inherit" }}>Merge into canonical</button>
                    <button style={{ padding: "var(--space-1) var(--space-3)", borderRadius: "var(--radius)", border: "var(--hairline)", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-xs)", cursor: "pointer", fontFamily: "inherit" }}>Keep separate</button>
                  </div>
                </div>
                );
              })()}
            </div>
          )}
        </div>
        <div style={{ borderLeft: mobile ? "none" : "var(--hairline)", borderTop: mobile ? "var(--hairline)" : "none", background: "var(--paper-soft)", overflow: "auto", padding: "var(--space-5) var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "var(--space-1)" }}>
            <EnsoRing progress={c.confidence} size={104} stroke={8} color="var(--accent)" label={Math.round(c.confidence * 100)} />
            <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)" }}>Confidence</span>
          </div>
          <div>
            <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-1)" }}>Attribution</div>
            <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.5 }}>
              {c.origin === "client" ? "Source-anonymized — no contributor or client identity attached." : <>Named to <b style={{ color: "var(--ink)", fontWeight: 600 }}>{c.by}</b>, {c.origin === "community" ? "from the community" : "org-internal"}.</>}
            </div>
          </div>
          <div>
            <div style={{ display: "flex", alignItems: "center", marginBottom: "var(--space-1)" }}>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600 }}>Distribute to</span>
              <span style={{ flex: 1 }} />
              <button style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", background: "transparent", border: "none", cursor: "pointer", fontSize: "var(--text-xs)", fontFamily: "var(--font-mono)", color: "var(--ink-mute)" }}>preset · team default ▾</button>
            </div>
            <button style={{ width: "100%", display: "flex", alignItems: "center", gap: "var(--space-2)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-3)", cursor: "pointer", textAlign: "left" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{c.scopeKanji}</span>
              <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", flex: 1 }}>{c.scope}</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", textTransform: "uppercase", letterSpacing: ".06em" }}>binding</span>
              <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>▾</span>
            </button>
            <div style={{ marginTop: "var(--space-2)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-3)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>誰</span>
                <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Who gets this</span>
                <span style={{ flex: 1 }} />
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{r ? `${r.repos} repos · ${r.devs} devs` : "scope not sized yet"}</span>
              </div>
              <button className="mono" onClick={() => setShowRecipients(v => !v)} style={{ marginTop: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--accent)", background: "none", border: "none", cursor: "pointer", padding: 0 }}>{showRecipients ? "Hide recipients ↑" : "Preview recipients →"}</button>
              {showRecipients && (
                <div style={{ marginTop: "var(--space-2)", borderTop: "1px solid var(--paper-edge)", paddingTop: "var(--space-2)" }}>
                  {["ledger-core · 6 devs", "payments-api · 4 devs", "checkout-web · 4 devs"].map(x => (
                    <div key={x} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-1) 0", fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>
                      <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>庫</span><span className="mono">{x}</span>
                    </div>
                  ))}
                  <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>Dry-run — who receives this on publish, per the binding above.</div>
                </div>
              )}
            </div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-2)", lineHeight: 1.45 }}>Inherits the contribution's binding. Narrowing is free; broadening beyond it asks you to confirm.</div>
          </div>
          <div style={{ borderTop: "var(--hairline)", paddingTop: "var(--space-4)", marginTop: "auto", display: "flex", flexDirection: "column", gap: "var(--space-2)",
                position: mobile ? "sticky" : "static", bottom: mobile ? 0 : "auto", background: mobile ? "var(--paper-soft)" : "transparent",
                paddingBottom: mobile ? "var(--space-2)" : 0, marginLeft: mobile ? -20 : 0, marginRight: mobile ? -20 : 0, paddingLeft: mobile ? "var(--space-4)" : 0, paddingRight: mobile ? "var(--space-4)" : 0, boxShadow: mobile ? "var(--shadow-up)" : "none" }}>
            {c.impact === "high" && (
              <div style={{ background: "var(--accent-soft)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-3)" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>検</span>
                  <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--accent)", fontWeight: 600 }}>Second approval required</span>
                </div>
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", lineHeight: 1.45 }}>High-impact items need a second maintainer. Threshold set per scope in Scopes &amp; policies.</div>
                <button style={{ width: "100%", marginTop: "var(--space-2)", display: "flex", alignItems: "center", gap: "var(--space-2)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-2)", cursor: "pointer", textAlign: "left" }}>
                  <Avatar name={secondApprover} size={18} />
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", flex: 1 }}>{secondApprover} · suggested approver</span>
                  <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>▾</span>
                </button>
              </div>
            )}
            <button style={{ width: "100%", padding: "var(--space-3)", borderRadius: "var(--radius-lg)", border: "none", cursor: "pointer",
                background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-sm)", fontWeight: 500, fontFamily: "inherit",
                display: "inline-flex", alignItems: "center", justifyContent: "center", gap: "var(--space-2)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>決</span> {c.impact === "high" ? "Approve & request 2nd" : "Approve & publish"}
            </button>
            <div style={{ display: "flex", gap: "var(--space-2)" }}>
              <button onClick={() => setRevising(true)} style={{ flex: 1, padding: "var(--space-2)", borderRadius: "var(--radius-lg)", border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-sm)", fontFamily: "inherit" }}>Revise</button>
              <button style={{ flex: 1, padding: "var(--space-2)", borderRadius: "var(--radius-lg)", border: "1px solid var(--danger-edge)", cursor: "pointer", background: "var(--danger-soft)", color: "var(--danger)", fontSize: "var(--text-sm)", fontFamily: "inherit" }}>Decline</button>
            </div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)", textAlign: "center" }}>{c.impact === "high" ? "Two named approvals, with notes, land in the audit trail." : "A named decision, with a note, lands in the audit trail."}</div>
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
    <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>{children}</div>
  );
  const Row = ({ it, tone }) => (
    <div style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center",
                  padding: "var(--space-3) var(--space-4)", borderBottom: "1px solid var(--paper-edge)" }}>
      <span className="kanji" style={{ fontSize: "var(--text-lg)", color: tone || "var(--accent)", width: 20, textAlign: "center" }}>{it.k}</span>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{it.title}</div>
        <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-1)", alignItems: "center", flexWrap: "wrap" }}>
          <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{it.scope}</span>
          {it.used && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>· {it.used}</span>}
          {it.reason && <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>· {it.reason}</span>}
        </div>
      </div>
      {it.age && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{it.age}</span>}
      {it.left != null && (
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: it.left <= 7 ? "var(--accent)" : "var(--ink-mute)" }}>evicted in {it.left}d</span>
      )}
    </div>
  );
  return (
    <div style={{ height: "100%", overflow: "auto" }}>
      <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-4)", padding: "var(--space-5) var(--space-5) var(--space-4)", borderBottom: "var(--hairline)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-3xl)", color: "var(--accent)", lineHeight: 1, flexShrink: 0 }}>蔵</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".18em", textTransform: "uppercase", color: "var(--ink-mute)", marginBottom: "var(--space-1)" }}>Govern · published library</div>
          <h1 className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: 1.05 }}>Knowledge</h1>
          <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, margin: "var(--space-1) 0 0", maxWidth: 720 }}>
            The Dōjō holds only active, derived intelligence — guards, patterns, principles, skills and personas. As practice
            moves on, a maintainer disables what's gone redundant; disabled knowledge is then evicted automatically.
          </p>
        </div>
      </div>
      <div style={{ padding: "var(--space-5)" }}>
        <HeadCard>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)" }}>時</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>Prune disabled knowledge</div>
              <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)", lineHeight: 1.5 }}>
                How long an item stays recoverable after a maintainer disables it. After the window it's evicted from the library for good.
              </div>
            </div>
            <select value={pruneDays} onChange={e => setPruneDays(e.target.value)}
                    style={{ fontSize: "var(--text-sm)", border: "var(--hairline)", borderRadius: "var(--radius-sm)", background: "var(--paper)",
                             color: "var(--ink)", cursor: "pointer", fontFamily: "inherit", padding: "var(--space-1) var(--space-2)" }}>
              <option value="7">7 days</option>
              <option value="30">30 days · default</option>
              <option value="90">90 days</option>
              <option value="never">Never · keep disabled</option>
            </select>
          </div>
        </HeadCard>
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, margin: "var(--space-5) 0 var(--space-2)" }}>
          Active · {active.length}
        </div>
        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
          {active.map((it, i) => <Row key={i} it={it}/>)}
        </div>
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, margin: "var(--space-5) 0 var(--space-2)", display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
          Disabled · pending pruning
          <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", letterSpacing: 0, textTransform: "none" }}>
            {pruneDays === "never" ? "retained — pruning off" : `evicting ${pruneDays}d after disable`}
          </span>
        </div>
        <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden", opacity: 0.92 }}>
          {disabled.map((it, i) => <Row key={i} it={it} tone="var(--ink-faint)"/>)}
        </div>
      </div>
    </div>
  );
}

/* ─── Approvals · awaiting my second approval ────────────── */
function DojoApprovals({ go }) {
  const D = window.DOJO;
  const items = (D.queue || []).filter(c => c.impact === "high").slice(0, 3);
  const fallback = items.length ? items : (D.queue || []).slice(0, 2);
  const rows = fallback.map((c) => ({ ...c, first: SCOPE_OWNERS[c.scope] || SCOPE_FALLBACK, when: c.age || "—" }));
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="承" eyebrow="Govern · second approval" title="Awaiting your approval"
        sub="High-impact teachings a first maintainer approved and routed to you for a second sign-off. Two named approvals — with notes — land in the audit trail before anything publishes."
        right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">{rows.length} waiting</DojoChip>} />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-5)" }}>
        {rows.length === 0 ? (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "var(--space-3)", padding: "var(--space-8) 0", color: "var(--ink-mute)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-3xl)", color: "var(--ink-faint)" }}>空</span>
            <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>Nothing awaiting your second approval.</div>
          </div>
        ) : (
          <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden", maxWidth: 900 }}>
            {rows.map((c, i) => (
              <div key={c.id} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-4)", alignItems: "center", padding: "var(--space-4) var(--space-4)", borderBottom: i < rows.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 22, textAlign: "center" }}>{c.kanji}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{c.title}</div>
                  <div style={{ display: "flex", gap: "var(--space-2)", marginTop: "var(--space-1)", alignItems: "center", flexWrap: "wrap" }}>
                    <DojoChip tone="var(--accent)" soft="var(--accent-soft)">high impact</DojoChip>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{c.scopeKanji} {c.scope}</span>
                    <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
                      <Avatar name={c.first} size={16} /> 1st · {c.first} · {c.when}
                    </span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: "var(--space-2)" }}>
                  <DojoBtn size="sm" variant="ghost" onClick={() => go && go("triage", c.id)}>Review</DojoBtn>
                  <DojoBtn size="sm" kanji="承">Approve</DojoBtn>
                </div>
              </div>
            ))}
          </div>
        )}
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
  else if (section === "approvals") screen = <DojoApprovals go={go} mobile={mobile} />;
  else if (section === "catalog") screen = <DojoExtensions mobile={mobile} />;
  else screen = <DojoTriage go={go} mobile={mobile} />;
  return (
    <DojoRoleShell label="Dōjō · Maintainer console" role={{ kanji: "先", label: "Maintainer" }}
      nav={MAINT_NAV} active={candidate ? "triage" : section} setActive={(s) => go(s)} mobile={mobile} relayStart={relayStart}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoMaintainerConsole, DojoTriage, DojoCandidate, DojoKnowledge });
