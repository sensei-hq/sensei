// Dōjō — shared chrome & primitives for the per-role consoles.
// Split out of the old monolithic dojo-console.jsx so each role file
// (dojo-admin · dojo-maintainer · dojo-lead) defines ONLY its own panels
// and reuses this one copy of the frame — no component name is defined twice.
//
// STYLING CONVENTION (design-system): semantic utility classes for color
// and type (text-ink-*, bg-paper-*, text-xs/sm, zs-eyebrow, zs-meta) and
// zs-* components; inline style is reserved for geometry the scale doesn't
// model (fixed control dimensions, asymmetric chip padding, offsets).
//
// Exports (window): DOJO_ORIGIN · DOJO_TYPE · DojoChip · OriginChip ·
// Confidence · DojoHead · DojoTopBar · DojoRoleNav · DojoRoleShell.

/* ─── attribution / type vocab ──────────────────────────── */
const DOJO_ORIGIN = {
  employer:  { label: "Employer",     tone: "var(--ink-soft)", soft: "var(--paper-mute)" },
  client:    { label: "Anonymized", tone: "var(--accent)",  soft: "var(--accent-soft)" },
  community: { label: "Community",    tone: "var(--success)", soft: "var(--success-soft)" },
  oss:       { label: "Open source",  tone: "var(--ink-soft)", soft: "var(--paper-mute)" },
};
const DOJO_TYPE = {
  guard: "Guard", pattern: "Pattern", principle: "Principle",
  prompt: "Prompt", skill: "Skill", agent: "Agent",
};

function DojoChip({ children, tone = "var(--ink-mute)", soft = "var(--paper-mute)", border }) {
  return (
    <span className="mono text-xs inline-flex items-center gap-1 rounded-full whitespace-nowrap" style={{
 letterSpacing: ".04em", color: tone, background: soft,
 border: border || "1px solid transparent", padding: "4px 8px" }}>{children}</span>
  );
}
function OriginChip({ origin }) {
  const o = DOJO_ORIGIN[origin] || DOJO_ORIGIN.employer;
  return <DojoChip tone={o.tone} soft={o.soft}>{origin === "client" && "盾 "}{o.label}</DojoChip>;
}
function Confidence({ v, w = 84 }) {
  const tone = v >= 0.85 ? "var(--success)" : v >= 0.7 ? "var(--accent)" : "var(--warning)";
  return (
    <div className="flex items-center gap-2">
      <div className="bg-paper-mute rounded-full overflow-hidden" style={{ width: w, height: 4 }}>
        <div className="rounded-full h-full" style={{ width: (v * 100) + "%", background: tone }} />
      </div>
      <span className="mono text-xs text-ink-soft">{Math.round(v * 100)}</span>
    </div>
  );
}
function DojoHead({ kanji, eyebrow, title, sub, right, mobile = false }) {
  return (
    <div className={"flex items-start border-b " + (mobile ? "flex-wrap gap-3" : "gap-4")}
         style={{ padding: mobile ? "var(--space-4) var(--space-4) var(--space-3)" : "var(--space-6) var(--space-8) var(--space-4)", flexShrink: 0 }}>
      <span className="kanji text-accent shrink-0" style={{ fontSize: mobile ? "var(--text-2xl)" : "var(--text-3xl)", lineHeight: 1 }}>{kanji}</span>
      <div className="flex-1" style={{ minWidth: mobile ? 180 : 0 }}>
        <div className="zs-eyebrow mb-1">{eyebrow}</div>
        <h1 className={"display font-normal tracking-tight " + (mobile ? "text-lg" : "text-xl")} style={{ margin: 0, lineHeight: 1.15 }}>{title}</h1>
        {sub && <p className="zs-body-sm mt-2 mb-0" style={{ maxWidth: mobile ? "100%" : 680 }}>{sub}</p>}
      </div>
      {right && <div className="flex flex-wrap gap-2 shrink-0" style={{ width: mobile ? "100%" : "auto" }}>{right}</div>}
    </div>
  );
}

