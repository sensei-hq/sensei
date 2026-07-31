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
  client:    "Contributor · anonymized",
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
  const kindTone = { employer: "var(--ink-soft)", client: "var(--accent)", community: "var(--success)", personal: "var(--ink-mute)" };
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="群" eyebrow="You · memberships" title="Your teams & orgs"
        sub="One login, every Dōjō you belong to. A project routes only to the membership it's bound to — findings never cross into an unrelated hive-mind."
        right={<DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">{D.memberships.length} memberships</DojoChip>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <div className="grid gap-3" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(2, 1fr)" }}>
          {D.memberships.map(m => (
            <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4" key={m.id} style={{
 borderLeft: `3px solid ${kindTone[m.kind]}` }}>
              <div className="flex items-center gap-3" >
                <span className="kanji text-xl" style={{ color: kindTone[m.kind], lineHeight: 1 }}>{m.kanji}</span>
                <div className="flex-1 min-w-0" >
                  <div className="text-base text-ink flex items-center gap-2" >
                    {m.name}{m.current && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">active</DojoChip>}
                  </div>
                  <div className="mono text-xs text-ink-faint uppercase mt-1" style={{ letterSpacing: ".06em" }}>{m.kind}</div>
                </div>
              </div>
              <div className="grid mt-3 text-sm" style={{ gridTemplateColumns: "auto 1fr", gap: "var(--space-2) var(--space-3)" }}>
                <span className="text-ink-faint" >Role</span>
                <span className="text-ink" >{DEV_ROLE_BY_KIND[m.kind]}</span>
                <span className="text-ink-faint" >Following</span>
                <span className="text-ink-soft" >{DEV_FOLLOWS[m.id]}</span>
              </div>
            </div>
          ))}
        </div>
        <div className="flex items-center gap-2 mt-4 py-3 px-4 bg-paper-soft border border-paper-edge rounded-lg" >
          <span className="kanji text-base text-accent" >客</span>
          <span className="text-sm text-ink-soft flex-1" style={{ lineHeight: 1.5 }}>
            On <b className="font-semibold text-ink" >client</b> memberships your contributions are automatically anonymized — the lesson travels, the client and repo never do.
          </span>
        </div>
      </div>
    </div>
  );
}

