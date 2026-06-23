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
    <span className="mono" style={{
      fontSize: 10, letterSpacing: ".04em", color: tone, background: soft,
      border: border || "1px solid transparent", borderRadius: 20,
      padding: "2px 8px", display: "inline-flex", alignItems: "center", gap: 5,
      whiteSpace: "nowrap",
    }}>{children}</span>
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
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <div style={{ width: w, height: 4, borderRadius: 2, background: "var(--paper-3)", overflow: "hidden" }}>
        <div style={{ width: (v * 100) + "%", height: "100%", background: tone, borderRadius: 2 }} />
      </div>
      <span className="mono" style={{ fontSize: 11, color: "var(--ink-2)" }}>{Math.round(v * 100)}</span>
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
const DOJO_BUILT = new Set(["overview", "triage", "members", "clients", "scopes", "audit", "monitor"]);

function DojoTopBar({ org, onSwitch }) {
  const D = window.DOJO;
  return (
    <div style={{
      height: 54, flexShrink: 0, display: "flex", alignItems: "center",
      gap: 16, padding: "0 18px", borderBottom: "var(--hairline)", background: "var(--paper)",
    }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 9 }}>
        <span className="kanji" style={{ fontSize: 22, color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: 18, letterSpacing: "-0.01em" }}>Dōjō</span>
      </div>
      {/* org switcher — the multi-membership model */}
      <button style={{
        display: "inline-flex", alignItems: "center", gap: 8, marginLeft: 6,
        background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 7,
        padding: "6px 11px", cursor: "pointer",
      }}>
        <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>{org.kanji}</span>
        <span style={{ fontSize: 13, color: "var(--ink)" }}>{org.name}</span>
        <span className="mono" style={{ fontSize: 9, color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: ".08em" }}>employer</span>
        <span style={{ fontSize: 9, color: "var(--ink-3)" }}>▾</span>
      </button>
      <div style={{ flex: 1 }} />
      <div style={{
        display: "flex", alignItems: "center", gap: 8, background: "var(--paper-2)",
        border: "var(--hairline)", borderRadius: 7, padding: "6px 11px", width: 260,
      }}>
        <span className="kanji" style={{ fontSize: 12, color: "var(--ink-3)" }}>探</span>
        <span style={{ fontSize: 12, color: "var(--ink-4)" }}>search knowledge…</span>
      </div>
      <span style={{ fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--font-mono)" }}>
        {D.org.members} members
      </span>
      <Avatar name="Keiko" size={28} />
    </div>
  );
}

function DojoNav({ active, setActive }) {
  return (
    <aside style={{
      width: 218, flexShrink: 0, borderRight: "var(--hairline)", background: "var(--paper-2)",
      display: "flex", flexDirection: "column", padding: "16px 12px", overflow: "auto",
    }}>
      {DOJO_NAV.map(grp => (
        <div key={grp.group} style={{ marginBottom: 14 }}>
          <div style={{ fontSize: 9.5, letterSpacing: ".14em", textTransform: "uppercase",
                        color: "var(--ink-4)", fontWeight: 600, padding: "0 8px", marginBottom: 6 }}>{grp.group}</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {grp.items.map(it => {
              const on = active === it.id;
              const built = DOJO_BUILT.has(it.id);
              return (
                <button key={it.id} onClick={() => built && setActive(it.id)}
                  title={built ? "" : "Designed in a later pass"}
                  style={{
                    display: "grid", gridTemplateColumns: "auto 1fr auto", alignItems: "center",
                    gap: 9, width: "100%", textAlign: "left", borderRadius: 7, padding: "8px 9px",
                    background: on ? "var(--paper)" : "transparent",
                    border: on ? "var(--hairline)" : "1px solid transparent",
                    color: on ? "var(--ink)" : built ? "var(--ink-2)" : "var(--ink-4)",
                    cursor: built ? "pointer" : "default", fontSize: 13, opacity: built ? 1 : 0.6,
                  }}>
                  <span className="kanji" style={{ fontSize: 13, width: 15, textAlign: "center",
                                color: on ? "var(--accent)" : "var(--ink-3)" }}>{it.kanji}</span>
                  <span>{it.label}</span>
                  {it.badge != null
                    ? <span className="mono" style={{ fontSize: 10, fontWeight: 600, color: "var(--paper)",
                              background: "var(--accent)", borderRadius: 10, padding: "0 6px", lineHeight: "16px" }}>{it.badge}</span>
                    : !built ? <span style={{ fontSize: 8.5, color: "var(--ink-4)", letterSpacing: ".06em" }}>soon</span> : <span/>}
                </button>
              );
            })}
          </div>
        </div>
      ))}
      <div style={{ flex: 1 }} />
      <button style={{
        display: "grid", gridTemplateColumns: "auto 1fr", alignItems: "center", gap: 9,
        width: "100%", textAlign: "left", borderRadius: 7, padding: "8px 9px",
        background: "transparent", color: "var(--ink-3)", fontSize: 13, cursor: "default",
        borderTop: "var(--hairline)", borderRadius: 0, paddingTop: 12, opacity: 0.6,
      }}>
        <span className="kanji" style={{ fontSize: 13, width: 15, textAlign: "center", color: "var(--ink-3)" }}>調</span>
        <span>Settings · SSO</span>
      </button>
    </aside>
  );
}

