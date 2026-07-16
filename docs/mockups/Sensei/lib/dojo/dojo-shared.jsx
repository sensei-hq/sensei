// Dōjō — shared chrome & primitives for the per-role consoles.
// Split out of the old monolithic dojo-console.jsx so each role file
// (dojo-admin · dojo-maintainer · dojo-lead) defines ONLY its own panels
// and reuses this one copy of the frame — no component name is defined twice.
//
// Exports (window): DOJO_ORIGIN · DOJO_TYPE · DojoChip · OriginChip ·
// Confidence · DojoHead · DojoTopBar · DojoRoleNav · DojoRoleShell.

/* ─── attribution / type vocab ──────────────────────────── */
const DOJO_ORIGIN = {
  employer:  { label: "Employer",     tone: "var(--ink-2)",   soft: "var(--paper-3)" },
  client:    { label: "Anonymized", tone: "var(--accent)",  soft: "var(--accent-soft)" },
  community: { label: "Community",    tone: "var(--success)", soft: "var(--success-soft)" },
  oss:       { label: "Open source",  tone: "var(--ink-2)",   soft: "var(--paper-3)" },
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
      padding: "2px 8px", display: "inline-flex", alignItems: "center", gap: 5, whiteSpace: "nowrap",
    }}>{children}</span>
  );
}
function OriginChip({ origin }) {
  const o = DOJO_ORIGIN[origin] || DOJO_ORIGIN.employer;
  return <DojoChip tone={o.tone} soft={o.soft}>{origin === "client" && "盾 "}{o.label}</DojoChip>;
}
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
function DojoHead({ kanji, eyebrow, title, sub, right, mobile = false }) {
  return (
    <div style={{ display: "flex", flexWrap: mobile ? "wrap" : "nowrap", alignItems: "flex-start",
                  gap: mobile ? 12 : 18, padding: mobile ? "15px 16px 13px" : "22px 28px 18px",
                  borderBottom: "var(--hairline)", flexShrink: 0 }}>
      <span className="kanji" style={{ fontSize: mobile ? 26 : 38, color: "var(--accent)", lineHeight: 1, flexShrink: 0 }}>{kanji}</span>
      <div style={{ flex: 1, minWidth: mobile ? 180 : 0 }}>
        <div style={{ fontSize: mobile ? 10 : 11, letterSpacing: ".18em", textTransform: "uppercase", color: "var(--ink-3)", marginBottom: 4 }}>{eyebrow}</div>
        <h1 className="display" style={{ fontSize: mobile ? 19 : 24, fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: mobile ? 1.15 : 1.05 }}>{title}</h1>
        {sub && <p style={{ fontSize: mobile ? 12.5 : 13, color: "var(--ink-2)", lineHeight: 1.55, margin: "6px 0 0", maxWidth: mobile ? "100%" : 680 }}>{sub}</p>}
      </div>
      {right && <div style={{ flexShrink: 0, width: mobile ? "100%" : "auto", display: "flex", flexWrap: "wrap", gap: 8 }}>{right}</div>}
    </div>
  );
}

