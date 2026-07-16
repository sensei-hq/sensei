// Dōjō · Identity & access (Org admin) — the SSO / SCIM pillar that was
// referenced (Members mentions SCIM) but had no home. Connect an IdP, wire
// SCIM provisioning, and map git access → Dōjō role. Reuses DojoHead/DojoChip.

const { useState: idS } = React;

const IDP_PRESETS = [
  { id: "okta",   kanji: "識", name: "Okta",        proto: "OIDC / SAML", connected: true },
  { id: "entra",  kanji: "識", name: "Microsoft Entra", proto: "OIDC / SAML", connected: false },
  { id: "google", kanji: "識", name: "Google Workspace", proto: "OIDC", connected: false },
];
const ROLE_MAP = [
  { git: "org owner",        role: "Org admin",   tone: "var(--accent)" },
  { git: "repo admin",       role: "Maintainer",  tone: "var(--ink)" },
  { git: "write access",     role: "Contributor", tone: "var(--ink-2)" },
  { git: "read / no access", role: "Read-only",   tone: "var(--ink-4)" },
];

function IdPanel({ title, note, right, children }) {
  return window.DojoPanel({ title, note, right, children });
}

function DojoIdentity() {
  const [idp, setIdp] = idS("okta");
  const [scim, setScim] = idS(true);
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="鍵" eyebrow="Org · manage · identity" title="Identity & access"
        sub="Wire single sign-on so nothing is open access, let SCIM provision and deprovision members automatically, and map git access to Dōjō roles. Roles derive from git, then refine here."
        right={<DojoChip tone="var(--success)" soft="var(--success-soft)">● SSO active · Okta</DojoChip>} />

      <div style={{ flex: 1, overflow: "auto", padding: 28, display: "flex", flexDirection: "column", gap: 18 }}>
        {/* SSO provider */}
        <IdPanel title="Single sign-on" note="OIDC / SAML" right={<DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">Test connection</DojoChip>}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(190px, 1fr))", gap: 12, marginBottom: 14 }}>
            {IDP_PRESETS.map(p => {
              const on = idp === p.id;
              return (
                <button key={p.id} onClick={() => setIdp(p.id)} style={{ display: "flex", alignItems: "center", gap: 10, textAlign: "left", cursor: "pointer",
                  background: on ? "var(--paper)" : "transparent", border: on ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: 10, padding: "12px 14px", fontFamily: "inherit" }}>
                  <span className="kanji" style={{ fontSize: 18, color: on ? "var(--accent)" : "var(--ink-3)" }}>{p.kanji}</span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: 13.5, color: "var(--ink)", fontWeight: 500 }}>{p.name}</div>
                    <div className="mono" style={{ fontSize: 10, color: "var(--ink-4)", marginTop: 1 }}>{p.proto}</div>
                  </div>
                  {p.connected && <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />}
                </button>
              );
            })}
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 10 }}>
            {[["Metadata URL", "https://acme.okta.com/app/…/sso/saml/metadata"], ["ACS callback", "dojo.acme.internal/auth/saml/callback"]].map(([l, v]) => (
              <div key={l}>
                <div style={{ fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600, marginBottom: 5 }}>{l}</div>
                <div className="mono" style={{ fontSize: 11.5, color: "var(--ink-2)", background: "var(--paper)", border: "var(--hairline)", borderRadius: 8, padding: "9px 11px", wordBreak: "break-all" }}>{v}</div>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 12, fontSize: 12, color: "var(--ink-3)" }}>
            <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>試</span>
            Run a test sign-in before saving — verified logins are required before SSO is enforced.
          </div>
        </IdPanel>

        {/* SCIM */}
        <IdPanel title="SCIM provisioning" note="auto provision · deprovision"
          right={<span onClick={() => setScim(s => !s)} style={{ display: "inline-flex", alignItems: "center", width: 38, height: 20, borderRadius: 12, padding: 2, cursor: "pointer",
            background: scim ? "var(--accent)" : "var(--paper-3)", justifyContent: scim ? "flex-end" : "flex-start" }}>
            <span style={{ width: 16, height: 16, borderRadius: "50%", background: "var(--paper)" }} /></span>}>
          <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55 }}>
            {scim
              ? <span>On — members are created and disabled automatically from your IdP. Last sync <b style={{ fontWeight: 600, color: "var(--ink)" }}>4 min ago</b> · 48 in sync.</span>
              : <span>Off — where SCIM isn't available, members are provisioned just-in-time on first connect at the git-derived role.</span>}
          </div>
        </IdPanel>

        {/* git-role mapping */}
        <IdPanel title="Git access → Dōjō role" note="derive-then-refine">
          <div style={{ display: "flex", flexDirection: "column" }}>
            {ROLE_MAP.map((r, i) => (
              <div key={r.git} style={{ display: "grid", gridTemplateColumns: "1fr auto 1fr", gap: 14, alignItems: "center", padding: "11px 4px", borderBottom: i < ROLE_MAP.length - 1 ? "1px solid var(--edge)" : "none" }}>
                <span className="mono" style={{ fontSize: 12, color: "var(--ink-3)" }}>{r.git}</span>
                <span style={{ fontSize: 13, color: "var(--ink-4)" }}>→</span>
                <span style={{ fontSize: 13, color: r.tone, fontWeight: 500 }}>{r.role}</span>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "flex-start", gap: 9, marginTop: 12, fontSize: 12, color: "var(--ink-3)", lineHeight: 1.5 }}>
            <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>任</span>
            <span>Auto-provisioned members are capped at <b style={{ fontWeight: 600, color: "var(--ink-2)" }}>Read-only</b> — Maintainer and Org admin are only ever elevated by hand in Members &amp; roles.</span>
          </div>
        </IdPanel>
      </div>
    </div>
  );
}

window.DojoIdentity = DojoIdentity;
