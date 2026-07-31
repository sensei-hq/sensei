// Dōjō console — the company-hosted governance site (web).
// Shell (top bar + left nav) + screens: Overview · Triage · Candidate ·
// Members · Clients. Built on the shared zen-sumi primitives so it reads as
// the same family as the Observatory, but as a browser app, not the desktop.
//
// Exports (window): DojoConsole (interactive shell+screens), and the screen
// components for standalone artboards.

const { useState: dS } = React;

/* ─── small shared bits ─────────────────────────────────── */

// Origin → how a contribution is attributed (drives the dereferencing model).
const DOJO_ORIGIN = {
  employer:  { label: "Employer",   tone: "var(--ink-2)",  soft: "var(--paper-3)" },
  client:    { label: "Dereferenced", tone: "var(--accent)", soft: "var(--accent-soft)" },
  community: { label: "Community",  tone: "var(--success)", soft: "var(--success-soft)" },
  oss:       { label: "Open source",tone: "var(--ink-2)",  soft: "var(--paper-3)" },
};
const DOJO_TYPE = {
  guard: "Guard", pattern: "Pattern", principle: "Principle",
  prompt: "Prompt", skill: "Skill", agent: "Agent",
};

function DojoChip({ children, tone = "var(--ink-3)", soft = "var(--paper-3)", border }) {
  return (
    <span className="mono inline-flex items-center whitespace-nowrap" style={{
 fontSize: 10, letterSpacing: ".04em", color: tone, background: soft,
 border: border || "1px solid transparent", borderRadius: 20,
 padding: "2px 8px", gap: 5 }}>{children}</span>
  );
}

function OriginChip({ origin }) {
  const o = DOJO_ORIGIN[origin] || DOJO_ORIGIN.employer;
  return <DojoChip tone={o.tone} soft={o.soft}>{origin === "client" && "盾 "}{o.label}</DojoChip>;
}

// Confidence as a slim bar + value.
function Confidence({ v, w = 84 }) {
  const tone = v >= 0.85 ? "var(--success)" : v >= 0.7 ? "var(--accent)" : "var(--warning)";
  return (
    <div className="flex items-center" style={{ gap: 8 }}>
      <div className="bg-paper-3 overflow-hidden" style={{ width: w, height: 4, borderRadius: 2 }}>
        <div className="h-full" style={{ width: (v * 100) + "%", background: tone, borderRadius: 2 }} />
      </div>
      <span className="mono text-ink-2" style={{ fontSize: 11 }}>{Math.round(v * 100)}</span>
    </div>
  );
}

/* ─── shell ──────────────────────────────────────────────── */

const DOJO_NAV = [
  { group: "Govern", items: [
    { id: "overview", kanji: "全", label: "Overview" },
    { id: "triage",   kanji: "門", label: "Triage", badge: 7 },
    { id: "knowledge",kanji: "蔵", label: "Knowledge" },
  ]},
  { group: "Org", items: [
    { id: "members", kanji: "任", label: "Members & roles" },
    { id: "scopes",  kanji: "規", label: "Scopes & policies" },
    { id: "clients", kanji: "守", label: "Clients" },
  ]},
  { group: "Trust", items: [
    { id: "monitor", kanji: "観", label: "Monitor" },
    { id: "audit",   kanji: "録", label: "Audit trail" },
  ]},
];
const DOJO_BUILT = new Set(["overview", "triage", "knowledge", "members", "clients", "scopes", "audit", "monitor"]);

function DojoTopBar({ org, onSwitch }) {
  const D = window.DOJO;
  return (
    <div className="shrink-0 flex items-center border-b bg-paper" style={{
 height: 54,
 gap: 16, padding: "0 18px" }}>
      <div className="flex items-baseline" style={{ gap: 9 }}>
        <span className="kanji text-accent" style={{ fontSize: 22, lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: 18, letterSpacing: "-0.01em" }}>Dōjō</span>
      </div>
      {/* org switcher — the multi-membership model */}
      <button className="inline-flex items-center bg-paper-2 border border-paper-edge cursor-pointer" style={{ gap: 8, marginLeft: 6, borderRadius: 7,
 padding: "6px 11px" }}>
        <span className="kanji text-accent" style={{ fontSize: 13 }}>{org.kanji}</span>
        <span className="text-ink" style={{ fontSize: 13 }}>{org.name}</span>
        <span className="mono text-ink-4 uppercase" style={{ fontSize: 9, letterSpacing: ".08em" }}>employer</span>
        <span className="text-ink-3" style={{ fontSize: 9 }}>▾</span>
      </button>
      <div className="flex-1" />
      <div className="flex items-center bg-paper-2 border border-paper-edge" style={{ gap: 8, borderRadius: 7, padding: "6px 11px", width: 260 }}>
        <span className="kanji text-ink-3" style={{ fontSize: 12 }}>探</span>
        <span className="text-ink-4" style={{ fontSize: 12 }}>search knowledge…</span>
      </div>
      <span className="text-ink-3" style={{ fontSize: 11, fontFamily: "var(--font-mono)" }}>
        {D.org.members} members
      </span>
      <Avatar name="Keiko" size={28} />
      <button className="inline-flex items-center bg-transparent border border-paper-edge cursor-pointer text-ink-2" title="Log out" style={{ gap: 6, borderRadius: 7,
 padding: "6px 11px",
 fontFamily: "inherit", fontSize: 12 }}>
        <span className="kanji text-ink-3" style={{ fontSize: 12 }}>出</span>
        Log out
      </button>
    </div>
  );
}

function DojoNav({ active, setActive }) {
  return (
    <aside className="shrink-0 border-r bg-paper-2 flex flex-col overflow-auto" style={{
 width: 218, padding: "16px 12px" }}>
      {DOJO_NAV.map(grp => (
        <div key={grp.group} style={{ marginBottom: 14 }}>
          <div className="uppercase text-ink-4 font-semibold" style={{ fontSize: 9.5, letterSpacing: ".14em", padding: "0 8px", marginBottom: 6 }}>{grp.group}</div>
          <div className="flex flex-col" style={{ gap: 2 }}>
            {grp.items.map(it => {
              const on = active === it.id;
              const built = DOJO_BUILT.has(it.id);
              return (
                <button className="grid items-center w-full text-left" key={it.id} onClick={() => built && setActive(it.id)}
 title={built ? "" : "Designed in a later pass"}
 style={{ gridTemplateColumns: "auto 1fr auto",
 gap: 9, borderRadius: 7, padding: "8px 9px",
 background: on ? "var(--paper)" : "transparent",
 border: on ? "var(--hairline)" : "1px solid transparent",
 color: on ? "var(--ink)" : built ? "var(--ink-2)" : "var(--ink-4)",
 cursor: built ? "pointer" : "default", fontSize: 13, opacity: built ? 1 : 0.6 }}>
                  <span className="kanji text-center" style={{ fontSize: 13, width: 15,
 color: on ? "var(--accent)" : "var(--ink-3)" }}>{it.kanji}</span>
                  <span>{it.label}</span>
                  {it.badge != null
                    ? <span className="mono font-semibold text-paper bg-accent" style={{ fontSize: 10, borderRadius: 10, padding: "0 6px", lineHeight: "16px" }}>{it.badge}</span>
                    : !built ? <span className="text-ink-4" style={{ fontSize: 8.5, letterSpacing: ".06em" }}>soon</span> : <span/>}
                </button>
              );
            })}
          </div>
        </div>
      ))}
      <div className="flex-1" />
      <button className="grid items-center w-full text-left bg-transparent text-ink-3 cursor-default border-t" style={{ gridTemplateColumns: "auto 1fr", gap: 9, borderRadius: 7, padding: "8px 9px", fontSize: 13, borderRadius: 0, paddingTop: 12, opacity: 0.6 }}>
        <span className="kanji text-center text-ink-3" style={{ fontSize: 13, width: 15 }}>調</span>
        <span>Settings · SSO</span>
      </button>
    </aside>
  );
}

// Section header band inside the main column.
function DojoHead({ kanji, eyebrow, title, sub, right }) {
  return (
    <div className="flex items-start border-b shrink-0" style={{ gap: 18, padding: "22px 28px 18px" }}>
      <span className="kanji text-accent shrink-0" style={{ fontSize: 38, lineHeight: 1 }}>{kanji}</span>
      <div className="flex-1 min-w-0" >
        <div className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".18em", marginBottom: 4 }}>{eyebrow}</div>
        <h1 className="display font-normal m-0" style={{ fontSize: 24, letterSpacing: "-0.015em", lineHeight: 1.05 }}>{title}</h1>
        {sub && <p className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.55, margin: "6px 0 0", maxWidth: 680 }}>{sub}</p>}
      </div>
      {right && <div className="shrink-0" >{right}</div>}
    </div>
  );
}

/* ─── Overview ───────────────────────────────────────────── */