/* ─── top bar (org switcher + role) ─────────────────────── */
function DojoTopBar({ org, role }) {
  const D = window.DOJO;
  const [swOpen, setSwOpen] = React.useState(false);
  const mem = D.memberships || [];
  return (
    <div style={{ height: 54, flexShrink: 0, display: "flex", alignItems: "center", gap: 16,
                  padding: "0 18px", borderBottom: "var(--hairline)", background: "var(--paper)" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 9 }}>
        <span className="kanji" style={{ fontSize: 22, color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: 18, letterSpacing: "-0.01em" }}>Dōjō</span>
      </div>
      <div style={{ position: "relative", marginLeft: 6 }}>
        <button onClick={() => setSwOpen(o => !o)} style={{ display: "inline-flex", alignItems: "center", gap: 8,
          background: "var(--paper-2)", border: swOpen ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: 7, padding: "6px 11px", cursor: "pointer", fontFamily: "inherit" }}>
          <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>{org.kanji}</span>
          <span style={{ fontSize: 13, color: "var(--ink)" }}>{org.name}</span>
          <span className="mono" style={{ fontSize: 9, color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: ".08em" }}>{org.kind || "employer"}</span>
          <span style={{ fontSize: 9, color: "var(--ink-3)" }}>▾</span>
        </button>
        {swOpen && (
          <div style={{ position: "absolute", top: "calc(100% + 6px)", left: 0, width: 300, zIndex: 50,
            background: "var(--paper)", border: "var(--hairline)", borderRadius: 12,
            boxShadow: "var(--shadow-lg, 0 12px 40px rgba(0,0,0,.18))", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "10px 12px", borderBottom: "var(--hairline)" }}>
              <span className="kanji" style={{ fontSize: 12, color: "var(--ink-3)" }}>探</span>
              <span style={{ fontSize: 12.5, color: "var(--ink-4)", flex: 1 }}>Switch Dōjō…</span>
              <span className="mono" style={{ fontSize: 9.5, color: "var(--ink-4)", background: "var(--paper-3)", borderRadius: 5, padding: "2px 6px" }}>⌘K</span>
            </div>
            <button style={{ display: "flex", alignItems: "center", gap: 10, width: "100%", textAlign: "left",
              padding: "10px 12px", background: "var(--accent-soft)", border: "none", borderBottom: "var(--hairline)", cursor: "pointer", fontFamily: "inherit" }}>
              <span className="kanji" style={{ fontSize: 15, color: "var(--accent)" }}>場</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13, color: "var(--ink)", fontWeight: 500 }}>Relay · you</div>
                <div style={{ fontSize: 10.5, color: "var(--ink-3)" }}>all Dōjōs · no switching needed</div>
              </div>
            </button>
            <div style={{ maxHeight: 280, overflow: "auto", padding: "4px 0" }}>
              {mem.map(m => {
                const on = m.current;
                return (
                  <button key={m.id} style={{ display: "flex", alignItems: "center", gap: 10, width: "100%", textAlign: "left",
                    padding: "9px 12px", background: on ? "var(--paper-2)" : "transparent", border: "none", cursor: "pointer", fontFamily: "inherit" }}>
                    <span className="kanji" style={{ fontSize: 14, color: "var(--accent)", width: 18, textAlign: "center" }}>{m.kanji}</span>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 13, color: "var(--ink)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{m.name}</div>
                      <div className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{m.kind || "member"}</div>
                    </div>
                    {on && <span style={{ fontSize: 12, color: "var(--accent)" }}>✓</span>}
                  </button>
                );
              })}
            </div>
            <button style={{ display: "flex", alignItems: "center", gap: 10, width: "100%", textAlign: "left",
              padding: "10px 12px", background: "transparent", border: "none", borderTop: "var(--hairline)", cursor: "pointer", fontFamily: "inherit" }}>
              <span className="kanji" style={{ fontSize: 14, color: "var(--ink-3)", width: 18, textAlign: "center" }}>群</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 12.5, color: "var(--ink)" }}>Your Dōjōs</div>
                <div style={{ fontSize: 10, color: "var(--ink-4)" }}>see &amp; manage all</div>
              </div>
              <span style={{ fontSize: 12, color: "var(--ink-4)" }}>→</span>
            </button>
            <button style={{ display: "flex", alignItems: "center", gap: 9, width: "100%", textAlign: "left",
              padding: "11px 12px", background: "transparent", border: "none", borderTop: "var(--hairline)", cursor: "pointer",
              fontFamily: "inherit", color: "var(--ink-2)", fontSize: 12.5 }}>
              <span style={{ color: "var(--accent)" }}>＋</span> Create or join a Dōjō
            </button>
          </div>
        )}
      </div>
      {role && (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, fontFamily: "var(--font-mono)",
          color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid var(--accent-edge)",
          borderRadius: 20, padding: "3px 11px" }}>{role.kanji} {role.label}</span>
      )}
      <div style={{ flex: 1 }} />
      <div style={{ display: "flex", alignItems: "center", gap: 8, background: "var(--paper-2)",
        border: "var(--hairline)", borderRadius: 7, padding: "6px 11px", width: 240 }}>
        <span className="kanji" style={{ fontSize: 12, color: "var(--ink-3)" }}>探</span>
        <span style={{ fontSize: 12, color: "var(--ink-4)" }}>search knowledge…</span>
      </div>
      <span style={{ fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--font-mono)" }}>{D.org.members} members</span>
      <Avatar name="Keiko" size={28} />
      <button title="Log out" style={{ display: "inline-flex", alignItems: "center", gap: 6,
        background: "transparent", border: "var(--hairline)", borderRadius: 7, padding: "6px 11px",
        cursor: "pointer", color: "var(--ink-2)", fontFamily: "inherit", fontSize: 12 }}>
        <span className="kanji" style={{ fontSize: 12, color: "var(--ink-3)" }}>出</span>Log out
      </button>
    </div>
  );
}

