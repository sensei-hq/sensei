// Dōjō · Developer console — the individual's view of the SaaS Dōjō.
// Every user can log in; the Dōjō's benefit is primarily for teams & orgs, but
// a developer belongs to MANY of them, so they get a personal, read-mostly view:
//   My teams          — every membership (employer · clients · communities ·
//                        personal) and what they follow.
//   My contributions  — what they've sent upstream + its status per destination.
//   For me            — approved teachings distributed down to them.
// Contribute/approve/publish stay with maintainers & admins; this is the
// contributor seat. Reuses the shared frame from dojo-shared.jsx.

const { useState: ddS } = React;

const DEV_NAV = [
  { group: "Me", items: [
    { id: "teams",         kanji: "群", label: "My teams" },
    { id: "contributions", kanji: "共", label: "My contributions", badge: 2 },
    { id: "downstream",    kanji: "贈", label: "For me" },
  ]},
];

const DEV_ROLE_BY_KIND = {
  employer:  "Contributor",
  client:    "Contributor · dereferenced",
  community: "Member",
  personal:  "Owner",
};
const DEV_FOLLOWS = {
  acme:   "Web · Auth · Payments",
  globex: "lumen-auth · billing",
  initech:"initech-portal",
  rustco: "rust · axum · sqlx",
  self:   "everything (private)",
};

/* ─── My teams ───────────────────────────────────────────── */
function DojoDevTeams({ go, mobile = false }) {
  const D = window.DOJO;
  const kindTone = { employer: "var(--ink-2)", client: "var(--accent)", community: "var(--success)", personal: "var(--ink-3)" };
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="群" eyebrow="You · memberships" title="Your teams & orgs"
        sub="One login, every Dōjō you belong to. A project routes only to the membership it's bound to — findings never cross into an unrelated hive-mind."
        right={<DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">{D.memberships.length} memberships</DojoChip>} />
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? 16 : 28 }}>
        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(2, 1fr)", gap: 14 }}>
          {D.memberships.map(m => (
            <div key={m.id} style={{ background: "var(--paper-2)", border: "var(--hairline)",
                  borderLeft: `3px solid ${kindTone[m.kind]}`, borderRadius: 12, padding: "16px 18px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 11 }}>
                <span className="kanji" style={{ fontSize: 24, color: kindTone[m.kind], lineHeight: 1 }}>{m.kanji}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 15, color: "var(--ink)", display: "flex", alignItems: "center", gap: 8 }}>
                    {m.name}{m.current && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">active</DojoChip>}
                  </div>
                  <div className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: ".06em", marginTop: 2 }}>{m.kind}</div>
                </div>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "8px 12px", marginTop: 14, fontSize: 12.5 }}>
                <span style={{ color: "var(--ink-4)" }}>Role</span>
                <span style={{ color: "var(--ink)" }}>{DEV_ROLE_BY_KIND[m.kind]}</span>
                <span style={{ color: "var(--ink-4)" }}>Following</span>
                <span style={{ color: "var(--ink-2)" }}>{DEV_FOLLOWS[m.id]}</span>
              </div>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 9, marginTop: 16, padding: "13px 16px",
                      background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12 }}>
          <span className="kanji" style={{ fontSize: 15, color: "var(--accent)" }}>客</span>
          <span style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.5, flex: 1 }}>
            On <b style={{ fontWeight: 600, color: "var(--ink)" }}>client</b> memberships your contributions are automatically source-dereferenced — the lesson travels, the client and repo never do.
          </span>
        </div>
      </div>
    </div>
  );
}