function DojoOverview({ go }) {
  const D = window.DOJO, m = D.metrics;
  // Resolution (Publish & measure): published teachings carry a lifecycle and an
  // impact reading; negative impact is flagged and offers one-click retract.
  const published = [
    { kanji: "守", title: "Never log refresh tokens", scope: "Company", adoption: 0.92, delta: 6, status: "active" },
    { kanji: "紋", title: "Idempotency key on money-moving mutations", scope: "Team · Payments", adoption: 0.78, delta: 9, status: "active" },
    { kanji: "問", title: "Prefer optimistic UI for list mutations", scope: "Stack · React", adoption: 0.41, delta: -3, status: "flagged" },
  ];
  const Metric = ({ kanji, label, value, sub, children, onClick }) => (
    <div className="flex-1 bg-paper-2 border border-paper-edge min-w-0" onClick={onClick} style={{ borderRadius: 12,
 padding: "16px 18px", cursor: onClick ? "pointer" : "default" }}>
      <div className="flex items-center" style={{ gap: 7, marginBottom: 10 }}>
        <span className="kanji text-accent" style={{ fontSize: 14 }}>{kanji}</span>
        <span className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".12em" }}>{label}</span>
      </div>
      <div className="flex items-end justify-between" style={{ gap: 10 }}>
        <div className="display font-light text-ink" style={{ fontSize: 34, lineHeight: 1 }}>{value}</div>
        {children}
      </div>
      {sub && <div className="text-ink-3" style={{ fontSize: 11, marginTop: 8 }}>{sub}</div>}
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="全" eyebrow="Acme Corp · Dōjō" title="The shared mind, governed."
        sub="What your org has learned — triaged, approved, and routed to the scopes that need it."
        right={<div className="text-right text-ink-3" style={{ fontSize: 11, fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div>{D.org.scopes} scopes · {D.org.repos} repos</div>
          <div className="text-success" >{m.incidents} confidentiality incidents</div>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: 28 }}>
        {/* metric row */}
        <div className="flex" style={{ gap: 14 }}>
          <Metric kanji="門" label="Pending triage" value={m.pendingTriage}
            sub="across 4 scopes · oldest 3d" onClick={() => go("triage")}>
            <span className="mono text-accent" style={{ fontSize: 11 }}>review →</span>
          </Metric>
          <Metric kanji="共" label="Contributions · 7d" value={m.contribWeek}>
            <Sparkline data={m.contribSpark} width={92} height={30} color="var(--accent)" fill="var(--accent-soft)" />
          </Metric>
          <Metric kanji="決" label="Approved · 7d" value={m.approvedWeek} sub="published to matching scopes" />
          <Metric kanji="果" label="Adoption lift" value={"+" + Math.round(m.adoptionLift * 100) + "pp"} sub="FTR across adopting scopes">
            <Sparkline data={m.ftrSpark} width={92} height={30} color="var(--success)" />
          </Metric>
        </div>

        {/* two columns */}
        <div className="grid" style={{ gridTemplateColumns: "1.4fr 1fr", gap: 18, marginTop: 18 }}>
          {/* triage preview */}
          <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
            <div className="flex items-center border-b" style={{ padding: "13px 16px" }}>
              <span className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".14em" }}>Top of the triage queue</span>
              <span className="flex-1" />
              <button onClick={() => go("triage")} className="mono text-accent border-0 cursor-pointer" style={{ fontSize: 11, background: "none" }}>all 7 →</button>
            </div>
            {D.queue.slice(0, 4).map((c, i) => (
              <button className="grid items-center w-full text-left cursor-pointer bg-transparent border-0" key={c.id} onClick={() => go("triage", c.id)} style={{ gridTemplateColumns: "auto 1fr auto", gap: 12, padding: "12px 16px",
 borderBottom: i < 3 ? "1px solid var(--edge)" : "none" }}>
                <span className="kanji text-accent text-center" style={{ fontSize: 17, width: 20 }}>{c.kanji}</span>
                <div className="min-w-0" >
                  <div className="text-ink whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 13 }}>{c.title}</div>
                  <div className="flex items-center" style={{ gap: 7, marginTop: 4 }}>
                    <span className="mono text-ink-3" style={{ fontSize: 10 }}>{c.scope}</span>
                    <OriginChip origin={c.origin} />
                  </div>
                </div>
                <Confidence v={c.confidence} w={56} />
              </button>
            ))}
          </div>

          {/* activity + leak guard */}
          <div className="flex flex-col" style={{ gap: 18 }}>
            <div className="bg-paper-2" style={{ border: "1px solid var(--success-edge)", borderRadius: 12, padding: "15px 17px" }}>
              <div className="flex items-center" style={{ gap: 8, marginBottom: 8 }}>
                <span className="kanji text-success" style={{ fontSize: 15 }}>盾</span>
                <span className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".14em" }}>Confidentiality</span>
              </div>
              <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.55 }}>
                <b className="font-semibold" >{m.dereferenced}</b> client lessons auto-dereferenced this week ·
                <span className="text-success" > 0 incidents</span>. Sources dropped automatically; only flagged exceptions reach a lead.
              </div>
              <div className="flex items-center" style={{ gap: 7, marginTop: 10, paddingTop: 10, borderTop: "1px solid var(--edge)" }}>
                <span className="rounded-full bg-success" style={{ width: 6, height: 6 }} />
                <span className="mono text-ink-3 uppercase" style={{ fontSize: 10.5, letterSpacing: ".06em" }}>Leak-guard armed</span>
                <span className="flex-1" />
                <span className="text-ink-3" style={{ fontSize: 11 }}>alerts shared to the client lead <span className="text-accent" >→</span></span>
              </div>
            </div>
            <div className="bg-paper-2 border border-paper-edge overflow-hidden flex-1" style={{ borderRadius: 12 }}>
              <div className="border-b uppercase text-ink-3" style={{ padding: "13px 16px", fontSize: 11, letterSpacing: ".14em" }}>Recent activity</div>
              <div style={{ padding: "4px 0" }}>
                {D.activity.map((a, i) => (
                  <div className="grid items-start" key={i} style={{ gridTemplateColumns: "auto 1fr auto", gap: 10, padding: "9px 16px" }}>
                    <span className="kanji text-center" style={{ fontSize: 13, width: 16,
 color: a.tone === "success" ? "var(--success)" : a.tone === "accent" ? "var(--accent)" : "var(--ink-3)" }}>{a.kanji}</span>
                    <span className="text-ink-2" style={{ fontSize: 12, lineHeight: 1.45 }}>{a.text}</span>
                    <span className="mono text-ink-4" style={{ fontSize: 10 }}>{a.when}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Resolution (Publish & measure): the Impact loop — lifecycle + retract */}
        <div style={{ marginTop: 18 }}>
          <div className="flex items-center" style={{ marginBottom: 9 }}>
            <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".14em" }}>Published · adoption &amp; health</span>
            <span className="mono text-ink-4" style={{ fontSize: 11, marginLeft: 10 }}>the Impact loop, scoped to the org</span>
          </div>
          <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
            {published.map((p, i) => {
              const neg = p.delta < 0;
              return (
              <div className="grid items-center" key={p.title} style={{ gridTemplateColumns: "auto 1fr 150px 92px 168px", gap: 14,
 padding: "13px 16px", borderBottom: i < published.length - 1 ? "1px solid var(--edge)" : "none",
 background: neg ? "var(--warning-soft)" : "transparent" }}>
                <span className="kanji text-center" style={{ fontSize: 17, color: neg ? "oklch(0.52 0.13 60)" : "var(--accent)", width: 20 }}>{p.kanji}</span>
                <div className="min-w-0" >
                  <div className="text-ink whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 13 }}>{p.title}</div>
                  <div className="mono text-ink-4" style={{ fontSize: 10, marginTop: 3 }}>{p.scope}</div>
                </div>
                {/* adoption */}
                <div className="flex items-center" style={{ gap: 8 }}>
                  <div className="flex-1 bg-paper-3 overflow-hidden" style={{ height: 4, borderRadius: 2 }}>
                    <div className="h-full bg-accent" style={{ width: (p.adoption * 100) + "%", borderRadius: 2 }} />
                  </div>
                  <span className="mono text-ink-3" style={{ fontSize: 11 }}>{Math.round(p.adoption * 100)}%</span>
                </div>
                {/* impact delta */}
                <span className="mono text-right" style={{ fontSize: 12, color: neg ? "oklch(0.52 0.13 60)" : "var(--success)" }}>
                  {neg ? "" : "+"}{p.delta}pp FTR
                </span>
                {/* lifecycle / action */}
                <div className="flex justify-end" >
                  {neg
                    ? <button className="inline-flex items-center bg-paper cursor-pointer" style={{ gap: 6, padding: "6px 12px", borderRadius: 7,
 border: "1px solid oklch(0.52 0.13 60/.4)", color: "oklch(0.5 0.13 60)", fontSize: 12, fontFamily: "inherit" }}>
                        <span className="kanji" style={{ fontSize: 12 }}>退</span> Retract downstream
                      </button>
                    : <DojoChip tone="var(--success)" soft="var(--success-soft)">active</DojoChip>}
                </div>
              </div>
              );
            })}
            <div className="flex items-center text-ink-3" style={{ gap: 7, padding: "10px 16px", borderTop: "1px solid var(--edge)", fontSize: 11.5, lineHeight: 1.45 }}>
              <span className="kanji" style={{ fontSize: 12, color: "oklch(0.52 0.13 60)" }}>退</span>
              <span>Lifecycle <b className="font-semibold text-ink-2" >active → deprecated → retracted</b>. Negative impact is flagged automatically; one-click retract pulls a teaching back and notifies adopters.</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Triage queue ───────────────────────────────────────── */

// Resolution (Open the queue): every scope has a named owner, set in Scopes &
// policies; anything unowned routes to a fallback maintainer so nothing sits idle.
const SCOPE_OWNERS = {
  "Company":         "Keiko T.",
  "Team · Payments": "Marco D.",
  "Client · Globex": "Sven K.",
  "Stack · React":   "Sven K.",
  // "Stack · Postgres" intentionally has no owner → falls back
};
const SCOPE_FALLBACK = "Keiko T.";
const IMPACT_RANK = { high: 0, med: 1, low: 2 };
function ageHours(a) { const n = parseFloat(a) || 0; return /d/.test(a) ? n * 24 : /m/.test(a) ? n / 60 : n; }
function rankCandidates(arr) {
  return [...arr].sort((x, y) => (IMPACT_RANK[x.impact] - IMPACT_RANK[y.impact]) || (ageHours(x.age) - ageHours(y.age)));
}