/* ─── role-scoped left nav (all items live) ─────────────── */
function DojoRoleNav({ nav, active, setActive }) {
  return (
    <aside style={{ width: 218, flexShrink: 0, borderRight: "var(--hairline)", background: "var(--paper-2)",
                    display: "flex", flexDirection: "column", padding: "16px 12px", overflow: "auto" }}>
      {(() => {
        const renderGroup = grp => (
        <div key={grp.group} style={{ marginBottom: 14, opacity: grp.manage ? 0.82 : 1 }}>
          <div style={{ fontSize: 9.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-4)",
                        fontWeight: 600, padding: "0 8px", marginBottom: 6 }}>{grp.group}</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {grp.items.map(it => {
              const on = active === it.id;
              return (
                <button key={it.id} onClick={() => setActive(it.id)} style={{
                  display: "grid", gridTemplateColumns: "auto 1fr auto", alignItems: "center", gap: 9,
                  width: "100%", textAlign: "left", borderRadius: 7, padding: "8px 9px",
                  background: on ? "var(--paper)" : "transparent",
                  border: on ? "var(--hairline)" : "1px solid transparent",
                  color: on ? "var(--ink)" : "var(--ink-2)", cursor: "pointer", fontSize: 13,
                }}>
                  <span className="kanji" style={{ fontSize: 13, width: 15, textAlign: "center",
                                color: on ? "var(--accent)" : "var(--ink-3)" }}>{it.kanji}</span>
                  <span>{it.label}</span>
                  {it.badge != null
                    ? <span className="mono" style={{ fontSize: 10, fontWeight: 600, color: "var(--paper)",
                              background: "var(--accent)", borderRadius: 10, padding: "0 6px", lineHeight: "16px" }}>{it.badge}</span>
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
            <div style={{ flex: 1, minHeight: 12 }} />
            {manage.length > 0 && <div style={{ borderTop: "var(--hairline)", margin: "0 8px 12px" }} />}
            {manage.map(renderGroup)}
          </React.Fragment>
        );
      })()}
      <button onClick={() => setActive && setActive("identity")} style={{ display: "grid", gridTemplateColumns: "auto 1fr", alignItems: "center", gap: 9,
        width: "100%", textAlign: "left", padding: "8px 9px", background: "transparent",
        color: "var(--ink-2)", fontSize: 13, cursor: "pointer", borderTop: "var(--hairline)", paddingTop: 12 }}>
        <span className="kanji" style={{ fontSize: 13, width: 15, textAlign: "center", color: "var(--ink-3)" }}>調</span>
        <span>Settings · SSO</span>
      </button>
    </aside>
  );
}

/* ─── mobile shell — condensed bar + bottom tab nav ─────── */
function DojoMobileBar({ role, live = true }) {
  const D = window.DOJO;
  const org = D.memberships.find(m => m.current) || D.memberships[0];
  return (
    <div style={{ flexShrink: 0, display: "flex", alignItems: "center", gap: 10, padding: "12px 16px",
                  borderBottom: "var(--hairline)", background: "var(--paper)" }}>
      <span className="kanji" style={{ fontSize: 20, color: "var(--accent)", lineHeight: 1 }}>結</span>
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ fontSize: 14, color: "var(--ink)", fontWeight: 600, lineHeight: 1.1, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{org.name}</div>
        {role && <div className="mono" style={{ fontSize: 10, color: "var(--ink-4)", marginTop: 1 }}>{role.label}</div>}
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
function DojoTabBar({ tabs, active, onNav }) {
  return (
    <div style={{ flexShrink: 0, display: "grid", gridTemplateColumns: `repeat(${tabs.length}, 1fr)`,
                  borderTop: "var(--hairline)", background: "var(--paper)" }}>
      {tabs.map(it => {
        const on = active === it.id;
        return (
          <button key={it.id} onClick={() => onNav && onNav(it.id)} style={{
            display: "flex", flexDirection: "column", alignItems: "center", gap: 3, padding: "9px 4px 11px",
            background: "transparent", border: "none", cursor: "pointer",
            color: on ? "var(--ink)" : "var(--ink-3)", position: "relative",
          }}>
            <span className="kanji" style={{ fontSize: 17, color: on ? "var(--accent)" : "var(--ink-3)" }}>{it.kanji}</span>
            <span style={{ fontSize: 10, fontWeight: on ? 600 : 400, whiteSpace: "nowrap" }}>{it.label.split(" ")[0]}</span>
            {it.badge != null && (
              <span className="mono" style={{ position: "absolute", top: 5, right: "50%", marginRight: -18,
                fontSize: 9, fontWeight: 600, color: "var(--paper)", background: "var(--accent)",
                borderRadius: 10, padding: "0 5px", lineHeight: "14px" }}>{it.badge}</span>
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
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: align === "baseline" ? "baseline" : "center", gap: 10, padding: "13px 16px", borderBottom: "var(--hairline)" }}>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        {note && <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{note}</span>}
        <span style={{ flex: 1 }} />{right}
      </div>
      <div style={{ padding: 16 }}>{children}</div>
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
function DojoRoleShell({ label, role, nav, active, setActive, children, mobile = false, relay = true, relayStart = null }) {
  const D = window.DOJO;
  const org = D.memberships.find(m => m.current) || D.memberships[0];
  const [relayView, setRelayView] = React.useState(relayStart);
  const fullNav = relay ? [RELAY_GROUP, ...nav] : nav;
  const activeId = relayView ? ("__r_" + relayView) : active;
  const onNav = (id) => {
    if (RELAY_IDS[id]) setRelayView(RELAY_IDS[id]);
    else { setRelayView(null); if (setActive) setActive(id); }
  };
  const main = (relayView && window.RelayArea)
    ? React.createElement(window.RelayArea, { view: relayView, wide: !mobile, onOpen: () => setRelayView("watch") })
    : children;
  if (mobile) {
    return (
      <div className="sensei" data-screen-label={label} style={{ width: "100%", height: "100%",
            display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
        <DojoMobileBar role={role} />
        <div style={{ flex: 1, minHeight: 0, overflow: "auto", display: "flex", flexDirection: "column" }}>{main}</div>
        <DojoMobileTabs nav={fullNav} active={activeId} setActive={onNav} />
      </div>
    );
  }
  return (
    <div className="sensei" data-screen-label={label} style={{ width: "100%", height: "100%",
          display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoTopBar org={org} role={role} />
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <DojoRoleNav nav={fullNav} active={activeId} setActive={onNav} />
        <div style={{ flex: 1, minWidth: 0 }}>{main}</div>
      </div>
    </div>
  );
}

function DojoLive({ label = "live" }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 10.5, color: "var(--success)",
                  background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: 999, padding: "3px 9px" }}>
      <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--success)" }} />{label}
    </span>
  );
}

// Canonical button — one recipe for the five that drifted across screens.
// variant: primary (ink) · ghost (hairline) · danger. size sm|md.
function DojoBtn({ variant = "primary", size = "md", kanji, children, onClick, style }) {
  const cls = size === "sm" ? "text-sm" : "text-base";
  const pad = size === "sm" ? "6px 12px" : "10px 16px";
  const base = { display: "inline-flex", alignItems: "center", justifyContent: "center", gap: 7, borderRadius: "var(--radius)",
    padding: pad, fontWeight: 500, cursor: "pointer", fontFamily: "inherit", border: "none" };
  const skin = variant === "ghost"
    ? { background: "var(--paper)", color: "var(--ink)", border: "var(--hairline)" }
    : variant === "danger"
      ? { background: "var(--danger)", color: "var(--paper)" }
      : { background: "var(--ink)", color: "var(--paper)" };
  const kc = variant === "ghost" ? "var(--accent)" : variant === "danger" ? "var(--paper)" : "var(--accent)";
  return (
    <button onClick={onClick} className={cls} style={{ ...base, ...skin, ...style }}>
      {kanji && <span className="kanji" style={{ color: kc }}>{kanji}</span>}{children}
    </button>
  );
}

/* Per-kind membership tag — kanji + Dōjō name, tinted by kind (the one map). */
const DOJO_KIND_TONE = { Employer: "var(--ink-2)", Client: "var(--accent)", Community: "var(--success)", Personal: "var(--ink-3)" };
const DOJO_KIND_KANJI = { Employer: "社", Client: "客", Community: "群", Personal: "己" };
function DojoKindTag({ p }) {
  const tone = DOJO_KIND_TONE[p.kind] || "var(--ink-3)";
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 10.5, color: tone,
      background: `color-mix(in oklch, ${tone} 12%, transparent)`, border: `1px solid color-mix(in oklch, ${tone} 28%, transparent)`,
      borderRadius: 999, padding: "2px 8px", whiteSpace: "nowrap" }}>
      <span className="kanji" style={{ fontSize: 11 }}>{DOJO_KIND_KANJI[p.kind] || "結"}</span>{p.dojo}
    </span>
  );
}

Object.assign(window, {
  DOJO_ORIGIN, DOJO_TYPE, DOJO_KIND_TONE, DOJO_KIND_KANJI, DojoChip, OriginChip, Confidence, DojoHead, DojoLive, DojoBtn, DojoKindTag,
  DojoTopBar, DojoRoleNav, DojoRoleShell, DojoMobileBar, DojoMobileTabs, DojoTabBar, DojoPanel,
});
