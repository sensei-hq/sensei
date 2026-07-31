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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="門" eyebrow="Govern · triage" title="What's waiting for review"
        sub="Your scopes by default, ranked by projected impact then age. Every scope has an owner — anything unowned routes to a fallback so nothing sits idle. Nothing publishes without a decision."
        right={<div className="flex gap-2 items-center" >
          <span className="inline-flex items-center gap-1 text-xs text-accent bg-accent-soft rounded-full py-1 px-2" style={{ fontFamily: "var(--font-mono)", border: "1px solid var(--accent-edge)" }}>✓ My scopes</span>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Sort · impact, then age ▾</DojoChip>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-2) var(--space-4) var(--space-4)" : "var(--space-2) var(--space-6) var(--space-6)" }}>
        {ordered.map(([scope, items]) => {
          const owner = SCOPE_OWNERS[scope];
          return (
          <div className="mt-4" key={scope} >
            <div className="flex flex-wrap items-center gap-2 mb-2" >
              <span className="kanji text-sm text-ink-mute" >{items[0].scopeKanji}</span>
              <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".12em" }}>{scope}</span>
              <span className="mono text-xs text-ink-faint" >{items.length}</span>
              <span className="flex-1" />
              {owner
                ? <span className="inline-flex items-center gap-1 text-xs text-ink-mute" >
                    <Avatar name={owner} size={18} />
                    <span>owner · <span className="text-ink-soft" >{owner}</span></span>
                  </span>
                : <span className="inline-flex items-center gap-1 text-xs text-ink-faint" >
                    <span className="rounded-full" style={{ width: 7, height: 7, border: "1px dashed var(--ink-faint)" }} />
                    <span>unowned → <span className="text-ink-mute" >{SCOPE_FALLBACK}</span> · fallback</span>
                  </span>}
            </div>
            <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
              {items.map((c, i) => (
                <button className="grid items-center w-full text-left py-3 px-4 cursor-pointer bg-transparent border-0" key={c.id} onClick={() => go("triage", c.id)} style={{ gridTemplateColumns: mobile ? "auto 1fr auto" : "auto 1fr auto auto auto", gap: mobile ? "var(--space-3)" : "var(--space-3)", borderBottom: i < items.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{c.kanji}</span>
                  <div className="min-w-0" >
                    <div className="text-sm text-ink" >{c.title}</div>
                    <div className="flex gap-2 mt-1 items-center flex-wrap" >
                      <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
                      <OriginChip origin={c.origin} />
                      <span className="mono text-xs text-ink-faint" >
                        {c.origin === "client" ? "anonymized" : c.by} · {c.evidence} sessions · {c.age}
                      </span>
                      {mobile && <span className="mono text-xs text-ink-faint" >· conf {Math.round(c.confidence * 100)}%{c.conflicts > 0 ? ` · ${c.conflicts} conflict` : ""}</span>}
                    </div>
                  </div>
                  {!mobile && <div className="flex gap-2 items-center justify-end" style={{ minWidth: 96 }}>
                    {c.conflicts > 0 && <DojoChip tone="var(--warning)" soft="var(--warning-soft)">{c.conflicts} conflict</DojoChip>}
                    {c.dups > 0 && <DojoChip>{c.dups} dup</DojoChip>}
                  </div>}
                  {!mobile && <Confidence v={c.confidence} />}
                  <span className="text-sm text-ink-faint" >→</span>
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
    <div className="mb-4" >
      <div className="flex items-center gap-2 mb-1" >
        <span className="kanji text-sm text-accent" >{kanji}</span>
        <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>{label}</span>
      </div>
      <div className="text-sm text-ink" style={{ lineHeight: 1.6 }}>{children}</div>
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <div className="flex items-center gap-2 py-3 px-6 border-b shrink-0" >
        <button onClick={() => go("triage")} className="mono text-xs text-accent border-0 cursor-pointer" style={{ background: "none" }}>← Triage</button>
        <span className="text-xs text-ink-faint" >/</span>
        <span className="mono text-xs text-ink-mute" >{c.scope}</span>
      </div>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: mobile ? "1fr" : "1fr 332px" }}>
        <div className="overflow-auto" style={{ padding: "var(--space-6) var(--space-6) var(--space-8)" }}>
          {revising && (
            <div className="bg-paper-soft rounded-lg py-4 px-4 mb-4" style={{ border: "1px solid var(--accent)" }}>
              <div className="flex items-center gap-2 mb-3" >
                <span className="kanji text-base text-accent" >筆</span>
                <span className="text-xs uppercase text-accent" style={{ letterSpacing: ".12em", fontWeight: 700 }}>Revise before publishing</span>
                <span className="flex-1" />
                <span className="mono text-xs text-ink-faint" >edits are recorded in the audit trail</span>
              </div>
              <div className="text-xs uppercase text-ink-faint font-semibold mb-1" style={{ letterSpacing: ".1em" }}>Title</div>
              <div className="text-sm text-ink bg-paper border border-paper-edge rounded-lg py-2 px-3 mb-3" ref={revTitleRef} contentEditable suppressContentEditableWarning style={{ outline: "none" }}>{revision ? revision.title : c.title}</div>
              <div className="text-xs uppercase text-ink-faint font-semibold mb-1" style={{ letterSpacing: ".1em" }}>The learning</div>
              <div className="text-sm text-ink bg-paper border border-paper-edge rounded-lg py-2 px-3" ref={revLearnRef} contentEditable suppressContentEditableWarning style={{ lineHeight: 1.6, minHeight: 70, outline: "none" }}>{revision ? revision.learning : c.learning}</div>
              <div className="flex gap-2 mt-3" >
                <button className="py-2 px-4 rounded-lg border-0 cursor-pointer bg-ink text-paper text-sm font-medium inline-flex items-center gap-2" onClick={() => { setRevision({ title: (revTitleRef.current?.textContent || c.title).trim(), learning: (revLearnRef.current?.textContent || c.learning).trim() }); setRevising(false); }} style={{ fontFamily: "inherit" }}><span className="kanji text-sm text-accent" >筆</span> Save revision</button>
                <button className="py-2 px-4 rounded-lg border border-paper-edge cursor-pointer bg-paper text-ink-soft text-sm" onClick={() => setRevising(false)} style={{ fontFamily: "inherit" }}>Cancel</button>
              </div>
            </div>
          )}
          <div className="flex gap-2 mb-3 flex-wrap" >
            <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
            <DojoChip tone="var(--ink-soft)">{c.scopeKanji} {c.scope}</DojoChip>
            <OriginChip origin={c.origin} />
            <DojoChip tone={c.impact === "high" ? "var(--accent)" : "var(--ink-mute)"}>{c.impact} impact</DojoChip>
          </div>
          <h1 className="display text-2xl font-light text-ink" style={{ letterSpacing: "-0.015em", lineHeight: 1.18, margin: "0 0 var(--space-2)" }}>{revision ? revision.title : c.title}</h1>
          {revision && <div className="flex items-center gap-2 mb-3" >
            <DojoChip tone="var(--accent)" soft="var(--accent-soft)">筆 revised · recorded in audit trail</DojoChip>
            <button className="mono text-xs text-ink-mute border-0 cursor-pointer" onClick={() => setRevision(null)} style={{ background: "none" }}>revert</button>
          </div>}
          <Block kanji="芽" label="The learning">{revision ? revision.learning : c.learning}</Block>
          <Block kanji="因" label="The cause">{c.cause}</Block>
          <Block kanji="周" label="The context — where it applies">{c.context}</Block>
          <div className="bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 mb-4" >
            <div className="flex items-center gap-2" >
              <span className="kanji text-sm text-ink-mute" >証</span>
              <span className="text-sm text-ink" ><b className="font-semibold" >{c.evidence} sessions</b> support this</span>
              <span className="flex-1" />
              <button className="mono text-xs text-accent border-0 cursor-pointer" style={{ background: "none" }}>view evidence →</button>
            </div>
          </div>
          {c.origin === "client" && (
            <div className="bg-accent-soft rounded-lg py-3 px-4 mb-4" style={{ border: "1px solid var(--accent-edge)" }}>
              <div className="flex items-center gap-2 mb-1" >
                <span className="kanji text-sm text-accent" >盾</span>
                <span className="text-xs uppercase text-accent font-semibold" style={{ letterSpacing: ".12em" }}>Source anonymized automatically</span>
              </div>
              <div className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>
                The lesson, its cause and context are kept; the source reference was dropped before it reached you — so this can be published anywhere safely.
              </div>
              <div className="flex gap-1 mt-2 flex-wrap" >
                {c.anonymized.map(d => <DojoChip key={d} tone="var(--ink-mute)" soft="var(--paper)">dropped · {d}</DojoChip>)}
              </div>
            </div>
          )}
          {(c.conflicts > 0 || c.dups > 0) && (
            <div className="flex flex-col gap-2" >
              {c.conflicts > 0 && (
                <div className="bg-warning-soft rounded-lg py-3 px-4" >
                  <div className="flex items-center gap-2 mb-2" >
                    <span className="kanji text-sm text-warning" >衝</span>
                    <span className="text-xs uppercase text-warning font-semibold" style={{ letterSpacing: ".12em" }}>Conflicts with a published rule</span>
                  </div>
                  <div className="bg-paper border border-paper-edge rounded-lg overflow-hidden" >
                    <div className="flex gap-2 py-2 px-3 text-xs" style={{ borderBottom: "1px solid var(--paper-edge)", fontFamily: "var(--font-mono)" }}>
                      <span className="shrink-0 text-warning" style={{ width: 10, fontWeight: 700 }}>−</span>
                      <span className="text-ink-soft" ><span className="text-ink-faint" >Company</span> · “Retry freely on transient errors”</span>
                    </div>
                    <div className="flex gap-2 py-2 px-3 text-xs" style={{ fontFamily: "var(--font-mono)" }}>
                      <span className="shrink-0 text-success" style={{ width: 10, fontWeight: 700 }}>+</span>
                      <span className="text-ink" ><span className="text-ink-faint" >{c.scope}</span> · “{c.title}”</span>
                    </div>
                  </div>
                  <div className="text-xs text-ink-soft mt-2" style={{ lineHeight: 1.5 }}>The more specific scope wins — approving lets <b className="font-semibold" >{c.scope}</b> supersede the Company rule on money-moving paths, leaving it intact everywhere else.</div>
                  <div className="flex gap-2 mt-3" >
                    <button className="py-1 px-3 rounded border border-paper-edge bg-paper text-ink-soft text-xs cursor-pointer" style={{ fontFamily: "inherit" }}>View rule</button>
                    <button className="py-1 px-3 rounded bg-paper text-danger text-xs cursor-pointer" style={{ border: "1px solid var(--danger-edge)", fontFamily: "inherit" }}>Supersede on approve</button>
                  </div>
                </div>
              )}
              {c.dups > 0 && (() => {
                const dupsList = c.dups >= 2
                  ? [{ t: "Persona: drafts integration tests for auth", s: 0.86 }, { t: "Persona: refresh-flow test author", s: 0.81 }]
                  : [{ t: "Never write secrets to stdout in handlers", s: 0.78 }];
                return (
                <div className="bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" >
                  <div className="flex items-center gap-2 mb-2" >
                    <span className="kanji text-sm text-ink-mute" >双</span>
                    <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".12em" }}>{c.dups} near-duplicate{c.dups > 1 ? "s" : ""} · merge suggested</span>
                  </div>
                  {dupsList.map((d, i) => (
                    <div className="flex items-center gap-2 py-2 px-0" key={i} style={{ borderBottom: i < dupsList.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                      <span className="text-sm text-ink flex-1" >{d.t}</span>
                      <span className="mono text-xs" style={{ color: d.s >= 0.9 ? "var(--accent)" : "var(--ink-mute)" }}>sim {d.s.toFixed(2)}</span>
                    </div>
                  ))}
                  <div className="mt-2 text-xs text-ink-soft" style={{ lineHeight: 1.55 }}>
                    <span className="text-ink-mute" >Auto-dedupe · </span>≥&nbsp;0.90 merges automatically; <b className="font-semibold" >0.75–0.90 is flagged here</b> for you to confirm.
                  </div>
                  <div className="flex gap-2 mt-3" >
                    <button className="py-1 px-3 rounded bg-accent-soft text-accent text-xs cursor-pointer" style={{ border: "1px solid var(--accent-edge)", fontFamily: "inherit" }}>Merge into canonical</button>
                    <button className="py-1 px-3 rounded border border-paper-edge bg-paper text-ink-soft text-xs cursor-pointer" style={{ fontFamily: "inherit" }}>Keep separate</button>
                  </div>
                </div>
                );
              })()}
            </div>
          )}
        </div>
        <div className="bg-paper-soft overflow-auto py-6 px-4 flex flex-col gap-4" style={{ borderLeft: mobile ? "none" : "var(--hairline)", borderTop: mobile ? "var(--hairline)" : "none" }}>
          <div className="flex flex-col items-center gap-1" >
            <EnsoRing progress={c.confidence} size={104} stroke={8} color="var(--accent)" label={Math.round(c.confidence * 100)} />
            <span className="text-xs uppercase text-ink-mute" style={{ letterSpacing: ".12em" }}>Confidence</span>
          </div>
          <div>
            <div className="text-xs uppercase text-ink-faint font-semibold mb-1" style={{ letterSpacing: ".14em" }}>Attribution</div>
            <div className="text-sm text-ink-soft" style={{ lineHeight: 1.5 }}>
              {c.origin === "client" ? "Source-anonymized — no contributor or client identity attached." : <>Named to <b className="text-ink font-semibold" >{c.by}</b>, {c.origin === "community" ? "from the community" : "org-internal"}.</>}
            </div>
          </div>
          <div>
            <div className="flex items-center mb-1" >
              <span className="text-xs uppercase text-ink-faint font-semibold" style={{ letterSpacing: ".14em" }}>Distribute to</span>
              <span className="flex-1" />
              <button className="inline-flex items-center gap-1 bg-transparent border-0 cursor-pointer text-xs text-ink-mute" style={{ fontFamily: "var(--font-mono)" }}>preset · team default ▾</button>
            </div>
            <button className="w-full flex items-center gap-2 bg-paper border border-paper-edge rounded py-2 px-3 cursor-pointer text-left" >
              <span className="kanji text-sm text-accent" >{c.scopeKanji}</span>
              <span className="text-sm text-ink flex-1" >{c.scope}</span>
              <span className="mono text-xs text-ink-faint uppercase" style={{ letterSpacing: ".06em" }}>binding</span>
              <span className="text-xs text-ink-mute" >▾</span>
            </button>
            <div className="mt-2 bg-paper border border-paper-edge rounded-lg py-2 px-3" >
              <div className="flex items-center gap-2" >
                <span className="kanji text-xs text-ink-mute" >誰</span>
                <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".1em" }}>Who gets this</span>
                <span className="flex-1" />
                <span className="mono text-xs text-ink-soft" >{r ? `${r.repos} repos · ${r.devs} devs` : "scope not sized yet"}</span>
              </div>
              <button className="mono mt-1 text-xs text-accent border-0 cursor-pointer p-0" onClick={() => setShowRecipients(v => !v)} style={{ background: "none" }}>{showRecipients ? "Hide recipients ↑" : "Preview recipients →"}</button>
              {showRecipients && (
                <div className="mt-2 pt-2" style={{ borderTop: "1px solid var(--paper-edge)" }}>
                  {["ledger-core · 6 devs", "payments-api · 4 devs", "checkout-web · 4 devs"].map(x => (
                    <div className="flex items-center gap-2 py-1 px-0 text-xs text-ink-soft" key={x} >
                      <span className="kanji text-xs text-ink-faint" >庫</span><span className="mono">{x}</span>
                    </div>
                  ))}
                  <div className="text-xs text-ink-faint mt-1" >Dry-run — who receives this on publish, per the binding above.</div>
                </div>
              )}
            </div>
            <div className="text-xs text-ink-faint mt-2" style={{ lineHeight: 1.45 }}>Inherits the contribution's binding. Narrowing is free; broadening beyond it asks you to confirm.</div>
          </div>
          <div className="border-t pt-4 flex flex-col gap-2" style={{ marginTop: "auto",
 position: mobile ? "sticky" : "static", bottom: mobile ? 0 : "auto", background: mobile ? "var(--paper-soft)" : "transparent",
 paddingBottom: mobile ? "var(--space-2)" : 0, marginLeft: mobile ? -20 : 0, marginRight: mobile ? -20 : 0, paddingLeft: mobile ? "var(--space-4)" : 0, paddingRight: mobile ? "var(--space-4)" : 0, boxShadow: mobile ? "var(--shadow-up)" : "none" }}>
            {c.impact === "high" && (
              <div className="bg-accent-soft rounded-lg py-2 px-3" style={{ border: "1px solid var(--accent-edge)" }}>
                <div className="flex items-center gap-2 mb-1" >
                  <span className="kanji text-sm text-accent" >検</span>
                  <span className="text-xs uppercase text-accent font-semibold" style={{ letterSpacing: ".1em" }}>Second approval required</span>
                </div>
                <div className="text-xs text-ink-soft" style={{ lineHeight: 1.45 }}>High-impact items need a second maintainer. Threshold set per scope in Scopes &amp; policies.</div>
                <button className="w-full mt-2 flex items-center gap-2 bg-paper border border-paper-edge rounded py-2 px-2 cursor-pointer text-left" >
                  <Avatar name={secondApprover} size={18} />
                  <span className="text-xs text-ink-soft flex-1" >{secondApprover} · suggested approver</span>
                  <span className="text-xs text-ink-mute" >▾</span>
                </button>
              </div>
            )}
            <button className="w-full p-3 rounded-lg border-0 cursor-pointer bg-ink text-paper text-sm font-medium inline-flex items-center justify-center gap-2" style={{ fontFamily: "inherit" }}>
              <span className="kanji text-sm text-accent" >決</span> {c.impact === "high" ? "Approve & request 2nd" : "Approve & publish"}
            </button>
            <div className="flex gap-2" >
              <button className="flex-1 p-2 rounded-lg border border-paper-edge cursor-pointer bg-paper text-ink-soft text-sm" onClick={() => setRevising(true)} style={{ fontFamily: "inherit" }}>Revise</button>
              <button className="flex-1 p-2 rounded-lg cursor-pointer bg-danger-soft text-danger text-sm" style={{ border: "1px solid var(--danger-edge)", fontFamily: "inherit" }}>Decline</button>
            </div>
            <div className="text-xs text-ink-faint mt-1 text-center" >{c.impact === "high" ? "Two named approvals, with notes, land in the audit trail." : "A named decision, with a note, lands in the audit trail."}</div>
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
    <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4" >{children}</div>
  );
  const Row = ({ it, tone }) => (
    <div className="grid gap-3 items-center py-3 px-4" style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: "1px solid var(--paper-edge)" }}>
      <span className="kanji text-lg text-center" style={{ color: tone || "var(--accent)", width: 20 }}>{it.k}</span>
      <div className="min-w-0" >
        <div className="text-sm text-ink" >{it.title}</div>
        <div className="flex gap-2 mt-1 items-center flex-wrap" >
          <span className="mono text-xs text-ink-mute" >{it.scope}</span>
          {it.used && <span className="mono text-xs text-ink-faint" >· {it.used}</span>}
          {it.reason && <span className="text-xs text-ink-mute" >· {it.reason}</span>}
        </div>
      </div>
      {it.age && <span className="mono text-xs text-ink-faint" >{it.age}</span>}
      {it.left != null && (
        <span className="mono text-xs" style={{ color: it.left <= 7 ? "var(--accent)" : "var(--ink-mute)" }}>evicted in {it.left}d</span>
      )}
    </div>
  );
  return (
    <div className="h-full overflow-auto" >
      <div className="flex items-start gap-4 border-b" style={{ padding: "var(--space-6) var(--space-6) var(--space-4)" }}>
        <span className="kanji text-3xl text-accent shrink-0" style={{ lineHeight: 1 }}>蔵</span>
        <div className="flex-1 min-w-0" >
          <div className="text-xs uppercase text-ink-mute mb-1" style={{ letterSpacing: ".18em" }}>Govern · published library</div>
          <h1 className="display text-xl font-normal m-0" style={{ letterSpacing: "-0.015em", lineHeight: 1.05 }}>Knowledge</h1>
          <p className="text-sm text-ink-soft" style={{ lineHeight: 1.55, margin: "var(--space-1) 0 0", maxWidth: 720 }}>
            The Dōjō holds only active, derived intelligence — guards, patterns, principles, skills and personas. As practice
            moves on, a maintainer disables what's gone redundant; disabled knowledge is then evicted automatically.
          </p>
        </div>
      </div>
      <div className="p-6" >
        <HeadCard>
          <div className="flex items-center gap-3" >
            <span className="kanji text-lg text-accent" >時</span>
            <div className="flex-1 min-w-0" >
              <div className="text-sm text-ink" >Prune disabled knowledge</div>
              <div className="text-xs text-ink-mute mt-1" style={{ lineHeight: 1.5 }}>
                How long an item stays recoverable after a maintainer disables it. After the window it's evicted from the library for good.
              </div>
            </div>
            <select className="text-sm border border-paper-edge rounded-sm bg-paper text-ink cursor-pointer py-1 px-2" value={pruneDays} onChange={e => setPruneDays(e.target.value)}
 style={{ fontFamily: "inherit" }}>
              <option value="7">7 days</option>
              <option value="30">30 days · default</option>
              <option value="90">90 days</option>
              <option value="never">Never · keep disabled</option>
            </select>
          </div>
        </HeadCard>
        <div className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em", margin: "var(--space-6) 0 var(--space-2)" }}>
          Active · {active.length}
        </div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {active.map((it, i) => <Row key={i} it={it}/>)}
        </div>
        <div className="text-xs uppercase text-ink-mute font-semibold flex items-center gap-2" style={{ letterSpacing: ".14em", margin: "var(--space-6) 0 var(--space-2)" }}>
          Disabled · pending pruning
          <span className="mono text-xs text-ink-faint normal-case" style={{ letterSpacing: 0 }}>
            {pruneDays === "never" ? "retained — pruning off" : `evicting ${pruneDays}d after disable`}
          </span>
        </div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" style={{ opacity: 0.92 }}>
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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="承" eyebrow="Govern · second approval" title="Awaiting your approval"
        sub="High-impact teachings a first maintainer approved and routed to you for a second sign-off. Two named approvals — with notes — land in the audit trail before anything publishes."
        right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">{rows.length} waiting</DojoChip>} />
      <div className="flex-1 overflow-auto p-6" >
        {rows.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-3 py-16 px-0 text-ink-mute" >
            <span className="kanji text-3xl text-ink-faint" >空</span>
            <div className="text-sm text-ink-soft" >Nothing awaiting your second approval.</div>
          </div>
        ) : (
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" style={{ maxWidth: 900 }}>
            {rows.map((c, i) => (
              <div className="grid gap-4 items-center py-4 px-4" key={c.id} style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: i < rows.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{c.kanji}</span>
                <div className="min-w-0" >
                  <div className="text-sm text-ink" >{c.title}</div>
                  <div className="flex gap-2 mt-1 items-center flex-wrap" >
                    <DojoChip tone="var(--accent)" soft="var(--accent-soft)">high impact</DojoChip>
                    <span className="mono text-xs text-ink-faint" >{c.scopeKanji} {c.scope}</span>
                    <span className="inline-flex items-center gap-1 text-xs text-ink-mute" >
                      <Avatar name={c.first} size={16} /> 1st · {c.first} · {c.when}
                    </span>
                  </div>
                </div>
                <div className="flex gap-2" >
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
function DojoMaintainerConsole({ initial = "triage", initialCandidate = null, mobile = false, relayStart = null, onExit, enteredOrg }) {
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
      nav={MAINT_NAV} active={candidate ? "triage" : section} setActive={(s) => go(s)} mobile={mobile} relayStart={relayStart} zone="dojo" onExit={onExit} orgOverride={enteredOrg}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoMaintainerConsole, DojoTriage, DojoCandidate, DojoKnowledge });