/* ─── My contributions ───────────────────────────────────── */
function DojoDevContributions({ mobile = false }) {
  const mine = [
    { k: "紋", title: "Adapter wraps a third-party SDK behind a trait", dest: "Acme Corp", scope: "Stack · Rust", status: "approved", when: "2d", note: "published · +7pp FTR" },
    { k: "直", title: "`let` → `$state(...)` in Svelte 5 components", dest: "Rust Guild", scope: "Stack · Svelte", status: "pending", when: "6h", note: "in triage · owner Sven K." },
    { k: "盾", title: "Verify webhook signature before parsing", dest: "Globex", scope: "Client · anonymized", status: "approved", when: "1d", note: "anonymized · shared safely", client: true },
    { k: "問", title: "Persona: integration-test author for auth", dest: "Acme Corp", scope: "Stack · React", status: "declined", when: "3d", note: "merged into an existing persona" },
  ];
  const statusMeta = {
    approved: { tone: "var(--success)", soft: "var(--success-soft)", label: "approved" },
    pending:  { tone: "var(--accent)",  soft: "var(--accent-soft)",  label: "in triage" },
    declined: { tone: "var(--danger)",  soft: "var(--danger-soft)", label: "declined" },
  };
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="共" eyebrow="You · upstream" title="What you've shared"
        sub="Lessons you sent up to a Dōjō, and where each one stands. You propose; a maintainer decides — nothing publishes without their named approval."
        right={<div className="text-right text-xs text-ink-mute" style={{ fontFamily: "var(--font-mono)", lineHeight: 1.7 }}>
          <div><b className="text-success" >2</b> approved · <b className="text-accent" >1</b> pending</div>
          <div>612 devs helped · lifetime</div>
        </div>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {mine.map((c, i) => {
            const sm = statusMeta[c.status];
            return (
              <div className="grid gap-3 items-center py-3 px-4" key={i} style={{ gridTemplateColumns: mobile ? "auto 1fr" : "auto 1fr auto auto", borderBottom: i < mine.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{c.k}</span>
                <div className="min-w-0" >
                  <div className="text-sm text-ink" >{c.title}</div>
                  <div className="flex gap-2 mt-1 items-center flex-wrap" >
                    <DojoChip tone={c.client ? "var(--accent)" : "var(--ink-soft)"} soft={c.client ? "var(--accent-soft)" : "var(--paper-mute)"}>{c.client && "盾 "}{c.dest}</DojoChip>
                    {mobile && <DojoChip tone={sm.tone} soft={sm.soft}>{sm.label}</DojoChip>}
                    <span className="mono text-xs text-ink-faint" >{c.scope} · {c.note}{mobile ? " · " + c.when : ""}</span>
                  </div>
                </div>
                {!mobile && <DojoChip tone={sm.tone} soft={sm.soft}>{sm.label}</DojoChip>}
                {!mobile && <span className="mono text-xs text-ink-faint text-right" style={{ width: 28 }}>{c.when}</span>}
              </div>
            );
          })}
        </div>
        <div className="flex items-center gap-2 mt-4 py-3 px-4 bg-paper-soft border border-paper-edge rounded-lg" >
          <span className="kanji text-base text-accent" >芽</span>
          <span className="text-sm text-ink-soft flex-1" style={{ lineHeight: 1.5 }}>
            You share from the Observatory's <b className="font-semibold text-ink" >ready-to-share</b> lane; it lands in the bound Dōjō's triage queue. Track the outcome here.
          </span>
        </div>
      </div>
    </div>
  );
}

/* ─── For me · downstream ────────────────────────────────── */
function DojoDevDownstream({ mobile = false }) {
  const items = [
    { k: "守", title: "Never log refresh tokens, even at debug level", from: "Acme Corp", scope: "Company", when: "8m", adopted: false, kind: "guard" },
    { k: "紋", title: "Idempotency key on money-moving mutations", from: "Acme Corp", scope: "Team · Payments", when: "4h", adopted: true, kind: "pattern" },
    { k: "技", title: "Skill: explain a slow query plan", from: "Rust Guild", scope: "Stack · Postgres", when: "1d", adopted: false, kind: "skill" },
  ];
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="贈" eyebrow="You · downstream" title="Approved for you"
        sub="Practice your teams approved, distributed to every scope you're in. It arrives in your Observatory's Today & Upgrades — mute or pin anything that doesn't fit your work."
        right={<DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">across 4 memberships</DojoChip>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {items.map((it, i) => (
            <div className="grid gap-3 items-center py-3 px-4" key={i} style={{ gridTemplateColumns: mobile ? "auto 1fr" : "auto 1fr auto auto", borderBottom: i < items.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{it.k}</span>
              <div className="min-w-0" >
                <div className="text-sm text-ink" >{it.title}</div>
                <div className="flex gap-2 mt-1 items-center flex-wrap" >
                  <DojoChip tone="var(--ink-soft)">{it.from}</DojoChip>
                  {mobile && (it.adopted
                    ? <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ adopted</DojoChip>
                    : <DojoChip tone="var(--accent)" soft="var(--accent-soft)">new</DojoChip>)}
                  <span className="mono text-xs text-ink-faint" >{it.scope} · {it.when} ago</span>
                  {mobile && (
                    <span className="inline-flex gap-2" >
                      <button className="mono text-xs text-ink-mute border border-paper-edge rounded py-1 px-2 cursor-pointer" style={{ background: "none" }}>mute</button>
                      <button className="mono text-xs text-accent rounded py-1 px-2 cursor-pointer" style={{ background: "none", border: "1px solid var(--accent-edge)" }}>pin</button>
                    </span>
                  )}
                </div>
              </div>
              {!mobile && (it.adopted
                ? <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ adopted</DojoChip>
                : <DojoChip tone="var(--accent)" soft="var(--accent-soft)">new</DojoChip>)}
              {!mobile && <div className="flex gap-2" >
                <button className="mono text-xs text-ink-mute border border-paper-edge rounded py-1 px-2 cursor-pointer" style={{ background: "none" }}>mute</button>
                <button className="mono text-xs text-accent rounded py-1 px-2 cursor-pointer" style={{ background: "none", border: "1px solid var(--accent-edge)" }}>pin</button>
              </div>}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/* ─── the developer console ──────────────────────────────── */
function DojoDeveloperConsole({ initial = "teams", mobile = false, relayStart = null, onEnterDojo, onOpenProject }) {
  const [active, setActive] = ddS(initial);
  let screen;
  if (active === "contributions") screen = <DojoDevContributions mobile={mobile} />;
  else if (active === "downstream") screen = <DojoDevDownstream mobile={mobile} />;
  else screen = <DojoDevTeams mobile={mobile} onEnterDojo={onEnterDojo} />;
  const nav = onOpenProject ? [...DEV_NAV, { group: "Your work", items: [{ id: "__project", kanji: "序", label: "Project rules" }] }] : DEV_NAV;
  const onSetActive = (id) => { if (id === "__project") { onOpenProject && onOpenProject(); } else setActive(id); };
  return (
    <DojoRoleShell label="Dōjō · Developer console" role={{ kanji: "弟", label: "Developer" }}
      nav={nav} active={active} setActive={onSetActive} mobile={mobile} relayStart={relayStart} onEnterDojo={onEnterDojo}>
      {screen}
    </DojoRoleShell>
  );
}

Object.assign(window, { DojoDeveloperConsole, DojoDevTeams, DojoDevContributions, DojoDevDownstream });