function DojoTriage({ go }) {
  const D = window.DOJO;
  // group by scope
  const groups = {};
  D.queue.forEach(c => { (groups[c.scope] ||= []).push(c); });
  // Resolution (Open the queue): rank groups by their strongest candidate
  // (projected impact, then age), and rank rows within each group the same way.
  const ordered = Object.entries(groups)
    .map(([scope, items]) => [scope, rankCandidates(items)])
    .sort((a, b) => (IMPACT_RANK[a[1][0].impact] - IMPACT_RANK[b[1][0].impact]) || (ageHours(a[1][0].age) - ageHours(b[1][0].age)));
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="門" eyebrow="Govern · triage" title="What's waiting for review"
        sub="Your scopes by default, ranked by projected impact then age. Every scope has an owner — anything unowned routes to a fallback so nothing sits idle. Nothing publishes without a decision."
        right={<div className="flex items-center" style={{ gap: 8 }}>
          <span className="inline-flex items-center text-accent bg-accent-soft" style={{ gap: 5, fontSize: 11, fontFamily: "var(--font-mono)", border: "1px solid oklch(0.58 0.15 35/.28)",
 borderRadius: 20, padding: "3px 10px" }}>✓ My scopes</span>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Sort · impact, then age ▾</DojoChip>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: "8px 28px 28px" }}>
        {ordered.map(([scope, items]) => {
          const owner = SCOPE_OWNERS[scope];
          return (
          <div key={scope} style={{ marginTop: 18 }}>
            <div className="flex items-center" style={{ gap: 8, marginBottom: 8 }}>
              <span className="kanji text-ink-3" style={{ fontSize: 13 }}>{items[0].scopeKanji}</span>
              <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".12em" }}>{scope}</span>
              <span className="mono text-ink-4" style={{ fontSize: 11 }}>{items.length}</span>
              <span className="flex-1" />
              {owner
                ? <span className="inline-flex items-center text-ink-3" style={{ gap: 6, fontSize: 11 }}>
                    <Avatar name={owner} size={18} />
                    <span>owner · <span className="text-ink-2" >{owner}</span></span>
                  </span>
                : <span className="inline-flex items-center text-ink-4" style={{ gap: 6, fontSize: 11 }}>
                    <span className="rounded-full" style={{ width: 7, height: 7, border: "1px dashed var(--ink-4)" }} />
                    <span>unowned → <span className="text-ink-3" >{SCOPE_FALLBACK}</span> · fallback</span>
                  </span>}
            </div>
            <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
              {items.map((c, i) => (
                <button className="grid items-center w-full text-left cursor-pointer bg-transparent border-0" key={c.id} onClick={() => go("triage", c.id)} style={{ gridTemplateColumns: "auto 1fr auto auto auto", gap: 14, padding: "13px 16px", borderBottom: i < items.length - 1 ? "1px solid var(--edge)" : "none" }}>
                  <span className="kanji text-accent text-center" style={{ fontSize: 18, width: 22 }}>{c.kanji}</span>
                  <div className="min-w-0" >
                    <div className="text-ink" style={{ fontSize: 13.5 }}>{c.title}</div>
                    <div className="flex items-center flex-wrap" style={{ gap: 8, marginTop: 5 }}>
                      <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
                      <OriginChip origin={c.origin} />
                      <span className="mono text-ink-4" style={{ fontSize: 10 }}>
                        {c.origin === "client" ? "source dropped" : c.by} · {c.evidence} sessions · {c.age}
                      </span>
                    </div>
                  </div>
                  {/* signals */}
                  <div className="flex items-center justify-end" style={{ gap: 10, minWidth: 96 }}>
                    {c.conflicts > 0 && <DojoChip tone="oklch(0.52 0.13 60)" soft="var(--warning-soft)">{c.conflicts} conflict</DojoChip>}
                    {c.dups > 0 && <DojoChip>{c.dups} dup</DojoChip>}
                  </div>
                  <Confidence v={c.confidence} />
                  <span className="text-ink-4" style={{ fontSize: 13 }}>→</span>
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
  // Resolution (Set distribution): dry-run reach per scope for 'who gets this'.
  const REACH = { "Company": { repos: 134, devs: 48 }, "Team · Payments": { repos: 6, devs: 14 },
    "Client · Globex": { repos: 3, devs: 7 }, "Stack · React": { repos: 22, devs: 19 }, "Stack · Postgres": { repos: 17, devs: 12 } };
  const r = REACH[c.scope] || { repos: 4, devs: 8 };
  const Block = ({ kanji, label, children }) => (
    <div style={{ marginBottom: 18 }}>
      <div className="flex items-center" style={{ gap: 7, marginBottom: 6 }}>
        <span className="kanji text-accent" style={{ fontSize: 13 }}>{kanji}</span>
        <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 10.5, letterSpacing: ".14em" }}>{label}</span>
      </div>
      <div className="text-ink" style={{ fontSize: 13.5, lineHeight: 1.6 }}>{children}</div>
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      {/* breadcrumb */}
      <div className="flex items-center border-b shrink-0" style={{ gap: 8, padding: "12px 28px" }}>
        <button onClick={() => go("triage")} className="mono text-accent border-0 cursor-pointer" style={{ fontSize: 11, background: "none" }}>← Triage</button>
        <span className="text-ink-4" style={{ fontSize: 11 }}>/</span>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{c.scope}</span>
      </div>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: "1fr 332px" }}>
        {/* main */}
        <div className="overflow-auto" style={{ padding: "24px 28px 32px" }}>
          <div className="flex flex-wrap" style={{ gap: 9, marginBottom: 12 }}>
            <DojoChip>{DOJO_TYPE[c.type]}</DojoChip>
            <DojoChip tone="var(--ink-2)">{c.scopeKanji} {c.scope}</DojoChip>
            <OriginChip origin={c.origin} />
            <DojoChip tone={c.impact === "high" ? "var(--accent)" : "var(--ink-3)"}>{c.impact} impact</DojoChip>
          </div>
          <h1 className="display font-light text-ink" style={{ fontSize: 27, letterSpacing: "-0.015em", lineHeight: 1.18, margin: "0 0 20px" }}>{c.title}</h1>

          <Block kanji="芽" label="The learning">{c.learning}</Block>
          <Block kanji="因" label="The cause">{c.cause}</Block>
          <Block kanji="周" label="The context — where it applies">{c.context}</Block>

          {/* evidence */}
          <div className="bg-paper-2 border border-paper-edge" style={{ borderRadius: 10, padding: "13px 16px", marginBottom: 16 }}>
            <div className="flex items-center" style={{ gap: 8 }}>
              <span className="kanji text-ink-3" style={{ fontSize: 13 }}>証</span>
              <span className="text-ink" style={{ fontSize: 13 }}><b className="font-semibold" >{c.evidence} sessions</b> support this</span>
              <span className="flex-1" />
              <button className="mono text-accent border-0 cursor-pointer" style={{ fontSize: 11, background: "none" }}>view evidence →</button>
            </div>
          </div>

          {/* dereferencing note for client work */}
          {c.origin === "client" && (
            <div className="bg-accent-soft" style={{ border: "1px solid oklch(0.58 0.15 35/.25)", borderRadius: 10, padding: "13px 16px", marginBottom: 16 }}>
              <div className="flex items-center" style={{ gap: 8, marginBottom: 6 }}>
                <span className="kanji text-accent" style={{ fontSize: 14 }}>盾</span>
                <span className="uppercase text-accent font-semibold" style={{ fontSize: 11, letterSpacing: ".12em" }}>Source dereferenced automatically</span>
              </div>
              <div className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.55 }}>
                The lesson, its cause and context are kept; the source reference was dropped before it reached you — so this can be published anywhere safely.
              </div>
              <div className="flex flex-wrap" style={{ gap: 6, marginTop: 9 }}>
                {c.dereferenced.map(d => <DojoChip key={d} tone="var(--ink-3)" soft="var(--paper)">dropped · {d}</DojoChip>)}
              </div>
            </div>
          )}

          {/* conflicts / dups — Resolution (Evaluate): always surface a conflict
              DIFF against the rule it would supersede, and a merge suggestion for
              near-duplicates with the auto-dedupe band made explicit. */}
          {(c.conflicts > 0 || c.dups > 0) && (
            <div className="flex flex-col" style={{ gap: 10 }}>
              {c.conflicts > 0 && (
                <div className="bg-warning-soft" style={{ borderRadius: 10, padding: "13px 15px" }}>
                  <div className="flex items-center" style={{ gap: 7, marginBottom: 9 }}>
                    <span className="kanji" style={{ fontSize: 13, color: "oklch(0.52 0.13 60)" }}>衝</span>
                    <span className="uppercase font-semibold" style={{ fontSize: 11, letterSpacing: ".12em", color: "oklch(0.52 0.13 60)" }}>Conflicts with a published rule</span>
                  </div>
                  <div className="bg-paper border border-paper-edge overflow-hidden" style={{ borderRadius: 8 }}>
                    <div className="flex" style={{ gap: 9, padding: "8px 12px", borderBottom: "1px solid var(--edge)", fontFamily: "var(--font-mono)", fontSize: 11.5 }}>
                      <span className="shrink-0" style={{ width: 10, color: "oklch(0.52 0.13 60)", fontWeight: 700 }}>−</span>
                      <span className="text-ink-2" ><span className="text-ink-4" >Company</span> · “Retry freely on transient errors”</span>
                    </div>
                    <div className="flex" style={{ gap: 9, padding: "8px 12px", fontFamily: "var(--font-mono)", fontSize: 11.5 }}>
                      <span className="shrink-0 text-success" style={{ width: 10, fontWeight: 700 }}>+</span>
                      <span className="text-ink" ><span className="text-ink-4" >{c.scope}</span> · “{c.title}”</span>
                    </div>
                  </div>
                  <div className="text-ink-2" style={{ fontSize: 12, lineHeight: 1.5, marginTop: 9 }}>The more specific scope wins — approving lets <b className="font-semibold" >{c.scope}</b> supersede the Company rule on money-moving paths, leaving it intact everywhere else.</div>
                  <div className="flex" style={{ gap: 8, marginTop: 11 }}>
                    <button className="border border-paper-edge bg-paper text-ink-2 cursor-pointer" style={{ padding: "6px 12px", borderRadius: 7, fontSize: 12, fontFamily: "inherit" }}>View rule</button>
                    <button className="bg-paper cursor-pointer" style={{ padding: "6px 12px", borderRadius: 7, border: "1px solid oklch(0.52 0.13 60/.4)", color: "oklch(0.5 0.13 60)", fontSize: 12, fontFamily: "inherit" }}>Supersede on approve</button>
                  </div>
                </div>
              )}
              {c.dups > 0 && (() => {
                const dupsList = c.dups >= 2
                  ? [{ t: "Persona: drafts integration tests for auth", s: 0.86 }, { t: "Persona: refresh-flow test author", s: 0.81 }]
                  : [{ t: "Never write secrets to stdout in handlers", s: 0.78 }];
                return (
                <div className="bg-paper-2 border border-paper-edge" style={{ borderRadius: 10, padding: "13px 15px" }}>
                  <div className="flex items-center" style={{ gap: 7, marginBottom: 9 }}>
                    <span className="kanji text-ink-3" style={{ fontSize: 13 }}>双</span>
                    <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".12em" }}>{c.dups} near-duplicate{c.dups > 1 ? "s" : ""} · merge suggested</span>
                  </div>
                  {dupsList.map((d, i) => (
                    <div className="flex items-center" key={i} style={{ gap: 10, padding: "7px 0", borderBottom: i < dupsList.length - 1 ? "1px solid var(--edge)" : "none" }}>
                      <span className="text-ink flex-1" style={{ fontSize: 12.5 }}>{d.t}</span>
                      <span className="mono" style={{ fontSize: 11, color: d.s >= 0.9 ? "var(--accent)" : "var(--ink-3)" }}>sim {d.s.toFixed(2)}</span>
                    </div>
                  ))}
                  <div className="text-ink-2" style={{ marginTop: 10, fontSize: 11.5, lineHeight: 1.55 }}>
                    <span className="text-ink-3" >Auto-dedupe · </span>≥&nbsp;0.90 merges automatically; <b className="font-semibold" >0.75–0.90 is flagged here</b> for you to confirm.
                  </div>
                  <div className="flex" style={{ gap: 8, marginTop: 11 }}>
                    <button className="bg-accent-soft text-accent cursor-pointer" style={{ padding: "6px 12px", borderRadius: 7, border: "1px solid oklch(0.58 0.15 35/.4)", fontSize: 12, fontFamily: "inherit" }}>Merge into canonical</button>
                    <button className="border border-paper-edge bg-paper text-ink-2 cursor-pointer" style={{ padding: "6px 12px", borderRadius: 7, fontSize: 12, fontFamily: "inherit" }}>Keep separate</button>
                  </div>
                </div>
                );
              })()}
            </div>
          )}
        </div>

        {/* decide rail */}
        <div className="border-l bg-paper-2 overflow-auto flex flex-col" style={{ padding: "22px 20px", gap: 18 }}>
          <div className="flex flex-col items-center" style={{ gap: 6 }}>
            <EnsoRing progress={c.confidence} size={104} stroke={8} color="var(--accent)" label={Math.round(c.confidence * 100)} />
            <span className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".12em" }}>Confidence</span>
          </div>

          <div>
            <div className="uppercase text-ink-4 font-semibold" style={{ fontSize: 10.5, letterSpacing: ".14em", marginBottom: 6 }}>Attribution</div>
            <div className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.5 }}>
              {c.origin === "client" ? "Source-dereferenced — no contributor or client identity attached." : <>Named to <b className="text-ink font-semibold" >{c.by}</b>, {c.origin === "community" ? "from the community" : "org-internal"}.</>}
            </div>
          </div>

          <div>
            <div className="flex items-center" style={{ marginBottom: 6 }}>
              <span className="uppercase text-ink-4 font-semibold" style={{ fontSize: 10.5, letterSpacing: ".14em" }}>Distribute to</span>
              <span className="flex-1" />
              <button className="inline-flex items-center bg-transparent border-0 cursor-pointer text-ink-3" style={{ gap: 5, fontSize: 11, fontFamily: "var(--font-mono)" }}>preset · team default ▾</button>
            </div>
            <button className="w-full flex items-center bg-paper border border-paper-edge cursor-pointer text-left" style={{ gap: 8, borderRadius: 7, padding: "9px 11px" }}>
              <span className="kanji text-accent" style={{ fontSize: 13 }}>{c.scopeKanji}</span>
              <span className="text-ink flex-1" style={{ fontSize: 13 }}>{c.scope}</span>
              <span className="mono text-ink-4 uppercase" style={{ fontSize: 9, letterSpacing: ".06em" }}>binding</span>
              <span className="text-ink-3" style={{ fontSize: 9 }}>▾</span>
            </button>
            {/* Resolution (Set distribution): dry-run 'who gets this' before publish */}
            <div className="bg-paper border border-paper-edge" style={{ marginTop: 8, borderRadius: 8, padding: "9px 11px" }}>
              <div className="flex items-center" style={{ gap: 7 }}>
                <span className="kanji text-ink-3" style={{ fontSize: 12 }}>誰</span>
                <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 10, letterSpacing: ".1em" }}>Who gets this</span>
                <span className="flex-1" />
                <span className="mono text-ink-2" style={{ fontSize: 11 }}>{r.repos} repos · {r.devs} devs</span>
              </div>
              <button className="mono text-accent border-0 cursor-pointer p-0" style={{ marginTop: 6, fontSize: 11, background: "none" }}>Preview recipients →</button>
            </div>
            <div className="text-ink-4" style={{ fontSize: 10.5, marginTop: 7, lineHeight: 1.45 }}>Inherits the contribution's binding. Narrowing is free; broadening beyond it asks you to confirm.</div>
          </div>

          <div className="border-t flex flex-col" style={{ paddingTop: 16, marginTop: "auto", gap: 8 }}>
            {/* Resolution (Decide): high-impact items require a second approver */}
            {c.impact === "high" && (
              <div className="bg-accent-soft" style={{ border: "1px solid oklch(0.58 0.15 35/.25)", borderRadius: 8, padding: "10px 11px" }}>
                <div className="flex items-center" style={{ gap: 7, marginBottom: 5 }}>
                  <span className="kanji text-accent" style={{ fontSize: 13 }}>検</span>
                  <span className="uppercase text-accent font-semibold" style={{ fontSize: 10, letterSpacing: ".1em" }}>Second approval required</span>
                </div>
                <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.45 }}>High-impact items need a second maintainer. Threshold set per scope in Scopes &amp; policies.</div>
                <button className="w-full flex items-center bg-paper border border-paper-edge cursor-pointer text-left" style={{ marginTop: 8, gap: 8, borderRadius: 7, padding: "7px 10px" }}>
                  <Avatar name="Sven K." size={18} />
                  <span className="text-ink-2 flex-1" style={{ fontSize: 12 }}>Sven K. · suggested approver</span>
                  <span className="text-ink-3" style={{ fontSize: 9 }}>▾</span>
                </button>
              </div>
            )}
            <button className="w-full border-0 cursor-pointer bg-ink text-paper font-medium inline-flex items-center justify-center" style={{ padding: "11px", borderRadius: 8, fontSize: 13, fontFamily: "inherit", gap: 7 }}>
              <span className="kanji text-accent" style={{ fontSize: 13 }}>決</span> {c.impact === "high" ? "Approve & request 2nd" : "Approve & publish"}
            </button>
            <div className="flex" style={{ gap: 8 }}>
              <button className="flex-1 border border-paper-edge cursor-pointer bg-paper text-ink-2" style={{ padding: "9px", borderRadius: 8, fontSize: 12.5, fontFamily: "inherit" }}>Revise</button>
              <button className="flex-1 border border-paper-edge cursor-pointer bg-paper text-ink-2" style={{ padding: "9px", borderRadius: 8, fontSize: 12.5, fontFamily: "inherit" }}>Decline</button>
            </div>
            <div className="text-ink-4 text-center" style={{ fontSize: 10.5, marginTop: 2 }}>{c.impact === "high" ? "Two named approvals, with notes, land in the audit trail." : "A named decision, with a note, lands in the audit trail."}</div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── Members & roles ────────────────────────────────────── */