// Section header band inside the main column.
function DojoHead({ kanji, eyebrow, title, sub, right }) {
  return (
    <div style={{ display: "flex", alignItems: "flex-start", gap: 18, padding: "22px 28px 18px",
                  borderBottom: "var(--hairline)", flexShrink: 0 }}>
      <span className="kanji" style={{ fontSize: 38, color: "var(--accent)", lineHeight: 1, flexShrink: 0 }}>{kanji}</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11, letterSpacing: ".18em", textTransform: "uppercase", color: "var(--ink-3)", marginBottom: 4 }}>{eyebrow}</div>
        <h1 className="display" style={{ fontSize: 24, fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: 1.05 }}>{title}</h1>
        {sub && <p style={{ fontSize: 13, color: "var(--ink-2)", lineHeight: 1.55, margin: "6px 0 0", maxWidth: 680 }}>{sub}</p>}
      </div>
      {right && <div style={{ flexShrink: 0 }}>{right}</div>}
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
    <div onClick={onClick} style={{
      flex: 1, background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12,
      padding: "16px 18px", cursor: onClick ? "pointer" : "default", minWidth: 0,
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 10 }}>
        <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)" }}>{label}</span>
      </div>
      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: 10 }}>
        <div className="display" style={{ fontSize: 34, fontWeight: 300, lineHeight: 1, color: "var(--ink)" }}>{value}</div>
        {children}
      </div>
      {sub && <div style={{ fontSize: 11, color: "var(--ink-3)", marginTop: 8 }}>{sub}</div>}
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="全" eyebrow="Acme Corp · Dōjō" title="The shared mind, governed."
        sub="What your org has learned — triaged, approved, and routed to the scopes that need it."
        right={<div style={{ textAlign: "right", fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div>{D.org.scopes} scopes · {D.org.repos} repos</div>
          <div style={{ color: "var(--success)" }}>{m.incidents} confidentiality incidents</div>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        {/* metric row */}
        <div style={{ display: "flex", gap: 14 }}>
          <Metric kanji="門" label="Pending triage" value={m.pendingTriage}
            sub="across 4 scopes · oldest 3d" onClick={() => go("triage")}>
            <span className="mono" style={{ fontSize: 11, color: "var(--accent)" }}>review →</span>
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
        <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1fr", gap: 18, marginTop: 18 }}>
          {/* triage preview */}
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", padding: "13px 16px", borderBottom: "var(--hairline)" }}>
              <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)" }}>Top of the triage queue</span>
              <span style={{ flex: 1 }} />
              <button onClick={() => go("triage")} className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>all 7 →</button>
            </div>
            {D.queue.slice(0, 4).map((c, i) => (
              <button key={c.id} onClick={() => go("triage", c.id)} style={{
                display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 12, alignItems: "center",
                width: "100%", textAlign: "left", padding: "12px 16px", cursor: "pointer",
                background: "transparent", border: "none",
                borderBottom: i < 3 ? "1px solid var(--edge)" : "none",
              }}>
                <span className="kanji" style={{ fontSize: 17, color: "var(--accent)", width: 20, textAlign: "center" }}>{c.kanji}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.title}</div>
                  <div style={{ display: "flex", gap: 7, marginTop: 4, alignItems: "center" }}>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-3)" }}>{c.scope}</span>
                    <OriginChip origin={c.origin} />
                  </div>
                </div>
                <Confidence v={c.confidence} w={56} />
              </button>
            ))}
          </div>

          {/* activity + leak guard */}
          <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
            <div style={{ background: "var(--paper-2)", border: "1px solid var(--success-edge)", borderRadius: 12, padding: "15px 17px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                <span className="kanji" style={{ fontSize: 15, color: "var(--success)" }}>盾</span>
                <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)" }}>Confidentiality</span>
              </div>
              <div style={{ fontSize: 13, color: "var(--ink)", lineHeight: 1.55 }}>
                <b style={{ fontWeight: 600 }}>{m.dereferenced}</b> client lessons auto-dereferenced this week ·
                <span style={{ color: "var(--success)" }}> 0 incidents</span>. Sources dropped automatically; only flagged exceptions reach a lead.
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 7, marginTop: 10, paddingTop: 10, borderTop: "1px solid var(--edge)" }}>
                <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--success)" }} />
                <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: ".06em" }}>Leak-guard armed</span>
                <span style={{ flex: 1 }} />
                <span style={{ fontSize: 11, color: "var(--ink-3)" }}>alerts shared to the client lead <span style={{ color: "var(--accent)" }}>→</span></span>
              </div>
            </div>
            <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden", flex: 1 }}>
              <div style={{ padding: "13px 16px", borderBottom: "var(--hairline)", fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)" }}>Recent activity</div>
              <div style={{ padding: "4px 0" }}>
                {D.activity.map((a, i) => (
                  <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 10, alignItems: "start", padding: "9px 16px" }}>
                    <span className="kanji" style={{ fontSize: 13, width: 16, textAlign: "center",
                                  color: a.tone === "success" ? "var(--success)" : a.tone === "accent" ? "var(--accent)" : "var(--ink-3)" }}>{a.kanji}</span>
                    <span style={{ fontSize: 12, color: "var(--ink-2)", lineHeight: 1.45 }}>{a.text}</span>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{a.when}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Resolution (Publish & measure): the Impact loop — lifecycle + retract */}
        <div style={{ marginTop: 18 }}>
          <div style={{ display: "flex", alignItems: "center", marginBottom: 9 }}>
            <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Published · adoption &amp; health</span>
            <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", marginLeft: 10 }}>the Impact loop, scoped to the org</span>
          </div>
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
            {published.map((p, i) => {
              const neg = p.delta < 0;
              return (
              <div key={p.title} style={{ display: "grid", gridTemplateColumns: "auto 1fr 150px 92px 168px", gap: 14, alignItems: "center",
                            padding: "13px 16px", borderBottom: i < published.length - 1 ? "1px solid var(--edge)" : "none",
                            background: neg ? "var(--warning-soft)" : "transparent" }}>
                <span className="kanji" style={{ fontSize: 17, color: neg ? "oklch(0.52 0.13 60)" : "var(--accent)", width: 20, textAlign: "center" }}>{p.kanji}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.title}</div>
                  <div className="mono" style={{ fontSize: 10, color: "var(--ink-4)", marginTop: 3 }}>{p.scope}</div>
                </div>
                {/* adoption */}
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <div style={{ flex: 1, height: 4, borderRadius: 2, background: "var(--paper-3)", overflow: "hidden" }}>
                    <div style={{ width: (p.adoption * 100) + "%", height: "100%", background: "var(--accent)", borderRadius: 2 }} />
                  </div>
                  <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{Math.round(p.adoption * 100)}%</span>
                </div>
                {/* impact delta */}
                <span className="mono" style={{ fontSize: 12, color: neg ? "oklch(0.52 0.13 60)" : "var(--success)", textAlign: "right" }}>
                  {neg ? "" : "+"}{p.delta}pp FTR
                </span>
                {/* lifecycle / action */}
                <div style={{ display: "flex", justifyContent: "flex-end" }}>
                  {neg
                    ? <button style={{ display: "inline-flex", alignItems: "center", gap: 6, padding: "6px 12px", borderRadius: 7,
                              border: "1px solid oklch(0.52 0.13 60/.4)", background: "var(--paper)", color: "oklch(0.5 0.13 60)", fontSize: 12, cursor: "pointer", fontFamily: "inherit" }}>
                        <span className="kanji" style={{ fontSize: 12 }}>退</span> Retract downstream
                      </button>
                    : <DojoChip tone="var(--success)" soft="var(--success-soft)">active</DojoChip>}
                </div>
              </div>
              );
            })}
            <div style={{ display: "flex", alignItems: "center", gap: 7, padding: "10px 16px", borderTop: "1px solid var(--edge)", fontSize: 11.5, color: "var(--ink-3)", lineHeight: 1.45 }}>
              <span className="kanji" style={{ fontSize: 12, color: "oklch(0.52 0.13 60)" }}>退</span>
              <span>Lifecycle <b style={{ fontWeight: 600, color: "var(--ink-2)" }}>active → deprecated → retracted</b>. Negative impact is flagged automatically; one-click retract pulls a teaching back and notifies adopters.</span>
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
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="門" eyebrow="Govern · triage" title="What's waiting for review"
        sub="Your scopes by default, ranked by projected impact then age. Every scope has an owner — anything unowned routes to a fallback so nothing sits idle. Nothing publishes without a decision."
        right={<div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 11, fontFamily: "var(--font-mono)",
                        color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid oklch(0.58 0.15 35/.28)",
                        borderRadius: 20, padding: "3px 10px" }}>✓ My scopes</span>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Sort · impact, then age ▾</DojoChip>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: "8px 28px 28px" }}>
        {ordered.map(([scope, items]) => {
          const owner = SCOPE_OWNERS[scope];
          return (
          <div key={scope} style={{ marginTop: 18 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
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
                  display: "grid", gridTemplateColumns: "auto 1fr auto auto auto", gap: 14, alignItems: "center",
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
                    </div>
                  </div>
                  {/* signals */}
                  <div style={{ display: "flex", gap: 10, alignItems: "center", minWidth: 96, justifyContent: "flex-end" }}>
                    {c.conflicts > 0 && <DojoChip tone="oklch(0.52 0.13 60)" soft="var(--warning-soft)">{c.conflicts} conflict</DojoChip>}
                    {c.dups > 0 && <DojoChip>{c.dups} dup</DojoChip>}
                  </div>
                  <Confidence v={c.confidence} />
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
  // Resolution (Set distribution): dry-run reach per scope for 'who gets this'.
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
      {/* breadcrumb */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "12px 28px", borderBottom: "var(--hairline)", flexShrink: 0 }}>
        <button onClick={() => go("triage")} className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>← Triage</button>
        <span style={{ fontSize: 11, color: "var(--ink-4)" }}>/</span>
        <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{c.scope}</span>
      </div>
      <div style={{ flex: 1, display: "grid", gridTemplateColumns: "1fr 332px", minHeight: 0 }}>
        {/* main */}
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

          {/* evidence */}
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 10, padding: "13px 16px", marginBottom: 16 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--ink-3)" }}>証</span>
              <span style={{ fontSize: 13, color: "var(--ink)" }}><b style={{ fontWeight: 600 }}>{c.evidence} sessions</b> support this</span>
              <span style={{ flex: 1 }} />
              <button className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer" }}>view evidence →</button>
            </div>
          </div>

          {/* dereferencing note for client work */}
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

          {/* conflicts / dups — Resolution (Evaluate): always surface a conflict
              DIFF against the rule it would supersede, and a merge suggestion for
              near-duplicates with the auto-dedupe band made explicit. */}
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

        {/* decide rail */}
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
            {/* Resolution (Set distribution): dry-run 'who gets this' before publish */}
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
            {/* Resolution (Decide): high-impact items require a second approver */}
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

/* ─── Members & roles ────────────────────────────────────── */

function DojoMembers() {
  const D = window.DOJO;
  const roleTone = { "Org admin": "var(--accent)", "Maintainer": "var(--ink)", "Contributor": "var(--ink-2)", "Read-only": "var(--ink-4)" };
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="任" eyebrow="Org · members" title="Members &amp; roles"
        sub="Roles are derived from git — the highest across a member's associated repos — then fine-tuned here."
        right={<div style={{ display: "flex", gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">↻ Sync from git</DojoChip>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Bulk import · CSV · SCIM ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", padding: "6px 12px", borderRadius: 7, background: "var(--ink)", color: "var(--paper)", fontSize: 12.5, cursor: "pointer" }}>Invite</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        {/* Resolution (Provision members): JIT on first connect, capped at Read-only */}
        <div style={{ display: "flex", alignItems: "flex-start", gap: 11, background: "var(--paper-2)", border: "var(--hairline)",
                      borderLeft: "3px solid var(--accent)", borderRadius: 10, padding: "13px 16px", marginBottom: 18 }}>
          <span className="kanji" style={{ fontSize: 16, color: "var(--accent)", lineHeight: 1.2 }}>任</span>
          <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
            <b style={{ color: "var(--ink)", fontWeight: 600 }}>Just-in-time on first connect.</b> A new member is provisioned automatically at their git-derived role — but the auto-default is capped at <b style={{ fontWeight: 600 }}>Read-only</b>. Maintainer and Org admin are never granted automatically; they're elevated by hand below.
          </div>
        </div>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
          <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: 14, padding: "11px 18px",
                        borderBottom: "var(--hairline)", fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>
            <span>Member</span><span>Git role</span><span>Dōjō role</span><span>Scopes</span><span>Active</span>
          </div>
          {D.members.map((m, i) => (
            <div key={m.name} style={{ display: "grid", gridTemplateColumns: "1.4fr 1.2fr 1fr 1.2fr auto", gap: 14, padding: "13px 18px",
                          alignItems: "center", borderBottom: i < D.members.length - 1 ? "1px solid var(--edge)" : "none" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <Avatar name={m.name} size={28} />
                <span style={{ fontSize: 13, color: "var(--ink)" }}>{m.name}</span>
              </div>
              <span className="mono" style={{ fontSize: 11.5, color: "var(--ink-3)" }}>{m.git}</span>
              <button style={{ display: "inline-flex", alignItems: "center", gap: 6, background: "var(--paper)", border: "var(--hairline)",
                            borderRadius: 6, padding: "4px 9px", cursor: "pointer", justifySelf: "start" }}>
                <span style={{ fontSize: 12.5, color: roleTone[m.dojo] || "var(--ink)", fontWeight: 500 }}>{m.dojo}</span>
                <span style={{ fontSize: 8, color: "var(--ink-4)" }}>▾</span>
              </button>
              <span style={{ fontSize: 12, color: "var(--ink-2)" }}>{m.scopes}</span>
              <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{m.active}</span>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "flex-start", gap: 9, marginTop: 14, fontSize: 12, color: "var(--ink-3)", lineHeight: 1.5, maxWidth: 760 }}>
          <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>規</span>
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
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        {note && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{note}</span>}
        <span style={{ flex: 1 }} />
        {right}
      </div>
      <div style={{ padding: 16 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="守" eyebrow="Trust · clients" title="Client confidentiality"
        sub="One model for every client and every org: keep the lesson, drop the source. Standards, patterns, anti-patterns and the memory behind them travel — repo, client and identifiers never do."
        right={<div style={{ textAlign: "right", fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--ink-3)", lineHeight: 1.7 }}>
          <div><b style={{ color: "var(--success)" }}>142</b> anonymized · 7d</div>
          <div style={{ color: "var(--success)" }}>0 incidents · uniform policy</div>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28, display: "flex", flexDirection: "column", gap: 20 }}>
        {/* the one universal model — keep / drop */}
        <Panel title="One universal model" note="applied identically to every engagement"
          right={<DojoChip tone="var(--accent)" soft="var(--accent-soft)">no per-client settings</DojoChip>}>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 0 }}>
            {/* kept */}
            <div style={{ paddingRight: 22, borderRight: "1px solid var(--edge)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 13 }}>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />
                <span style={{ fontSize: 11, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--success)", fontWeight: 700 }}>Kept — travels upstream</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                {kept.map(x => (
                  <div key={x.t} style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: 11, alignItems: "start" }}>
                    <span className="kanji" style={{ fontSize: 16, color: "var(--success)", width: 18, textAlign: "center" }}>{x.k}</span>
                    <div><div style={{ fontSize: 13.5, color: "var(--ink)" }}>{x.t}</div><div style={{ fontSize: 11.5, color: "var(--ink-3)", marginTop: 2 }}>{x.d}</div></div>
                  </div>
                ))}
              </div>
            </div>
            {/* dropped */}
            <div style={{ paddingLeft: 22 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 13 }}>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--ink-4)" }} />
                <span style={{ fontSize: 11, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 700 }}>Dropped — never leaves</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                {dropped.map(x => (
                  <div key={x.t} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 11, alignItems: "center" }}>
                    <span className="kanji" style={{ fontSize: 16, color: "var(--ink-4)", width: 18, textAlign: "center" }}>{x.k}</span>
                    <span style={{ fontSize: 13.5, color: "var(--ink-2)", textDecoration: "line-through", textDecorationColor: "var(--ink-4)" }}>{x.t}</span>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>stripped</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Panel>

        <div style={{ display: "grid", gridTemplateColumns: "1.25fr 1fr", gap: 18, alignItems: "start" }}>
          {/* anonymized example diff — examples can be included, stripped */}
          <Panel title="Examples are kept — anonymized" note="raw → what actually leaves">
            <div style={{ border: "var(--hairline)", borderRadius: 8, overflow: "hidden" }}>
              <div style={{ display: "flex", gap: 9, padding: "9px 13px", borderBottom: "1px solid var(--edge)", fontFamily: "var(--font-mono)", fontSize: 11.5, background: "var(--paper)" }}>
                <span style={{ width: 10, flexShrink: 0, color: "var(--ink-4)", fontWeight: 700 }}>−</span>
                <span style={{ color: "var(--ink-3)", textDecoration: "line-through", textDecorationColor: "var(--ink-4)" }}>globex/lumen-auth · POST /v2/webhooks/billing · ACME_WEBHOOK_SECRET</span>
              </div>
              <div style={{ display: "flex", gap: 9, padding: "9px 13px", fontFamily: "var(--font-mono)", fontSize: 11.5, background: "var(--paper)" }}>
                <span style={{ width: 10, flexShrink: 0, color: "var(--success)", fontWeight: 700 }}>+</span>
                <span style={{ color: "var(--ink)" }}>verify the HMAC signature header against the shared secret, then parse the body</span>
              </div>
            </div>
            <div style={{ fontSize: 11.5, color: "var(--ink-3)", lineHeight: 1.5, marginTop: 11 }}>The teaching keeps a concrete example — the client, repo, route and secret name are dropped before it ever reaches a maintainer.</div>
          </Panel>

          {/* the safety floor + engagements registry */}
          <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
            <div style={{ display: "flex", alignItems: "flex-start", gap: 11, background: "var(--paper-2)", border: "var(--hairline)", borderLeft: "3px solid var(--accent)", borderRadius: 10, padding: "13px 15px" }}>
              <span className="kanji" style={{ fontSize: 16, color: "var(--accent)", lineHeight: 1.2 }}>盾</span>
              <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
                <b style={{ color: "var(--ink)", fontWeight: 600 }}>Can't be anonymized? It doesn't leave.</b> If a lesson can't stand without identifying context, it's dropped automatically — never weakened, never queued for a judgment call.
              </div>
            </div>
            <Panel title="Engagements" note="routing only"
              right={<button style={{ display: "inline-flex", alignItems: "center", gap: 6, padding: "5px 11px", borderRadius: 7, background: "var(--ink)", color: "var(--paper)", fontSize: 11.5, cursor: "pointer", fontFamily: "inherit", border: "none" }}>+ Register</button>}>
              <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {engagements.map((e, i) => (
                  <div key={e.name} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 11, alignItems: "center", padding: "9px 2px", borderBottom: i < engagements.length - 1 ? "1px solid var(--edge)" : "none" }}>
                    <span className="kanji" style={{ fontSize: 16, color: "var(--accent)", width: 18, textAlign: "center" }}>{e.kanji}</span>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontSize: 13, color: "var(--ink)" }}>{e.name}</div>
                      <div className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 2 }}>{e.scopes}</div>
                    </div>
                    <DojoChip tone="var(--success)" soft="var(--success-soft)">{e.lessons} anonymized</DojoChip>
                  </div>
                ))}
              </div>
            </Panel>
          </div>
        </div>

        {/* proof lives in the audit trail */}
        <div style={{ display: "flex", alignItems: "center", gap: 9, padding: "13px 16px", background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12 }}>
          <span className="kanji" style={{ fontSize: 15, color: "var(--accent)" }}>録</span>
          <span style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.5, flex: 1 }}>Every anonymization is written to the immutable audit trail — the proof the source was dropped, per engagement, exportable as a confidentiality report.</span>
          <button onClick={() => go && go("audit")} className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", whiteSpace: "nowrap" }}>open audit trail →</button>
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
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        {note && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{note}</span>}
      </div>
      <div style={{ padding: 14 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="規" eyebrow="Org · policy" title="Scopes &amp; policies"
        sub="The scope hierarchy and its attribution / confidentiality rules. Templates seed a scope; the precedence ladder resolves conflicts top-down. Client-origin lessons are anonymized universally — client, repo and source dropped — before any scope rule applies."
        right={<div style={{ display: "flex", gap: 8 }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Apply template ▾</DojoChip>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28, display: "grid", gridTemplateColumns: "minmax(0,1fr) 392px", gap: 22, alignItems: "start" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 22, minWidth: 0 }}>
          <Panel title="Scope hierarchy" note="company → team → project → repo · stack">
            <div style={{ display: "flex", flexDirection: "column" }}>
              {tree.map((s, i) => (
                <div key={s.name} style={{ display: "flex", alignItems: "center", gap: 10, padding: "9px 6px",
                              paddingLeft: 6 + s.lvl * 22, borderBottom: i < tree.length - 1 ? "1px solid var(--edge)" : "none" }}>
                  <span className="kanji" style={{ fontSize: 14, color: s.client ? "var(--accent)" : "var(--ink-3)", width: 18, textAlign: "center" }}>{s.k}</span>
                  <span style={{ fontSize: 13, color: "var(--ink)", flex: 1 }}>{s.name}</span>
                  {s.client && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">source dropped</DojoChip>}
                  <DojoChip>{s.preset}</DojoChip>
                  <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", width: 54, textAlign: "right" }}>{s.share}</span>
                </div>
              ))}
            </div>
          </Panel>
          <Panel title="Test a candidate · which rule wins" note="simulate before you ship a policy">
            <div style={{ display: "flex", flexDirection: "column", gap: 11 }}>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <DojoChip tone="var(--ink-2)">type · guard ▾</DojoChip>
                <DojoChip tone="var(--ink-2)">bound · Repo ledger-core ▾</DojoChip>
                <DojoChip tone="var(--accent)" soft="var(--accent-soft)">origin · Client Globex ▾</DojoChip>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 11, background: "var(--paper)", border: "1px solid oklch(0.58 0.15 35/.25)", borderRadius: 9, padding: "11px 14px" }}>
                <span className="kanji" style={{ fontSize: 16, color: "var(--accent)" }}>守</span>
                <div style={{ fontSize: 12.5, color: "var(--ink)", lineHeight: 1.5 }}>
                  <b style={{ fontWeight: 600 }}>Client anonymization wins.</b> The lesson is anonymized — client, repo and source dropped — before any repo or team rule applies, then routes by its binding.
                </div>
              </div>
            </div>
          </Panel>
        </div>
        <Panel title="Precedence ladder" note="drag to reorder · top wins">
          <div style={{ display: "flex", flexDirection: "column", gap: 7 }}>
            {ladder.map((r, i) => (
              <div key={r.name} style={{ display: "grid", gridTemplateColumns: "auto auto 1fr auto", gap: 10, alignItems: "center",
                            padding: "9px 11px", borderRadius: 8,
                            background: r.locked ? "var(--accent-soft)" : "var(--paper)",
                            border: r.locked ? "1px solid oklch(0.58 0.15 35/.28)" : "var(--hairline)" }}>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", width: 12 }}>{i + 1}</span>
                <span style={{ fontSize: 13, color: "var(--ink-4)", opacity: r.locked ? 0.4 : 1, cursor: r.locked ? "default" : "grab", letterSpacing: "-2px" }}>⠿</span>
                <div style={{ minWidth: 0, display: "flex", alignItems: "baseline", gap: 8, flexWrap: "wrap" }}>
                  <span style={{ fontSize: 13, color: "var(--ink)" }}>{r.name}</span>
                  {r.note && <span style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{r.note}</span>}
                </div>
                {r.locked && <span className="mono" style={{ fontSize: 9, color: "var(--accent)", textTransform: "uppercase", letterSpacing: ".08em" }}>pinned</span>}
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
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        {note && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{note}</span>}
        <span style={{ flex: 1 }} />
        {right}
      </div>
      <div style={{ padding: 16 }}>{children}</div>
    </div>
  );
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="録" eyebrow="Trust · audit" title="Confidentiality audit trail"
        sub="An immutable record of every dereference, outbound lesson, and decision — per client. The proof that confidentiality held."
        right={<div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Filter · all clients ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6, padding: "6px 12px", borderRadius: 7, background: "var(--ink)", color: "var(--paper)", fontSize: 12.5, cursor: "pointer" }}>Export report</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28, display: "flex", flexDirection: "column", gap: 20 }}>
        {/* Incident response path — Resolution (Incident handling) */}
        <Panel title="Incident response"
          note="alert → quarantine → retract → review"
          right={<><DojoChip tone="var(--ink-3)">shared with admin · Monitor</DojoChip><DojoChip tone="var(--success)" soft="var(--success-soft)">no active incidents · armed</DojoChip></>}>
          <div style={{ display: "flex", alignItems: "stretch", gap: 0 }}>
            {steps.map((s, i) => (
              <React.Fragment key={s.name}>
                <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: 5, padding: "4px 8px" }}>
                  <span className="kanji" style={{ fontSize: 22, color: "var(--accent)", opacity: 0.85 }}>{s.k}</span>
                  <span style={{ fontSize: 13, color: "var(--ink)", fontWeight: 500 }}>{s.name}</span>
                  <span style={{ fontSize: 11, color: "var(--ink-4)" }}>{s.note}</span>
                </div>
                {i < steps.length - 1 && <span style={{ alignSelf: "center", fontSize: 15, color: "var(--ink-4)" }}>→</span>}
              </React.Fragment>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 12, paddingTop: 12, borderTop: "1px solid var(--edge)", fontSize: 12, color: "var(--ink-3)", lineHeight: 1.5 }}>
            <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>警</span>
            <span>The client lead and org admin are notified immediately; a severity tier decides whether the client is told, per the engagement's contract.</span>
          </div>
        </Panel>

        {/* retention + client read access */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
          <Panel title="Retention" note="per engagement">
            <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>Follows each engagement's contract, plus any statutory minimum.</div>
            <div style={{ display: "flex", gap: 18, marginTop: 11 }}>
              {[["Globex", "term + 1y"], ["Initech", "term"]].map(([n, v]) => (
                <div key={n}><div className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>客 {n}</div><div style={{ fontSize: 13.5, color: "var(--ink)", marginTop: 2 }}>{v}</div></div>
              ))}
            </div>
          </Panel>
          <Panel title="Client read access" note="their own log only">
            <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55, marginBottom: 11 }}>A client can be granted read-only access to its own confidentiality log.</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {access.map(a => (
                <div key={a.name} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <span style={{ fontSize: 13, color: "var(--ink)", flex: 1 }}><span className="kanji" style={{ color: "var(--accent)" }}>客</span> {a.name}</span>
                  <span style={{ display: "inline-flex", alignItems: "center", width: 38, height: 20, borderRadius: 12, padding: 2,
                              background: a.on ? "var(--accent)" : "var(--paper-3)", justifyContent: a.on ? "flex-end" : "flex-start" }}>
                    <span style={{ width: 16, height: 16, borderRadius: "50%", background: "var(--paper)" }} />
                  </span>
                  <span className="mono" style={{ fontSize: 10, width: 26, color: a.on ? "var(--accent)" : "var(--ink-4)", textTransform: "uppercase" }}>{a.on ? "on" : "off"}</span>
                </div>
              ))}
            </div>
          </Panel>
        </div>

        {/* the immutable ledger */}
        <Panel title="Ledger" note="immutable · hash-chained" right={<span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>144 entries · 7d</span>}>
          <div style={{ display: "grid", gridTemplateColumns: "92px 130px 1fr 96px 96px 78px", gap: 12, padding: "0 4px 9px",
                        fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600 }}>
            <span>Time</span><span>Event</span><span>Lesson</span><span>Client</span><span>Actor</span><span>Hash</span>
          </div>
          <div style={{ display: "flex", flexDirection: "column" }}>
            {log.map((e, i) => (
              <div key={i} style={{ display: "grid", gridTemplateColumns: "92px 130px 1fr 96px 96px 78px", gap: 12, alignItems: "center",
                            padding: "10px 4px", borderTop: "1px solid var(--edge)" }}>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{e.t}</span>
                <span><DojoChip tone={evTone[e.tone]} soft={evSoft[e.tone]}>{e.ev}</DojoChip></span>
                <span style={{ fontSize: 12.5, color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{e.lesson}</span>
                <span style={{ fontSize: 12, color: "var(--ink-2)" }}>{e.client}</span>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{e.actor}</span>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", display: "inline-flex", alignItems: "center", gap: 4 }}>⠿{e.hash}</span>
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
    <div style={{ flex: 1, background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "16px 18px", minWidth: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 7, marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)" }}>{label}</span>
      </div>
      <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", gap: 10 }}>
        <div className="display" style={{ fontSize: 36, fontWeight: 300, lineHeight: 1, color: tone }}>{value}</div>
        {children}
      </div>
      <div style={{ fontSize: 11.5, color: "var(--ink-3)", marginTop: 9, lineHeight: 1.4 }}>{sub}</div>
    </div>
  );

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="観" eyebrow="Org · monitor" title="Hive-mind health"
        sub="Three headline signals — throughput, adoption, and leak-guard — read over the full audit trail. Anomalies surface here and flow straight into the client lead's incident view."
        right={<div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Last 7 days ▾</DojoChip>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, fontFamily: "var(--font-mono)",
                        color: "var(--success)", background: "var(--success-soft)", border: "1px solid var(--success-edge)",
                        borderRadius: 20, padding: "3px 10px" }}>● leak-guard armed</span>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        {/* three headline signals */}
        <div style={{ display: "flex", gap: 14 }}>
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
            <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 4 }}>
              <div style={{ display: "flex", gap: 4 }}>
                {alerts.map((a, i) => <span key={i} style={{ width: 9, height: 9, borderRadius: "50%", background: sevTone[a.sev] }} />)}
              </div>
              <span className="mono" style={{ fontSize: 9.5, color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: ".06em" }}>2 exceptions · 1 held</span>
            </div>
          </Signal>
        </div>

        {/* trend + leak-guard feed */}
        <div style={{ display: "grid", gridTemplateColumns: "1.35fr 1fr", gap: 18, marginTop: 18 }}>
          {/* throughput trend */}
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 14, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
              <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Contributions vs. approvals</span>
              <span style={{ flex: 1 }} />
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--ink-3)" }}><span style={{ width: 9, height: 9, borderRadius: 2, background: "var(--accent)", opacity: .85 }} /> contributed</span>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--ink-3)" }}><span style={{ width: 9, height: 9, borderRadius: 2, background: "var(--success)" }} /> approved</span>
            </div>
            <div style={{ padding: "20px 18px 16px" }}>
              <div style={{ display: "flex", alignItems: "flex-end", gap: 16, height: 122 }}>
                {weeks.map((w, i) => (
                  <div key={i} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: 7 }}>
                    <div style={{ height: 96, width: "100%", display: "flex", alignItems: "flex-end", justifyContent: "center", gap: 5 }}>
                      <div style={{ width: 11, height: Math.round(w.c / maxBar * 96), background: "var(--accent)", opacity: .85, borderRadius: "3px 3px 0 0" }} />
                      <div style={{ width: 11, height: Math.round(w.a / maxBar * 96), background: "var(--success)", borderRadius: "3px 3px 0 0" }} />
                    </div>
                    <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{days[i]}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* leak-guard alert feed */}
          <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden", display: "flex", flexDirection: "column" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
              <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>警</span>
              <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Leak-guard alerts</span>
              <span style={{ flex: 1 }} />
              <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>near-leaks &amp; anomalous outbound</span>
            </div>
            <div style={{ flex: 1 }}>
              {alerts.map((a, i) => (
                <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: 11, padding: "12px 16px",
                              borderBottom: i < alerts.length - 1 ? "1px solid var(--edge)" : "none" }}>
                  <span className="kanji" style={{ fontSize: 16, color: sevTone[a.sev], width: 20, textAlign: "center", lineHeight: 1.3 }}>{a.k}</span>
                  <div style={{ minWidth: 0 }}>
                    <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                      <span style={{ fontSize: 13, color: "var(--ink)", flex: 1 }}>{a.title}</span>
                      <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{a.when}</span>
                    </div>
                    <div style={{ fontSize: 11.5, color: "var(--ink-2)", lineHeight: 1.45, marginTop: 3 }}>{a.note}</div>
                    <div style={{ display: "flex", gap: 7, marginTop: 7, alignItems: "center" }}>
                      <DojoChip tone={sevTone[a.sev]} soft={sevSoft[a.sev]}>{a.sev} severity</DojoChip>
                      <DojoChip tone={a.state === "quarantined" ? "oklch(0.52 0.13 60)" : "var(--ink-3)"} soft={a.state === "quarantined" ? "var(--warning-soft)" : "var(--paper-3)"}>{a.state}</DojoChip>
                      <DojoChip tone="var(--accent)" soft="var(--accent-soft)">客 {a.client}</DojoChip>
                    </div>
                  </div>
                </div>
              ))}
            </div>
            <button onClick={() => go && go("audit")} style={{ display: "flex", alignItems: "center", gap: 8, padding: "11px 16px",
                          background: "transparent", border: "none", borderTop: "1px solid var(--edge)",
                          cursor: "pointer", textAlign: "left", width: "100%", fontFamily: "inherit" }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>守</span>
              <span style={{ fontSize: 11.5, color: "var(--ink-3)", flex: 1 }}>Shared into the client lead's incident view</span>
              <span className="mono" style={{ fontSize: 11, color: "var(--accent)" }}>open audit trail →</span>
            </button>
          </div>
        </div>

        {/* signals read over the audit trail */}
        <div style={{ display: "flex", alignItems: "center", gap: 9, marginTop: 18, padding: "13px 16px",
                      background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12 }}>
          <span className="kanji" style={{ fontSize: 15, color: "var(--accent)" }}>録</span>
          <span style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.5, flex: 1 }}>
            Every signal here resolves to entries in the immutable, hash-chained <b style={{ fontWeight: 600, color: "var(--ink)" }}>audit trail</b> — the proof a number is real. Throughput and adoption read from the contribution ledger; leak-guard reads from the dereference log.
          </span>
          <button onClick={() => go && go("audit")} className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "none", cursor: "pointer", whiteSpace: "nowrap" }}>view ledger →</button>
        </div>
      </div>
    </div>
  );
}

/* ─── placeholder for not-yet-designed sections ──────────── */
function DojoSoon({ kanji, label }) {
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 12, background: "var(--paper)" }}>
      <span className="kanji" style={{ fontSize: 64, color: "var(--accent)", opacity: 0.35 }}>{kanji}</span>
      <div style={{ fontSize: 15, color: "var(--ink-2)" }}>{label}</div>
      <div style={{ fontSize: 12, color: "var(--ink-4)" }}>Designed in a later pass</div>
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
  else if (section === "knowledge") screen = <DojoSoon kanji="蔵" label="Knowledge — the published library" />;
  else screen = <DojoOverview go={go} />;

  return (
    <div className="sensei" data-screen-label="Dōjō Console" style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoTopBar org={org} />
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <DojoNav active={section === "triage" && candidate ? "triage" : section} setActive={(s) => go(s)} />
        <div style={{ flex: 1, minWidth: 0 }}>{screen}</div>
      </div>
    </div>
  );
}

Object.assign(window, {
  DojoConsole, DojoOverview, DojoTriage, DojoCandidate, DojoMembers, DojoClients, DojoScopes, DojoAudit, DojoMonitor,
});