/* ─── top bar (org switcher + role) ─────────────────────── */
function DojoTopBar({ org, role, onEnterDojo }) {
  const D = window.DOJO;
  const [swOpen, setSwOpen] = React.useState(false);
  const mem = D.memberships || [];
  return (
    <div className="flex items-center gap-4 border-b bg-paper px-4 shrink-0" style={{ height: 54 }}>
      <div className="flex items-baseline gap-2">
        <span className="kanji text-accent text-xl" style={{ lineHeight: 1 }}>結</span>
        <span className="display text-lg tracking-tight">Dōjō</span>
      </div>
      <div className="ml-1 relative" >
        <button onClick={() => setSwOpen(o => !o)}
          className={"inline-flex items-center gap-2 bg-paper-soft rounded " + (swOpen ? "border-accent" : "border border-paper-edge")}
          style={{ padding: "var(--space-1) var(--space-3)", minHeight: 32 }}>
          <span className="kanji text-accent text-sm">{org.kanji}</span>
          <span className="text-sm text-ink">{org.name}</span>
          <span className="mono text-xs text-ink-faint uppercase" style={{ letterSpacing: ".08em" }}>{org.kind || "employer"}</span>
          <span className="text-xs text-ink-mute">▾</span>
        </button>
        {swOpen && (
          <div className="bg-paper border border-paper-edge rounded-lg shadow-lg absolute overflow-hidden" style={{ top: "calc(100% + 6px)", left: 0, width: 300, zIndex: 50 }}>
            <div className="flex items-center gap-2 border-b py-2 px-3" >
              <span className="kanji text-sm text-ink-mute">探</span>
              <span className="flex-1 text-sm text-ink-faint">Switch Dōjō…</span>
              <span className="mono text-xs text-ink-faint bg-paper-mute rounded-sm" style={{ padding: "4px 8px" }}>⌘K</span>
            </div>
            <button className="flex items-center gap-3 w-full text-left bg-accent-soft border-b py-2 px-3" >
              <span className="kanji text-accent text-base">場</span>
              <div className="flex-1 min-w-0" >
                <div className="text-sm text-ink font-medium">Relay · you</div>
                <div className="text-xs text-ink-mute">all Dōjōs · no switching needed</div>
              </div>
            </button>
            <div className="py-1 overflow-auto" style={{ maxHeight: 280 }}>
              {mem.map(m => {
                const on = m.current;
                return (
                  <button key={m.id} onClick={() => onEnterDojo && onEnterDojo(m)} className={"flex items-center gap-3 w-full text-left " + (on ? "bg-paper-soft" : "")} style={{ padding: "var(--space-2) var(--space-3)" }}>
                    <span className="kanji text-accent text-sm text-center" style={{ width: 18 }}>{m.kanji}</span>
                    <div className="flex-1 min-w-0" >
                      <div className="text-sm text-ink whitespace-nowrap overflow-hidden text-ellipsis" >{m.name}</div>
                      <div className="mono text-xs text-ink-faint">{m.kind || "member"}</div>
                    </div>
                    {on && <span className="text-sm text-accent">✓</span>}
                  </button>
                );
              })}
            </div>
            <button className="flex items-center gap-3 w-full text-left border-t py-2 px-3" >
              <span className="kanji text-sm text-ink-mute text-center" style={{ width: 18 }}>群</span>
              <div className="flex-1 min-w-0" >
                <div className="text-sm text-ink">Your Dōjōs</div>
                <div className="text-xs text-ink-faint">see &amp; manage all</div>
              </div>
              <span className="text-sm text-ink-faint">→</span>
            </button>
            <button className="flex items-center gap-2 w-full text-left border-t text-sm text-ink-soft p-3" >
              <span className="text-accent">＋</span> Create or join a Dōjō
            </button>
          </div>
        )}
      </div>
      {role && (
        <span className="mono text-xs text-accent bg-accent-soft rounded-full inline-flex items-center gap-1" style={{ border: "1px solid var(--accent-edge)", padding: "4px 12px" }}>{role.kanji} {role.label}</span>
      )}
      <div className="flex-1" />
      <div className="zs-input" style={{ width: 240 }}>
        <span className="kanji text-sm text-ink-mute">探</span>
        <span className="text-ink-faint">search knowledge…</span>
      </div>
      <span className="zs-meta">{D.org.members} members</span>
      <Avatar name="Keiko" size={28} />
      <button title="Log out" className="zs-btn zs-btn-sm zs-btn-ghost border border-paper-edge">
        <span className="kanji text-ink-mute">出</span>Log out
      </button>
    </div>
  );
}