function DojoMembers() {
  const D = window.DOJO;
  const roleTone = { "Org admin": "var(--accent)", "Maintainer": "var(--ink)", "Contributor": "var(--ink-2)", "Read-only": "var(--ink-4)" };
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="任" eyebrow="Org · members" title="Members &amp; roles"
        sub="Roles are derived from git — the highest across a member's associated repos — then fine-tuned here."
        right={<div className="flex" style={{ gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">↻ Sync from git</DojoChip>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Bulk import · CSV · SCIM ▾</DojoChip>
          <span className="inline-flex items-center bg-ink text-paper cursor-pointer" style={{ padding: "6px 12px", borderRadius: 7, fontSize: 12.5 }}>Invite</span>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: 28 }}>
        {/* Resolution (Provision members): JIT on first connect, capped at Read-only */}
        <div className="flex items-start bg-paper-2 border border-paper-edge" style={{ gap: 11,
 borderLeft: "3px solid var(--accent)", borderRadius: 10, padding: "13px 16px", marginBottom: 18 }}>
          <span className="kanji text-accent" style={{ fontSize: 16, lineHeight: 1.2 }}>任</span>
          <div className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.55 }}>
            <b className="text-ink font-semibold" >Just-in-time on first connect.</b> A new member is provisioned automatically at their git-derived role — but the auto-default is capped at <b className="font-semibold" >Read-only</b>. Maintainer and Org admin are never granted automatically; they're elevated by hand below.
          </div>
        </div>
        <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
          <div className="grid border-b uppercase text-ink-3 font-semibold" style={{ gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: 14, padding: "11px 18px", fontSize: 10, letterSpacing: ".1em" }}>
            <span>Member</span><span>Git role</span><span>Dōjō role</span><span>Scopes</span><span>Active</span>
          </div>
          {D.members.map((m, i) => (
            <div className="grid items-center" key={m.name} style={{ gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: 14, padding: "13px 18px", borderBottom: i < D.members.length - 1 ? "1px solid var(--edge)" : "none" }}>
              <div className="flex items-center" style={{ gap: 10 }}>
                <Avatar name={m.name} size={28} />
                <span className="text-ink" style={{ fontSize: 13 }}>{m.name}</span>
              </div>
              <span className="mono text-ink-3" style={{ fontSize: 11.5 }}>{m.git}</span>
              <button className="inline-flex items-center bg-paper border border-paper-edge cursor-pointer" style={{ gap: 6,
 borderRadius: 6, padding: "4px 9px", justifySelf: "start" }}>
                <span className="font-medium" style={{ fontSize: 12.5, color: roleTone[m.dojo] || "var(--ink)" }}>{m.dojo}</span>
                <span className="text-ink-4" style={{ fontSize: 8 }}>▾</span>
              </button>
              <span className="text-ink-2" style={{ fontSize: 12 }}>{m.scopes}</span>
              <span className="mono text-ink-4" style={{ fontSize: 11 }}>{m.active}</span>
            </div>
          ))}
        </div>
        <div className="flex items-start text-ink-3" style={{ gap: 9, marginTop: 14, fontSize: 12, lineHeight: 1.5, maxWidth: 760 }}>
          <span className="kanji text-accent" style={{ fontSize: 14 }}>規</span>
          <span>Roles are derived from git then refined. Dōjō-only roles (admin · developer · tester) and per-project overrides — which git doesn't model — are set here; bulk onboarding runs through CSV or SCIM provisioning.</span>
        </div>
      </div>
    </div>
  );
}