/* ─── My contributions ───────────────────────────────────── */
function DojoDevContributions() {
  const mine = [
    { k: "紋", title: "Adapter wraps a third-party SDK behind a trait", dest: "Acme Corp", scope: "Stack · Rust", status: "approved", when: "2d", note: "published · +7pp FTR" },
    { k: "直", title: "`let` → `$state(...)` in Svelte 5 components", dest: "Rust Guild", scope: "Stack · Svelte", status: "pending", when: "6h", note: "in triage · owner Sven K." },
    { k: "盾", title: "Verify webhook signature before parsing", dest: "Globex", scope: "Client · dereferenced", status: "approved", when: "1d", note: "source dropped · shared safely", client: true },
    { k: "問", title: "Persona: integration-test author for auth", dest: "Acme Corp", scope: "Stack · React", status: "declined", when: "3d", note: "merged into an existing persona" },
  ];
  const statusMeta = {
    approved: { tone: "var(--success)", soft: "var(--success-soft)", label: "approved" },
    pending:  { tone: "var(--accent)",  soft: "var(--accent-soft)",  label: "in triage" },
    declined: { tone: "var(--ink-3)",   soft: "var(--paper-3)",      label: "declined" },
  };
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="共" eyebrow="You · upstream" title="What you've shared"
        sub="Lessons you sent up to a Dōjō, and where each one stands. You propose; a maintainer decides — nothing publishes without their named approval."
        right={<div style={{ textAlign: "right", fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--ink-3)", lineHeight: 1.7 }}>
          <div><b style={{ color: "var(--success)" }}>2</b> approved · <b style={{ color: "var(--accent)" }}>1</b> pending</div>
          <div>612 devs helped · lifetime</div>
        </div>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
          {mine.map((c, i) => {
            const sm = statusMeta[c.status];
            return (
              <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto auto", gap: 14, alignItems: "center",
                    padding: "14px 16px", borderBottom: i < mine.length - 1 ? "1px solid var(--edge)" : "none" }}>
                <span className="kanji" style={{ fontSize: 18, color: "var(--accent)", width: 22, textAlign: "center" }}>{c.k}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13.5, color: "var(--ink)" }}>{c.title}</div>
                  <div style={{ display: "flex", gap: 8, marginTop: 5, alignItems: "center", flexWrap: "wrap" }}>
                    <DojoChip tone={c.client ? "var(--accent)" : "var(--ink-2)"} soft={c.client ? "var(--accent-soft)" : "var(--paper-3)"}>{c.client && "盾 "}{c.dest}</DojoChip>
                    <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{c.scope} · {c.note}</span>
                  </div>
                </div>
                <DojoChip tone={sm.tone} soft={sm.soft}>{sm.label}</DojoChip>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)", width: 28, textAlign: "right" }}>{c.when}</span>
              </div>
            );
          })}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 9, marginTop: 16, padding: "13px 16px",
                      background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12 }}>
          <span className="kanji" style={{ fontSize: 15, color: "var(--accent)" }}>芽</span>
          <span style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.5, flex: 1 }}>
            You share from the Observatory's <b style={{ fontWeight: 600, color: "var(--ink)" }}>ready-to-share</b> lane; it lands in the bound Dōjō's triage queue. Track the outcome here.
          </span>
        </div>
      </div>
    </div>
  );
}

/* ─── For me · downstream ────────────────────────────────── */
function DojoDevDownstream() {
  const items = [
    { k: "守", title: "Never log refresh tokens, even at debug level", from: "Acme Corp", scope: "Company", when: "8m", adopted: false, kind: "guard" },
    { k: "紋", title: "Idempotency key on money-moving mutations", from: "Acme Corp", scope: "Team · Payments", when: "4h", adopted: true, kind: "pattern" },
    { k: "技", title: "Skill: explain a slow query plan", from: "Rust Guild", scope: "Stack · Postgres", when: "1d", adopted: false, kind: "skill" },
  ];
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="贈" eyebrow="You · downstream" title="Approved for you"
        sub="Practice your teams approved, distributed to every scope you're in. It arrives in your Observatory's Today & Upgrades — mute or pin anything that doesn't fit your work."
        right={<DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">across 4 memberships</DojoChip>} />
      <div style={{ flex: 1, overflow: "auto", padding: 28 }}>
        <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
          {items.map((it, i) => (
            <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto auto", gap: 14, alignItems: "center",
                  padding: "14px 16px", borderBottom: i < items.length - 1 ? "1px solid var(--edge)" : "none" }}>
              <span className="kanji" style={{ fontSize: 18, color: "var(--accent)", width: 22, textAlign: "center" }}>{it.k}</span>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontSize: 13.5, color: "var(--ink)" }}>{it.title}</div>
                <div style={{ display: "flex", gap: 8, marginTop: 5, alignItems: "center", flexWrap: "wrap" }}>
                  <DojoChip tone="var(--ink-2)">{it.from}</DojoChip>
                  <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{it.scope} · {it.when} ago</span>
                </div>
              </div>
              {it.adopted
                ? <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ adopted</DojoChip>
                : <DojoChip tone="var(--accent)" soft="var(--accent-soft)">new</DojoChip>}
              <div style={{ display: "flex", gap: 7 }}>
                <button className="mono" style={{ fontSize: 11, color: "var(--ink-3)", background: "none", border: "var(--hairline)", borderRadius: 6, padding: "5px 9px", cursor: "pointer" }}>mute</button>
                <button className="mono" style={{ fontSize: 11, color: "var(--accent)", background: "none", border: "1px solid oklch(0.58 0.15 35/.3)", borderRadius: 6, padding: "5px 9px", cursor: "pointer" }}>pin</button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/* ─── the developer console ──────────────────────────────── */
function DojoDeveloperConsole({ initial = "teams", mobile = false, relayStart = null }) {
  const [active, setActive] = ddS(initial);
  let screen;
  if (active === "contributions") screen = <DojoDevContributions />;
  else if (active === "downstream") screen = <DojoDevDownstream />;
  else screen = <DojoDevTeams mobile={mobile} />;
  return (
    <DojoRoleShell label="Dōjō · Developer console" role={{ kanji: "弟", label: "Developer" }}
      nav={DEV_NAV} active={active} setActive={setActive} mobile={mobile} relayStart={relayStart}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoDeveloperConsole, DojoDevTeams, DojoDevContributions, DojoDevDownstream });