/* ─── role-scoped left nav (all items live) ─────────────── */
function DojoRoleNav({ nav, active, setActive, header }) {
  return (
    <aside className="flex flex-col border-r bg-paper-soft shrink-0 py-4 px-3 overflow-auto" style={{ width: 218 }}>
      {header}
      {(() => {
        const renderGroup = grp => (
        <div key={grp.group} className="mb-4" style={{ opacity: grp.manage ? 0.82 : 1 }}>
          <div className="zs-eyebrow font-semibold px-2 mb-2">{grp.group}</div>
          <div className="flex flex-col gap-1">
            {grp.items.map(it => {
              const on = active === it.id;
              return (
                <button key={it.id} onClick={() => setActive(it.id)}
                  className={"w-full text-left rounded text-sm " + (on ? "bg-paper text-ink border border-paper-edge" : "text-ink-soft")}
                  style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", alignItems: "center",
                           gap: "var(--space-2)", padding: "var(--space-2)",
                           border: on ? undefined : "1px solid transparent" }}>
                  <span className={"kanji text-sm text-center " + (on ? "text-accent" : "text-ink-mute")} style={{ width: 15 }}>{it.kanji}</span>
                  <span>{it.label}</span>
                  {it.badge != null
                    ? <span className="mono text-xs font-semibold bg-accent rounded-full" style={{ padding: "0 8px", lineHeight: "16px" }}>{it.badge}</span>
                    : <span/>}
                </button>
              );
            })}
          </div>
        </div>
        );
        const top = nav.filter(g => !g.manage);
        const manage = nav.filter(g => g.manage);
        return (
          <React.Fragment>
            {top.map(renderGroup)}
            <div className="flex-1" style={{ minHeight: 12 }} />
            {manage.length > 0 && <div className="border-t mx-2 mb-3" />}
            {manage.map(renderGroup)}
          </React.Fragment>
        );
      })()}
      <button onClick={() => setActive && setActive("identity")}
 className="w-full text-left text-sm text-ink-soft border-t grid items-center gap-2"
 style={{ gridTemplateColumns: "auto 1fr", padding: "var(--space-3) var(--space-2) var(--space-2)" }}>
        <span className="kanji text-sm text-ink-mute text-center" style={{ width: 15 }}>調</span>
        <span>Settings · SSO</span>
      </button>
      <div className="mono text-xs text-ink-faint flex items-center gap-1" style={{ padding: "var(--space-2) var(--space-2) 0" }}>
        <span className="kanji">結</span>Dōjō v0.4.2
      </div>
    </aside>
  );
}

/* ─── mobile shell — condensed bar + bottom tab nav ─────── */
function DojoMobileBar({ role, live = true }) {
  const D = window.DOJO;
  const org = D.memberships.find(m => m.current) || D.memberships[0];
  return (
    <div className="flex items-center gap-3 border-b bg-paper shrink-0 py-3 px-4" >
      <span className="kanji text-accent text-xl" style={{ lineHeight: 1 }}>結</span>
      <div className="flex-1 min-w-0" >
        <div className="text-sm text-ink font-semibold whitespace-nowrap overflow-hidden text-ellipsis" style={{ lineHeight: 1.1 }}>{org.name}</div>
        {role && <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{role.label}</div>}
      </div>
      {live && <DojoLive />}
      <Avatar name="Rin Saito" size={28} />
    </div>
  );
}
function DojoMobileTabs({ nav, active, setActive }) {
  const items = [].concat(...nav.map(g => g.items)).slice(0, 5);
  return <DojoTabBar tabs={items} active={active} onNav={setActive} />;
}