/* ─── Clients · dereferencing oversight ──────────────────── */

// Resolution (Clients): one universal anonymization model for every client and
// org — there are no per-client postures, presets, knobs or a human exception
// queue. The standard / pattern / anti-pattern and the memory behind it travel;
// client, repo, identifiers and source are always dropped. Examples can ride
// along, anonymized. Anything that can't be anonymized is dropped, not weakened.
function DojoClients({ go }) {
  const engagements = [
    { kanji: "客", name: "Globex", lessons: 86, scopes: "lumen-auth · billing" },
    { kanji: "客", name: "Initech", lessons: 56, scopes: "initech-portal" },
  ];
  const kept = [
    { k: "標", t: "Standards, patterns & anti-patterns", d: "the reusable rule itself" },
    { k: "憶", t: "The memory — what · why · impact", d: "the reasoning that makes it teachable" },
    { k: "例", t: "Examples, anonymized", d: "illustrative code with every identifier stripped" },
  ];
  const dropped = [
    { k: "客", t: "Client & engagement name" },
    { k: "庫", t: "Repo, file paths & URLs" },
    { k: "名", t: "Identifiers & named entities" },
    { k: "源", t: "The original source & links" },
  ];
  const Panel = ({ title, note, right, children }) => (
    <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
      <div className="flex items-center border-b" style={{ gap: 10, padding: "13px 16px" }}>
        <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".14em" }}>{title}</span>
        {note && <span className="mono text-ink-4" style={{ fontSize: 10.5 }}>{note}</span>}
        <span className="flex-1" />
        {right}
      </div>
      <div style={{ padding: 16 }}>{children}</div>
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="守" eyebrow="Trust · clients" title="Client confidentiality"
        sub="One model for every client and every org: keep the lesson, drop the source. Standards, patterns, anti-patterns and the memory behind them travel — repo, client and identifiers never do."
        right={<div className="text-right text-ink-3" style={{ fontSize: 11, fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div><b className="text-success" >142</b> anonymized · 7d</div>
          <div className="text-success" >0 incidents · uniform policy</div>
        </div>} />
      <div className="flex-1 overflow-auto flex flex-col" style={{ padding: 28, gap: 20 }}>
        {/* the one universal model — keep / drop */}
        <Panel title="One universal model" note="applied identically to every engagement"
          right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">no per-client settings</DojoChip>}>
          <div className="grid gap-0" style={{ gridTemplateColumns: "1fr 1fr" }}>
            {/* kept */}
            <div style={{ paddingRight: 22, borderRight: "1px solid var(--edge)" }}>
              <div className="flex items-center" style={{ gap: 7, marginBottom: 13 }}>
                <span className="rounded-full bg-success" style={{ width: 7, height: 7 }} />
                <span className="uppercase text-success" style={{ fontSize: 11, letterSpacing: ".1em", fontWeight: 700 }}>Kept — travels upstream</span>
              </div>
              <div className="flex flex-col" style={{ gap: 12 }}>
                {kept.map(x => (
                  <div className="grid items-start" key={x.t} style={{ gridTemplateColumns: "auto 1fr", gap: 11 }}>
                    <span className="kanji text-success text-center" style={{ fontSize: 16, width: 18 }}>{x.k}</span>
                    <div><div className="text-ink" style={{ fontSize: 13.5 }}>{x.t}</div><div className="text-ink-3" style={{ fontSize: 11.5, marginTop: 2 }}>{x.d}</div></div>
                  </div>
                ))}
              </div>
            </div>
            {/* dropped */}
            <div style={{ paddingLeft: 22 }}>
              <div className="flex items-center" style={{ gap: 7, marginBottom: 13 }}>
                <span className="rounded-full bg-ink-4" style={{ width: 7, height: 7 }} />
                <span className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".1em", fontWeight: 700 }}>Dropped — never leaves</span>
              </div>
              <div className="flex flex-col" style={{ gap: 12 }}>
                {dropped.map(x => (
                  <div className="grid items-center" key={x.t} style={{ gridTemplateColumns: "auto 1fr auto", gap: 11 }}>
                    <span className="kanji text-ink-4 text-center" style={{ fontSize: 16, width: 18 }}>{x.k}</span>
                    <span className="text-ink-2" style={{ fontSize: 13.5, textDecoration: "line-through", textDecorationColor: "var(--ink-4)" }}>{x.t}</span>
                    <span className="mono text-ink-4" style={{ fontSize: 10 }}>stripped</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Panel>

        <div className="grid items-start" style={{ gridTemplateColumns: "1.25fr 1fr", gap: 18 }}>
          {/* anonymized example diff — examples can be included, stripped */}
          <Panel title="Examples are kept — anonymized" note="raw → what actually leaves">
            <div className="border border-paper-edge overflow-hidden" style={{ borderRadius: 8 }}>
              <div className="flex bg-paper" style={{ gap: 9, padding: "9px 13px", borderBottom: "1px solid var(--edge)", fontFamily: "var(--font-mono)", fontSize: 11.5 }}>
                <span className="shrink-0 text-ink-4" style={{ width: 10, fontWeight: 700 }}>−</span>
                <span className="text-ink-3" style={{ textDecoration: "line-through", textDecorationColor: "var(--ink-4)" }}>globex/lumen-auth · POST /v2/webhooks/billing · ACME_WEBHOOK_SECRET</span>
              </div>
              <div className="flex bg-paper" style={{ gap: 9, padding: "9px 13px", fontFamily: "var(--font-mono)", fontSize: 11.5 }}>
                <span className="shrink-0 text-success" style={{ width: 10, fontWeight: 700 }}>+</span>
                <span className="text-ink" >verify the HMAC signature header against the shared secret, then parse the body</span>
              </div>
            </div>
            <div className="text-ink-3" style={{ fontSize: 11.5, lineHeight: 1.5, marginTop: 11 }}>The teaching keeps a concrete example — the client, repo, route and secret name are dropped before it ever reaches a maintainer.</div>
          </Panel>

          {/* the safety floor + engagements registry */}
          <div className="flex flex-col" style={{ gap: 14 }}>
            <div className="flex items-start bg-paper-2 border border-paper-edge" style={{ gap: 11, borderLeft: "3px solid var(--accent)", borderRadius: 10, padding: "13px 15px" }}>
              <span className="kanji text-accent" style={{ fontSize: 16, lineHeight: 1.2 }}>盾</span>
              <div className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.55 }}>
                <b className="text-ink font-semibold" >Can't be anonymized? It doesn't leave.</b> If a lesson can't stand without identifying context, it's dropped automatically — never weakened, never queued for a judgment call.
              </div>
            </div>
            <Panel title="Engagements" note="routing only"
              right={<button className="inline-flex items-center bg-ink text-paper cursor-pointer border-0" style={{ gap: 6, padding: "5px 11px", borderRadius: 7, fontSize: 11.5, fontFamily: "inherit" }}>+ Register</button>}>
              <div className="flex flex-col" style={{ gap: 2 }}>
                {engagements.map((e, i) => (
                  <div className="grid items-center" key={e.name} style={{ gridTemplateColumns: "auto 1fr auto", gap: 11, padding: "9px 2px", borderBottom: i < engagements.length - 1 ? "1px solid var(--edge)" : "none" }}>
                    <span className="kanji text-accent text-center" style={{ fontSize: 16, width: 18 }}>{e.kanji}</span>
                    <div className="min-w-0" >
                      <div className="text-ink" style={{ fontSize: 13 }}>{e.name}</div>
                      <div className="mono text-ink-4" style={{ fontSize: 10.5, marginTop: 2 }}>{e.scopes}</div>
                    </div>
                    <DojoChip tone="var(--success)" soft="var(--success-soft)">{e.lessons} anonymized</DojoChip>
                  </div>
                ))}
              </div>
            </Panel>
          </div>
        </div>

        {/* proof lives in the audit trail */}
        <div className="flex items-center bg-paper-2 border border-paper-edge" style={{ gap: 9, padding: "13px 16px", borderRadius: 12 }}>
          <span className="kanji text-accent" style={{ fontSize: 15 }}>録</span>
          <span className="text-ink-2 flex-1" style={{ fontSize: 12.5, lineHeight: 1.5 }}>Every anonymization is written to the immutable audit trail — the proof the source was dropped, per engagement, exportable as a confidentiality report.</span>
          <button onClick={() => go && go("audit")} className="mono text-accent border-0 cursor-pointer whitespace-nowrap" style={{ fontSize: 11, background: "none" }}>open audit trail →</button>
        </div>
      </div>
    </div>
  );
}

