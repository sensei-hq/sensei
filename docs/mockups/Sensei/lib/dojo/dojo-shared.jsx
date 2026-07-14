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
  client:    { label: "Dereferenced", tone: "var(--accent)",  soft: "var(--accent-soft)" },
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

/* ─── top bar (org switcher + role) ─────────────────────── */
function DojoTopBar({ org, role }) {
  const D = window.DOJO;
  return (
    <div style={{ height: 54, flexShrink: 0, display: "flex", alignItems: "center", gap: 16,
                  padding: "0 18px", borderBottom: "var(--hairline)", background: "var(--paper)" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 9 }}>
        <span className="kanji" style={{ fontSize: 22, color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: 18, letterSpacing: "-0.01em" }}>Dōjō</span>
      </div>
      <button style={{ display: "inline-flex", alignItems: "center", gap: 8, marginLeft: 6,
        background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 7, padding: "6px 11px", cursor: "pointer" }}>
        <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>{org.kanji}</span>
        <span style={{ fontSize: 13, color: "var(--ink)" }}>{org.name}</span>
        <span className="mono" style={{ fontSize: 9, color: "var(--ink-4)", textTransform: "uppercase", letterSpacing: ".08em" }}>employer</span>
        <span style={{ fontSize: 9, color: "var(--ink-3)" }}>▾</span>
      </button>
      {role && (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, fontFamily: "var(--font-mono)",
          color: "var(--accent)", background: "var(--accent-soft)", border: "1px solid oklch(0.58 0.15 35/.28)",
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
      {nav.map(grp => (
        <div key={grp.group} style={{ marginBottom: 14 }}>
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
      ))}
      <div style={{ flex: 1 }} />
      <button style={{ display: "grid", gridTemplateColumns: "auto 1fr", alignItems: "center", gap: 9,
        width: "100%", textAlign: "left", padding: "8px 9px", background: "transparent",
        color: "var(--ink-3)", fontSize: 13, cursor: "default", borderTop: "var(--hairline)", paddingTop: 12, opacity: 0.6 }}>
        <span className="kanji" style={{ fontSize: 13, width: 15, textAlign: "center", color: "var(--ink-3)" }}>調</span>
        <span>Settings · SSO</span>
      </button>
    </aside>
  );
}

/* ─── role shell — top bar + nav + main ─────────────────── */
function DojoRoleShell({ label, role, nav, active, setActive, children }) {
  const D = window.DOJO;
  const org = D.memberships.find(m => m.current) || D.memberships[0];
  return (
    <div className="sensei" data-screen-label={label} style={{ width: "100%", height: "100%",
          display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoTopBar org={org} role={role} />
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <DojoRoleNav nav={nav} active={active} setActive={setActive} />
        <div style={{ flex: 1, minWidth: 0 }}>{children}</div>
      </div>
    </div>
  );
}

Object.assign(window, {
  DOJO_ORIGIN, DOJO_TYPE, DojoChip, OriginChip, Confidence, DojoHead,
  DojoTopBar, DojoRoleNav, DojoRoleShell,
});