/* Canonical bottom tab bar — one recipe for the console + relay mobile shells. */
const DOJO_MOBILE_TABS = [
  { id: "projects", kanji: "場", label: "Projects" },
  { id: "inbox", kanji: "決", label: "Inbox", badge: 3 },
  { id: "chat", kanji: "話", label: "Chat" },
  { id: "more", kanji: "⋯", label: "More" },
];
function DojoTabBar({ tabs, active, onNav }) {
  return (
    <div className="grid border-t bg-paper shrink-0" style={{ gridTemplateColumns: `repeat(${tabs.length}, 1fr)` }}>
      {tabs.map(it => {
        const on = active === it.id;
        return (
          <button key={it.id} onClick={() => onNav && onNav(it.id)}
            className={"flex flex-col items-center gap-1 " + (on ? "text-ink" : "text-ink-mute")}
            style={{ padding: "var(--space-2) var(--space-1) var(--space-3)", position: "relative" }}>
            <span className={"kanji text-lg " + (on ? "text-accent" : "text-ink-mute")}>{it.kanji}</span>
            <span className={"text-xs " + (on ? "font-semibold" : "font-normal")} style={{ whiteSpace: "nowrap" }}>{it.label.split(" ")[0]}</span>
            {it.badge != null && (
              <span className="mono text-xs font-semibold bg-accent rounded-full absolute" style={{ top: 5, right: "50%", marginRight: -18, padding: "0 8px", lineHeight: "14px" }}>{it.badge}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}

/* Canonical hairline panel — one recipe for the section cards across
   lead · admin · identity (replaces three local Panel/IdPanel copies). */
function DojoPanel({ title, note, right, align = "center", children }) {
  return (
    <div className="zs-card-flush overflow-hidden" >
      <div className={"flex gap-3 border-b px-4 py-3 " + (align === "baseline" ? "items-baseline" : "items-center")}>
        <span className="zs-eyebrow font-semibold">{title}</span>
        {note && <span className="zs-meta">{note}</span>}
        <span className="flex-1" />{right}
      </div>
      <div className="p-4">{children}</div>
    </div>
  );
}

/* ─── role shell — top bar + nav + main (responsive) ──────
   Injects a shared "Relay · you" group (Projects · Inbox · Chat) at the top of
   every role's nav, so the away-from-keyboard features live in the desktop
   console too — consistent with the mobile bottom tabs. */
const RELAY_GROUP = { group: "Relay · you", items: [
  { id: "__r_projects", kanji: "場", label: "Projects" },
  { id: "__r_inbox", kanji: "決", label: "Inbox", badge: 3 },
  { id: "__r_chat", kanji: "話", label: "Chat" },
] };
const RELAY_IDS = { __r_projects: "projects", __r_inbox: "inbox", __r_chat: "chat" };

/* Management top bar — a deliberately different surface (paper-soft, accent
   top-rule) so stepping into a Dōjō you administer FEELS like a distinct
   place from your own work. Carries the exit back to the You zone. */
function DojoManageBar({ org, role, onExit }) {
  const D = window.DOJO;
  const route = org.url || "sensei-hq.com/" + (org.name || "dojo").toLowerCase().replace(/[^a-z0-9]+/g, "-");
  return (
    <div className="flex items-center gap-4 border-b bg-paper-soft px-4 shrink-0" style={{ height: 54, borderTop: "2px solid var(--accent)" }}>
      <button onClick={onExit} className="zs-btn zs-btn-sm zs-btn-ghost border border-paper-edge" title="Back to your work">
        <span className="text-ink-mute">←</span><span className="kanji text-accent">携</span>Your work
      </button>
      <span className="bg-paper-edge" style={{ width: 1, height: 24 }} />
      <div className="flex items-center gap-2 min-w-0" >
        <span className="zs-eyebrow font-semibold text-ink-mute">Managing</span>
        <span className="kanji text-accent text-lg" style={{ lineHeight: 1 }}>{org.kanji}</span>
        <span className="display text-lg tracking-tight whitespace-nowrap" >{org.name}</span>
        <span className="mono text-xs text-ink-mute bg-paper rounded-full" style={{ border: "1px solid var(--paper-edge)", padding: "3px 10px" }}>{route}</span>
      </div>
      {role && (
        <span className="mono text-xs text-accent bg-accent-soft rounded-full inline-flex items-center gap-1" style={{ border: "1px solid var(--accent-edge)", padding: "4px 12px" }}>{role.kanji} {role.label}</span>
      )}
      <div className="flex-1" />
      <span className="zs-meta">{D.org.members} members</span>
      <Avatar name="Keiko" size={28} />
      <button title="Log out" className="zs-btn zs-btn-sm zs-btn-ghost border border-paper-edge">
        <span className="kanji text-ink-mute">出</span>Log out
      </button>
    </div>
  );
}
function DojoManageNavHeader({ org, role }) {
  return (
    <div className="mb-4">
      <div className="zs-eyebrow font-semibold text-ink-mute mb-2 pl-2" >Dōjō management</div>
      <div className="flex items-center gap-2 bg-paper border border-paper-edge rounded-lg py-2 px-3" >
        <span className="kanji text-accent text-base">{org.kanji}</span>
        <div className="flex-1 min-w-0" >
          <div className="text-sm text-ink font-medium whitespace-nowrap overflow-hidden text-ellipsis" >{org.name}</div>
          <div className="mono text-xs text-ink-faint">{role.label} · {org.kind || "employer"}</div>
        </div>
      </div>
    </div>
  );
}
function DojoRoleShell({ label, role, nav, active, setActive, children, mobile = false, relay = true, relayStart = null, zone = "you", onExit, onEnterDojo, orgOverride }) {
  const D = window.DOJO;
  const org = orgOverride || D.memberships.find(m => m.current) || D.memberships[0];
  const manage = zone === "dojo";
  const relayOn = relay && !manage;
  const [relayView, setRelayView] = React.useState(relayStart);
  const fullNav = relayOn ? [RELAY_GROUP, ...nav] : nav;
  const activeId = relayView ? ("__r_" + relayView) : active;
  const onNav = (id) => {
    if (RELAY_IDS[id]) setRelayView(RELAY_IDS[id]);
    else { setRelayView(null); if (setActive) setActive(id); }
  };
  const main = (relayView && window.RelayArea)
    ? React.createElement(window.RelayArea, { view: relayView, wide: !mobile, onOpen: (flag) => setRelayView(flag === "approve" ? "approve" : flag === "gate" ? "decision" : flag === "stall" ? "stall" : "watch") })
    : children;
  if (mobile) {
    return (
      <div className="sensei w-full h-full flex flex-col overflow-hidden bg-paper" data-screen-label={label} >
        <DojoMobileBar role={role} />
        <div className="flex flex-col flex-1 min-h-0 overflow-auto" >{main}</div>
        <DojoMobileTabs nav={fullNav} active={activeId} setActive={onNav} />
      </div>
    );
  }
  return (
    <div className="sensei w-full h-full flex flex-col overflow-hidden bg-paper" data-screen-label={label} >
      {manage
        ? <DojoManageBar org={org} role={role} onExit={onExit} />
        : <DojoTopBar org={org} role={role} onEnterDojo={onEnterDojo} />}
      <div className="flex flex-1 min-h-0" >
        <DojoRoleNav nav={fullNav} active={activeId} setActive={onNav}
          header={manage ? <DojoManageNavHeader org={org} role={role} /> : null} />
        <div className="flex-1 min-w-0" >{main}</div>
      </div>
    </div>
  );
}

function DojoLive({ label = "live" }) {
  return (
    <span className="inline-flex items-center gap-1 text-xs text-success bg-success-soft border border-success-edge rounded-full" style={{ padding: "4px 12px" }}>
      <span className="rounded-full bg-success" style={{ width: 6, height: 6 }} />{label}
    </span>
  );
}

// Canonical button — built on the zs-btn component classes.
// variant: primary (ink) · ghost (hairline) · danger. size sm|md.
function DojoBtn({ variant = "primary", size = "md", kanji, children, onClick, style }) {
  const cls = "zs-btn "
    + (size === "sm" ? "zs-btn-sm " : "")
    + (variant === "primary" ? "zs-btn-primary" : variant === "ghost" ? "bg-paper border border-paper-edge" : "");
  const skin = variant === "danger" ? { background: "var(--danger)", color: "var(--paper)" } : null;
  const kc = variant === "danger" ? "var(--paper)" : "var(--accent)";
  return (
    <button onClick={onClick} className={cls} style={{ justifyContent: "center", ...skin, ...style }}>
      {kanji && <span className="kanji" style={{ color: kc }}>{kanji}</span>}{children}
    </button>
  );
}

/* Per-kind membership tag — kanji + Dōjō name, tinted by kind (the one map). */
const DOJO_KIND_TONE = { Employer: "var(--ink-soft)", Client: "var(--accent)", Community: "var(--success)", Personal: "var(--ink-mute)", Solo: "var(--ink-mute)" };
const DOJO_KIND_KANJI = { Employer: "社", Client: "客", Community: "群", Personal: "己", Solo: "己" };
const DOJO_KIND_SOFT = { Employer: "var(--paper-mute)", Client: "var(--accent-soft)", Community: "var(--success-soft)", Personal: "var(--paper-mute)", Solo: "var(--paper-mute)" };
const DOJO_KIND_EDGE = { Employer: "var(--paper-edge)", Client: "var(--accent-edge)", Community: "var(--success-edge)", Personal: "var(--paper-edge)", Solo: "var(--paper-edge)" };
function DojoKindTag({ p }) {
  const tone = DOJO_KIND_TONE[p.kind] || "var(--ink-mute)";
  const soft = DOJO_KIND_SOFT[p.kind] || "var(--paper-mute)";
  const edge = DOJO_KIND_EDGE[p.kind] || "var(--paper-edge)";
  return (
    <span className="inline-flex items-center gap-1 text-xs rounded-full whitespace-nowrap" style={{ color: tone,
 background: soft, border: `1px solid ${edge}`, padding: "4px 8px" }}>
      <span className="kanji text-xs">{DOJO_KIND_KANJI[p.kind] || "結"}</span>{p.dojo}
    </span>
  );
}

Object.assign(window, {
  DOJO_ORIGIN, DOJO_TYPE, DOJO_KIND_TONE, DOJO_KIND_KANJI, DOJO_KIND_SOFT, DOJO_KIND_EDGE, DojoChip, OriginChip, Confidence, DojoHead, DojoLive, DojoBtn, DojoKindTag,
  DojoTopBar, DojoRoleNav, DojoRoleShell, DojoManageBar, DojoMobileBar, DojoMobileTabs, DojoTabBar, DOJO_MOBILE_TABS, DojoPanel,
});