/* ─── Scopes & policies ──────────────────────────────────── */
// Resolution (Scopes & policies): templates seed a scope; an explicit, testable
// precedence ladder resolves conflicts; client scopes override only stricter.
function DojoScopes() {
  const tree = [
    { k: "社", name: "Company", lvl: 0, preset: "Standard", share: "opt-in" },
    { k: "組", name: "Team · Payments", lvl: 1, preset: "Internal-only", share: "auto" },
    { k: "件", name: "Project · Ledger", lvl: 2, preset: "inherit", share: "inherit" },
    { k: "庫", name: "Repo · ledger-core", lvl: 3, preset: "inherit", share: "inherit" },
    { k: "技", name: "Stack · React", lvl: 1, preset: "Standard", share: "opt-in" },
    { k: "客", name: "Client · Globex", lvl: 1, client: true, preset: "Anonymized", share: "opt-in" },
  ];
  const ladder = [
    { k: "守", name: "Client anonymization", note: "strip client · repo · source before anything else applies", locked: true },
    { k: "庫", name: "Repo", note: "narrowest code scope" },
    { k: "件", name: "Project", note: "" },
    { k: "組", name: "Team", note: "" },
    { k: "社", name: "Company", note: "org-wide rules" },
    { k: "技", name: "Stack", note: "language / framework" },
    { k: "群", name: "Global · community", note: "shared upstream" },
    { k: "己", name: "Personal", note: "your own memory" },
  ];
  const Panel = ({ title, note, children }) => (
    <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
      <div className="flex items-baseline border-b" style={{ gap: 10, padding: "13px 16px" }}>
        <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".14em" }}>{title}</span>
        {note && <span className="mono text-ink-4" style={{ fontSize: 10.5 }}>{note}</span>}
      </div>
      <div style={{ padding: 14 }}>{children}</div>
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="規" eyebrow="Org · policy" title="Scopes &amp; policies"
        sub="The scope hierarchy and its attribution / confidentiality rules. Templates seed a scope; the precedence ladder resolves conflicts top-down. Client-origin lessons are anonymized universally — client, repo and source dropped — before any scope rule applies."
        right={<div className="flex" style={{ gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Apply template ▾</DojoChip>
        </div>} />
      <div className="flex-1 overflow-auto grid items-start" style={{ padding: 28, gridTemplateColumns: "minmax(0,1fr) 392px", gap: 22 }}>
        <div className="flex flex-col min-w-0" style={{ gap: 22 }}>
          <Panel title="Scope hierarchy" note="company → team → project → repo · stack">
            <div className="flex flex-col" >
              {tree.map((s, i) => (
                <div className="flex items-center" key={s.name} style={{ gap: 10, padding: "9px 6px",
 paddingLeft: 6 + s.lvl * 22, borderBottom: i < tree.length - 1 ? "1px solid var(--edge)" : "none" }}>
                  <span className="kanji text-center" style={{ fontSize: 14, color: s.client ? "var(--accent)" : "var(--ink-3)", width: 18 }}>{s.k}</span>
                  <span className="text-ink flex-1" style={{ fontSize: 13 }}>{s.name}</span>
                  {s.client && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">source dropped</DojoChip>}
                  <DojoChip>{s.preset}</DojoChip>
                  <span className="mono text-ink-4 text-right" style={{ fontSize: 10.5, width: 54 }}>{s.share}</span>
                </div>
              ))}
            </div>
          </Panel>
          <Panel title="Test a candidate · which rule wins" note="simulate before you ship a policy">
            <div className="flex flex-col" style={{ gap: 11 }}>
              <div className="flex flex-wrap" style={{ gap: 8 }}>
                <DojoChip tone="var(--ink-2)">type · guard ▾</DojoChip>
                <DojoChip tone="var(--ink-2)">bound · Repo ledger-core ▾</DojoChip>
                <DojoChip tone="var(--accent)" soft="var(--accent-soft)">origin · Client Globex ▾</DojoChip>
              </div>
              <div className="flex items-center bg-paper" style={{ gap: 11, border: "1px solid oklch(0.58 0.15 35/.25)", borderRadius: 9, padding: "11px 14px" }}>
                <span className="kanji text-accent" style={{ fontSize: 16 }}>守</span>
                <div className="text-ink" style={{ fontSize: 12.5, lineHeight: 1.5 }}>
                  <b className="font-semibold" >Client anonymization wins.</b> The lesson is anonymized — client, repo and source dropped — before any repo or team rule applies, then routes by its binding.
                </div>
              </div>
            </div>
          </Panel>
        </div>
        <Panel title="Precedence ladder" note="drag to reorder · top wins">
          <div className="flex flex-col" style={{ gap: 7 }}>
            {ladder.map((r, i) => (
              <div className="grid items-center" key={r.name} style={{ gridTemplateColumns: "auto auto 1fr auto", gap: 10,
 padding: "9px 11px", borderRadius: 8,
 background: r.locked ? "var(--accent-soft)" : "var(--paper)",
 border: r.locked ? "1px solid oklch(0.58 0.15 35/.28)" : "var(--hairline)" }}>
                <span className="mono text-ink-4" style={{ fontSize: 11, width: 12 }}>{i + 1}</span>
                <span className="text-ink-4" style={{ fontSize: 13, opacity: r.locked ? 0.4 : 1, cursor: r.locked ? "default" : "grab", letterSpacing: "-2px" }}>⠿</span>
                <div className="min-w-0 flex items-baseline flex-wrap" style={{ gap: 8 }}>
                  <span className="text-ink" style={{ fontSize: 13 }}>{r.name}</span>
                  {r.note && <span className="text-ink-4" style={{ fontSize: 10.5 }}>{r.note}</span>}
                </div>
                {r.locked && <span className="mono text-accent uppercase" style={{ fontSize: 9, letterSpacing: ".08em" }}>pinned</span>}
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}

/* ─── Audit trail · confidentiality ledger ───────────────── */
// Resolution (Audit trail): immutable per-client log of every dereference and
// outbound lesson, exportable; retention follows the contract; clients can be
// granted read access. Hosts the incident-response path (alert→quarantine→
// retract→review) — the trail is the proof an incident was handled.
function DojoAudit() {
  const log = [
    { t: "09:42", ev: "Dereference", tone: "accent", lesson: "Validate webhook signature before parsing", client: "Globex", actor: "system", hash: "a3f9c1" },
    { t: "09:40", ev: "Outbound", tone: "ink", lesson: "Idempotency key on money-moving mutations", client: "Globex", actor: "Keiko T.", hash: "b1c7e0" },
    { t: "08:55", ev: "Exception cleared", tone: "success", lesson: "Retry budget for a billing webhook", client: "Globex", actor: "Mei L.", hash: "77d24b" },
    { t: "Yest · 18:03", ev: "Quarantine", tone: "warn", lesson: "Cache key shape for a multi-tenant lookup", client: "Initech", actor: "leak-guard", hash: "0e4a8f" },
    { t: "Yest · 11:20", ev: "Dereference", tone: "accent", lesson: "Exponential backoff schedule", client: "Initech", actor: "system", hash: "5fb831" },
    { t: "Mon · 16:47", ev: "Outbound", tone: "ink", lesson: "Persona: integration-test author for auth", client: "—", actor: "Sven K.", hash: "c920a6" },
  ];
  const evTone = { accent: "var(--accent)", ink: "var(--ink-2)", success: "var(--success)", warn: "oklch(0.52 0.13 60)" };
  const evSoft = { accent: "var(--accent-soft)", ink: "var(--paper-3)", success: "var(--success-soft)", warn: "var(--warning-soft)" };
  const steps = [
    { k: "警", name: "Alert", note: "leak-guard fires" },
    { k: "隔", name: "Quarantine", note: "lesson held" },
    { k: "退", name: "Retract", note: "pull downstream" },
    { k: "省", name: "Review", note: "post-incident" },
  ];
  const access = [{ name: "Globex", on: true }, { name: "Initech", on: false }];
  const Panel = ({ title, note, right, children }) => (
    <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
      <div className="flex items-center border-b" style={{ gap: 10, padding: "13px 16px" }}>
        <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".14em" }}>{title}</span>
        {note && <span className="mono text-ink-4" style={{ fontSize: 10.5 }}>{note}</span>}
        <span className="flex-1" />
        {right}
      </div>
      <div style={{ padding: 16 }}>{children}</div>
    </div>
  );
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="録" eyebrow="Trust · audit" title="Confidentiality audit trail"
        sub="An immutable record of every dereference, outbound lesson, and decision — per client. The proof that confidentiality held."
        right={<div className="flex items-center" style={{ gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Filter · all clients ▾</DojoChip>
          <span className="inline-flex items-center bg-ink text-paper cursor-pointer" style={{ gap: 6, padding: "6px 12px", borderRadius: 7, fontSize: 12.5 }}>Export report</span>
        </div>} />
      <div className="flex-1 overflow-auto flex flex-col" style={{ padding: 28, gap: 20 }}>
        {/* Incident response path — Resolution (Incident handling) */}
        <Panel title="Incident response"
          note="alert → quarantine → retract → review"
          right={<><DojoChip tone="var(--ink-3)">shared with admin · Monitor</DojoChip><DojoChip tone="var(--success)" soft="var(--success-soft)">no active incidents · armed</DojoChip></>}>
          <div className="flex items-stretch gap-0" >
            {steps.map((s, i) => (
              <React.Fragment key={s.name}>
                <div className="flex-1 flex flex-col items-center" style={{ gap: 5, padding: "4px 8px" }}>
                  <span className="kanji text-accent" style={{ fontSize: 22, opacity: 0.85 }}>{s.k}</span>
                  <span className="text-ink font-medium" style={{ fontSize: 13 }}>{s.name}</span>
                  <span className="text-ink-4" style={{ fontSize: 11 }}>{s.note}</span>
                </div>
                {i < steps.length - 1 && <span className="self-center text-ink-4" style={{ fontSize: 15 }}>→</span>}
              </React.Fragment>
            ))}
          </div>
          <div className="flex items-center text-ink-3" style={{ gap: 8, marginTop: 12, paddingTop: 12, borderTop: "1px solid var(--edge)", fontSize: 12, lineHeight: 1.5 }}>
            <span className="kanji text-accent" style={{ fontSize: 13 }}>警</span>
            <span>The client lead and org admin are notified immediately; a severity tier decides whether the client is told, per the engagement's contract.</span>
          </div>
        </Panel>

        {/* retention + client read access */}
        <div className="grid" style={{ gridTemplateColumns: "1fr 1fr", gap: 14 }}>
          <Panel title="Retention" note="per engagement">
            <div className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.55 }}>Follows each engagement's contract, plus any statutory minimum.</div>
            <div className="flex" style={{ gap: 18, marginTop: 11 }}>
              {[["Globex", "term + 1y"], ["Initech", "term"]].map(([n, v]) => (
                <div key={n}><div className="mono text-ink-4" style={{ fontSize: 11 }}>客 {n}</div><div className="text-ink" style={{ fontSize: 13.5, marginTop: 2 }}>{v}</div></div>
              ))}
            </div>
          </Panel>
          <Panel title="Client read access" note="their own log only">
            <div className="text-ink-2" style={{ fontSize: 12.5, lineHeight: 1.55, marginBottom: 11 }}>A client can be granted read-only access to its own confidentiality log.</div>
            <div className="flex flex-col" style={{ gap: 8 }}>
              {access.map(a => (
                <div className="flex items-center" key={a.name} style={{ gap: 10 }}>
                  <span className="text-ink flex-1" style={{ fontSize: 13 }}><span className="kanji text-accent" >客</span> {a.name}</span>
                  <span className="inline-flex items-center" style={{ width: 38, height: 20, borderRadius: 12, padding: 2,
 background: a.on ? "var(--accent)" : "var(--paper-3)", justifyContent: a.on ? "flex-end" : "flex-start" }}>
                    <span className="rounded-full bg-paper" style={{ width: 16, height: 16 }} />
                  </span>
                  <span className="mono uppercase" style={{ fontSize: 10, width: 26, color: a.on ? "var(--accent)" : "var(--ink-4)" }}>{a.on ? "on" : "off"}</span>
                </div>
              ))}
            </div>
          </Panel>
        </div>

        {/* the immutable ledger */}
        <Panel title="Ledger" note="immutable · hash-chained" right={<span className="mono text-ink-4" style={{ fontSize: 10.5 }}>144 entries · 7d</span>}>
          <div className="grid uppercase text-ink-4 font-semibold" style={{ gridTemplateColumns: "92px 130px 1fr 96px 96px 78px", gap: 12, padding: "0 4px 9px",
 fontSize: 10, letterSpacing: ".1em" }}>
            <span>Time</span><span>Event</span><span>Lesson</span><span>Client</span><span>Actor</span><span>Hash</span>
          </div>
          <div className="flex flex-col" >
            {log.map((e, i) => (
              <div className="grid items-center" key={i} style={{ gridTemplateColumns: "92px 130px 1fr 96px 96px 78px", gap: 12,
 padding: "10px 4px", borderTop: "1px solid var(--edge)" }}>
                <span className="mono text-ink-4" style={{ fontSize: 11 }}>{e.t}</span>
                <span><DojoChip tone={evTone[e.tone]} soft={evSoft[e.tone]}>{e.ev}</DojoChip></span>
                <span className="text-ink whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 12.5 }}>{e.lesson}</span>
                <span className="text-ink-2" style={{ fontSize: 12 }}>{e.client}</span>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>{e.actor}</span>
                <span className="mono text-ink-4 inline-flex items-center" style={{ fontSize: 11, gap: 4 }}>⠿{e.hash}</span>
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}

/* ─── Monitor · hive-mind health (org admin) ─────────────── */
// Resolution (Monitor): three headline signals — contribution / approval
// throughput, adoption rate, and leak-guard alerts — read over a full audit
// trail. Leak-guard alerts are shared straight into the client lead's incident
// view; this is the admin's standing health dashboard.
function DojoMonitor({ go }) {
  const D = window.DOJO, m = D.metrics;
  // weekly throughput: contributions vs approvals over the last 7d
  const days = ["M", "T", "W", "T", "F", "S", "S"];
  const weeks = m.contribSpark.map((c, i) => ({ c, a: Math.max(0, Math.round(c * (0.62 + i * 0.025))) }));
  const maxBar = Math.max(...weeks.map(w => w.c));
  const approvalRate = Math.round((m.approvedWeek / m.contribWeek) * 100);
  const sevTone = { high: "oklch(0.55 0.17 25)", med: "oklch(0.52 0.13 60)", low: "var(--ink-3)" };
  const sevSoft = { high: "oklch(0.93 0.04 25)", med: "var(--warning-soft)", low: "var(--paper-3)" };
  const alerts = [
    { sev: "high", k: "警", title: "Anomalous outbound volume — Initech scope", state: "quarantined", client: "Initech", when: "2h",
      note: "14 lessons queued outbound in 5 min; auto-quarantined and held pending a lead review." },
    { sev: "med", k: "盾", title: "Named entity survived dereference — Globex", state: "exception", client: "Globex", when: "5h",
      note: "A tenant-id naming scheme cleared the classifier; routed to the client lead's exception queue." },
    { sev: "low", k: "盾", title: "Rare-context lesson flagged — Globex", state: "exception", client: "Globex", when: "1d",
      note: "Low k-anonymity; held for confirmation rather than weakened." },
  ];

  const Signal = ({ kanji, label, value, sub, tone = "var(--ink)", children }) => (
    <div className="flex-1 bg-paper-2 border border-paper-edge min-w-0" style={{ borderRadius: 12, padding: "16px 18px" }}>
      <div className="flex items-center" style={{ gap: 7, marginBottom: 12 }}>
        <span className="kanji text-accent" style={{ fontSize: 14 }}>{kanji}</span>
        <span className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".12em" }}>{label}</span>
      </div>
      <div className="flex items-end justify-between" style={{ gap: 10 }}>
        <div className="display font-light" style={{ fontSize: 36, lineHeight: 1, color: tone }}>{value}</div>
        {children}
      </div>
      <div className="text-ink-3" style={{ fontSize: 11.5, marginTop: 9, lineHeight: 1.4 }}>{sub}</div>
    </div>
  );

  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead kanji="観" eyebrow="Org · monitor" title="Hive-mind health"
        sub="Three headline signals — throughput, adoption, and leak-guard — read over the full audit trail. Anomalies surface here and flow straight into the client lead's incident view."
        right={<div className="flex items-center" style={{ gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Last 7 days ▾</DojoChip>
          <span className="inline-flex items-center text-success bg-success-soft" style={{ gap: 6, fontSize: 11, fontFamily: "var(--font-mono)", border: "1px solid var(--success-edge)",
 borderRadius: 20, padding: "3px 10px" }}>● leak-guard armed</span>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: 28 }}>
        {/* three headline signals */}
        <div className="flex" style={{ gap: 14 }}>
          <Signal kanji="通" label="Throughput · 7d" value={`${m.approvedWeek}/${m.contribWeek}`}
            sub={`${approvalRate}% approved · ${m.contribWeek} contributed, ${m.approvedWeek} published`}>
            <Sparkline data={m.contribSpark} width={96} height={32} color="var(--accent)" fill="var(--accent-soft)" />
          </Signal>
          <Signal kanji="果" label="Adoption rate" value={"+" + Math.round(m.adoptionLift * 100) + "pp"} tone="var(--success)"
            sub="FTR lift across adopting scopes, trending up week over week">
            <Sparkline data={m.ftrSpark} width={96} height={32} color="var(--success)" />
          </Signal>
          <Signal kanji="盾" label="Leak-guard · 7d" value={alerts.length}
            sub={`${m.incidents} confidentiality incidents · ${m.dereferenced} sources dropped`}>
            <div className="flex flex-col items-end" style={{ gap: 4 }}>
              <div className="flex" style={{ gap: 4 }}>
                {alerts.map((a, i) => <span className="rounded-full" key={i} style={{ width: 9, height: 9, background: sevTone[a.sev] }} />)}
              </div>
              <span className="mono text-ink-4 uppercase" style={{ fontSize: 9.5, letterSpacing: ".06em" }}>2 exceptions · 1 held</span>
            </div>
          </Signal>
        </div>

        {/* trend + leak-guard feed */}
        <div className="grid" style={{ gridTemplateColumns: "1.35fr 1fr", gap: 18, marginTop: 18 }}>
          {/* throughput trend */}
          <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
            <div className="flex items-center border-b" style={{ gap: 14, padding: "13px 16px" }}>
              <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".14em" }}>Contributions vs. approvals</span>
              <span className="flex-1" />
              <span className="inline-flex items-center text-ink-3" style={{ gap: 6, fontSize: 11 }}><span className="bg-accent" style={{ width: 9, height: 9, borderRadius: 2, opacity: .85 }} /> contributed</span>
              <span className="inline-flex items-center text-ink-3" style={{ gap: 6, fontSize: 11 }}><span className="bg-success" style={{ width: 9, height: 9, borderRadius: 2 }} /> approved</span>
            </div>
            <div style={{ padding: "20px 18px 16px" }}>
              <div className="flex items-end" style={{ gap: 16, height: 122 }}>
                {weeks.map((w, i) => (
                  <div className="flex-1 flex flex-col items-center" key={i} style={{ gap: 7 }}>
                    <div className="w-full flex items-end justify-center" style={{ height: 96, gap: 5 }}>
                      <div className="bg-accent" style={{ width: 11, height: Math.round(w.c / maxBar * 96), opacity: .85, borderRadius: "3px 3px 0 0" }} />
                      <div className="bg-success" style={{ width: 11, height: Math.round(w.a / maxBar * 96), borderRadius: "3px 3px 0 0" }} />
                    </div>
                    <span className="mono text-ink-4" style={{ fontSize: 10 }}>{days[i]}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* leak-guard alert feed */}
          <div className="bg-paper-2 border border-paper-edge overflow-hidden flex flex-col" style={{ borderRadius: 12 }}>
            <div className="flex items-center border-b" style={{ gap: 10, padding: "13px 16px" }}>
              <span className="kanji text-accent" style={{ fontSize: 14 }}>警</span>
              <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".14em" }}>Leak-guard alerts</span>
              <span className="flex-1" />
              <span className="mono text-ink-4" style={{ fontSize: 10.5 }}>near-leaks &amp; anomalous outbound</span>
            </div>
            <div className="flex-1" >
              {alerts.map((a, i) => (
                <div className="grid" key={i} style={{ gridTemplateColumns: "auto 1fr", gap: 11, padding: "12px 16px",
 borderBottom: i < alerts.length - 1 ? "1px solid var(--edge)" : "none" }}>
                  <span className="kanji text-center" style={{ fontSize: 16, color: sevTone[a.sev], width: 20, lineHeight: 1.3 }}>{a.k}</span>
                  <div className="min-w-0" >
                    <div className="flex items-baseline" style={{ gap: 8 }}>
                      <span className="text-ink flex-1" style={{ fontSize: 13 }}>{a.title}</span>
                      <span className="mono text-ink-4" style={{ fontSize: 10 }}>{a.when}</span>
                    </div>
                    <div className="text-ink-2" style={{ fontSize: 11.5, lineHeight: 1.45, marginTop: 3 }}>{a.note}</div>
                    <div className="flex items-center" style={{ gap: 7, marginTop: 7 }}>
                      <DojoChip tone={sevTone[a.sev]} soft={sevSoft[a.sev]}>{a.sev} severity</DojoChip>
                      <DojoChip tone={a.state === "quarantined" ? "oklch(0.52 0.13 60)" : "var(--ink-3)"} soft={a.state === "quarantined" ? "var(--warning-soft)" : "var(--paper-3)"}>{a.state}</DojoChip>
                      <DojoChip tone="var(--accent)" soft="var(--accent-soft)">客 {a.client}</DojoChip>
                    </div>
                  </div>
                </div>
              ))}
            </div>
            <button className="flex items-center bg-transparent border-0 cursor-pointer text-left w-full" onClick={() => go && go("audit")} style={{ gap: 8, padding: "11px 16px", borderTop: "1px solid var(--edge)", fontFamily: "inherit" }}>
              <span className="kanji text-accent" style={{ fontSize: 13 }}>守</span>
              <span className="text-ink-3 flex-1" style={{ fontSize: 11.5 }}>Shared into the client lead's incident view</span>
              <span className="mono text-accent" style={{ fontSize: 11 }}>open audit trail →</span>
            </button>
          </div>
        </div>

        {/* signals read over the audit trail */}
        <div className="flex items-center bg-paper-2 border border-paper-edge" style={{ gap: 9, marginTop: 18, padding: "13px 16px", borderRadius: 12 }}>
          <span className="kanji text-accent" style={{ fontSize: 15 }}>録</span>
          <span className="text-ink-2 flex-1" style={{ fontSize: 12.5, lineHeight: 1.5 }}>
            Every signal here resolves to entries in the immutable, hash-chained <b className="font-semibold text-ink" >audit trail</b> — the proof a number is real. Throughput and adoption read from the contribution ledger; leak-guard reads from the dereference log.
          </span>
          <button onClick={() => go && go("audit")} className="mono text-accent border-0 cursor-pointer whitespace-nowrap" style={{ fontSize: 11, background: "none" }}>view ledger →</button>
        </div>
      </div>
    </div>
  );
}

/* ─── placeholder for not-yet-designed sections ──────────── */
function DojoSoon({ kanji, label }) {
  return (
    <div className="w-full h-full flex flex-col items-center justify-center bg-paper" style={{ gap: 12 }}>
      <span className="kanji text-accent" style={{ fontSize: 64, opacity: 0.35 }}>{kanji}</span>
      <div className="text-ink-2" style={{ fontSize: 15 }}>{label}</div>
      <div className="text-ink-4" style={{ fontSize: 12 }}>Designed in a later pass</div>
    </div>
  );
}

/* ─── the console ────────────────────────────────────────── */

function DojoConsole({ initial = "overview", initialCandidate = null }) {
  const D = window.DOJO;
  const org = D.memberships.find(m => m.current) || D.memberships[0];
  const [section, setSection] = dS(initial);
  const [candidate, setCandidate] = dS(initialCandidate);

  const go = (sec, cand = null) => { setSection(sec); setCandidate(cand); };

  let screen;
  if (section === "triage" && candidate) screen = <DojoCandidate id={candidate} go={go} />;
  else if (section === "overview") screen = <DojoOverview go={go} />;
  else if (section === "triage") screen = <DojoTriage go={go} />;
  else if (section === "members") screen = <DojoMembers />;
  else if (section === "clients") screen = <DojoClients go={go} />;
  else if (section === "scopes") screen = <DojoScopes />;
  else if (section === "audit") screen = <DojoAudit />;
  else if (section === "monitor") screen = <DojoMonitor go={go} />;
  else if (section === "knowledge") screen = <DojoKnowledge />;
  else screen = <DojoOverview go={go} />;

  return (
    <div className="sensei w-full h-full flex flex-col overflow-hidden bg-paper" data-screen-label="Dōjō Console" >
      <DojoTopBar org={org} />
      <div className="flex-1 flex min-h-0" >
        <DojoNav active={section === "triage" && candidate ? "triage" : section} setActive={(s) => go(s)} />
        <div className="flex-1 min-w-0" >{screen}</div>
      </div>
    </div>
  );
}

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
  const [pruneDays, setPruneDays] = dS("30");

  const HeadCard = ({ children }) => (
    <div className="bg-paper-2 border border-paper-edge" style={{ borderRadius: 12, padding: "16px 18px" }}>{children}</div>
  );
  const Row = ({ it, tone }) => (
    <div className="grid items-center" style={{ gridTemplateColumns: "auto 1fr auto", gap: 13,
 padding: "13px 16px", borderBottom: "1px solid var(--edge)" }}>
      <span className="kanji text-center" style={{ fontSize: 17, color: tone || "var(--accent)", width: 20 }}>{it.k}</span>
      <div className="min-w-0" >
        <div className="text-ink" style={{ fontSize: 13.5 }}>{it.title}</div>
        <div className="flex items-center flex-wrap" style={{ gap: 8, marginTop: 4 }}>
          <span className="mono text-ink-3" style={{ fontSize: 10.5 }}>{it.scope}</span>
          {it.used && <span className="mono text-ink-4" style={{ fontSize: 10.5 }}>· {it.used}</span>}
          {it.reason && <span className="text-ink-3" style={{ fontSize: 11 }}>· {it.reason}</span>}
        </div>
      </div>
      {it.age && <span className="mono text-ink-4" style={{ fontSize: 11 }}>{it.age}</span>}
      {it.left != null && (
        <span className="mono" style={{ fontSize: 11, color: it.left <= 7 ? "var(--accent)" : "var(--ink-3)" }}>
          evicted in {it.left}d
        </span>
      )}
    </div>
  );

  return (
    <div className="h-full overflow-auto" >
      <div className="flex items-start border-b" style={{ gap: 16, padding: "22px 28px 18px" }}>
        <span className="kanji text-accent shrink-0" style={{ fontSize: 38, lineHeight: 1 }}>蔵</span>
        <div className="flex-1 min-w-0" >
          <div className="uppercase text-ink-3" style={{ fontSize: 11, letterSpacing: ".18em", marginBottom: 4 }}>Govern · published library</div>
          <h1 className="display font-normal m-0" style={{ fontSize: 23, letterSpacing: "-0.015em", lineHeight: 1.05 }}>Knowledge</h1>
          <p className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.55, margin: "6px 0 0", maxWidth: 720 }}>
            The Dōjō holds only active, derived intelligence — guards, patterns, principles, skills and personas. As practice
            moves on, a maintainer disables what's gone redundant; disabled knowledge is then evicted automatically.
          </p>
        </div>
      </div>

      <div style={{ padding: 28 }}>
        {/* Lifecycle policy */}
        <HeadCard>
          <div className="flex items-center" style={{ gap: 12 }}>
            <span className="kanji text-accent" style={{ fontSize: 18 }}>時</span>
            <div className="flex-1 min-w-0" >
              <div className="text-ink" style={{ fontSize: 14 }}>Prune disabled knowledge</div>
              <div className="text-ink-3" style={{ fontSize: 12, marginTop: 2, lineHeight: 1.5 }}>
                How long an item stays recoverable after a maintainer disables it. After the window it's evicted from the library for good.
              </div>
            </div>
            <select className="border border-paper-edge bg-paper text-ink cursor-pointer" value={pruneDays} onChange={e => setPruneDays(e.target.value)}
 style={{ fontSize: 13, borderRadius: 5, fontFamily: "inherit", padding: "6px 10px" }}>
              <option value="7">7 days</option>
              <option value="30">30 days · default</option>
              <option value="90">90 days</option>
              <option value="never">Never · keep disabled</option>
            </select>
          </div>
        </HeadCard>

        {/* Active */}
        <div className="uppercase text-ink-3 font-semibold" style={{ fontSize: 11, letterSpacing: ".14em", margin: "22px 0 10px" }}>
          Active · {active.length}
        </div>
        <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12 }}>
          {active.map((it, i) => <Row key={i} it={it}/>)}
        </div>

        {/* Disabled */}
        <div className="uppercase text-ink-3 font-semibold flex items-center" style={{ fontSize: 11, letterSpacing: ".14em", margin: "22px 0 10px", gap: 8 }}>
          Disabled · pending pruning
          <span className="mono text-ink-4 normal-case" style={{ fontSize: 10.5, letterSpacing: 0 }}>
            {pruneDays === "never" ? "retained — pruning off" : `evicting ${pruneDays}d after disable`}
          </span>
        </div>
        <div className="bg-paper-2 border border-paper-edge overflow-hidden" style={{ borderRadius: 12, opacity: 0.92 }}>
          {disabled.map((it, i) => <Row key={i} it={it} tone="var(--ink-4)"/>)}
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  DojoConsole, DojoOverview, DojoTriage, DojoCandidate, DojoMembers, DojoClients, DojoScopes, DojoAudit, DojoMonitor, DojoKnowledge,
});
