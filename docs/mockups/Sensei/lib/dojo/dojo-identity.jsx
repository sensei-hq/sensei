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
  { git: "write access",     role: "Contributor", tone: "var(--ink-soft)" },
  { git: "read / no access", role: "Read-only",   tone: "var(--ink-faint)" },
];

function IdPanel({ title, note, right, children }) {
  return window.DojoPanel({ title, note, right, children });
}

function DojoIdentity({ mobile = false }) {
  const [idp, setIdp] = idS("okta");
  const [scim, setScim] = idS(true);
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="鍵" eyebrow="Org · manage · identity" title="Identity & access"
        sub="Wire single sign-on so nothing is open access, let SCIM provision and deprovision members automatically, and map git access to Dōjō roles. Roles derive from git, then refine here."
        right={<DojoChip tone="var(--success)" soft="var(--success-soft)">● SSO active · Okta</DojoChip>} />

      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)", display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
        {/* SSO provider */}
        <IdPanel title="Single sign-on" note="OIDC / SAML" right={<DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Test connection</DojoChip>}>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(190px, 1fr))", gap: "var(--space-3)", marginBottom: "var(--space-3)" }}>
            {IDP_PRESETS.map(p => {
              const on = idp === p.id;
              return (
                <button key={p.id} onClick={() => setIdp(p.id)} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", textAlign: "left", cursor: "pointer",
                  background: on ? "var(--paper)" : "transparent", border: on ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-3)", fontFamily: "inherit" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-lg)", color: on ? "var(--accent)" : "var(--ink-mute)" }}>{p.kanji}</span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 500 }}>{p.name}</div>
                    <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{p.proto}</div>
                  </div>
                  {p.connected && <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />}
                </button>
              );
            })}
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: "var(--space-2)" }}>
            {[["Metadata URL", "https://acme.okta.com/app/…/sso/saml/metadata"], ["ACS callback", "dojo.acme.internal/auth/saml/callback"]].map(([l, v]) => (
              <div key={l}>
                <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-1)" }}>{l}</div>
                <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-3)", wordBreak: "break-all" }}>{v}</div>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-3)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>試</span>
            Run a test sign-in before saving — verified logins are required before SSO is enforced.
          </div>
        </IdPanel>

        {/* SCIM */}
        <IdPanel title="SCIM provisioning" note="auto provision · deprovision"
          right={<span onClick={() => setScim(s => !s)} style={{ display: "inline-flex", alignItems: "center", width: 38, height: 20, borderRadius: "var(--radius-lg)", padding: "var(--space-1)", cursor: "pointer",
            background: scim ? "var(--accent)" : "var(--paper-mute)", justifyContent: scim ? "flex-end" : "flex-start" }}>
            <span style={{ width: 16, height: 16, borderRadius: "50%", background: "var(--paper)" }} /></span>}>
          <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55 }}>
            {scim
              ? <span>On — members are created and disabled automatically from your IdP. Last sync <b style={{ fontWeight: 600, color: "var(--ink)" }}>4 min ago</b> · 48 in sync.</span>
              : <span>Off — where SCIM isn't available, members are provisioned just-in-time on first connect at the git-derived role.</span>}
          </div>
        </IdPanel>

        {/* git-role mapping */}
        <IdPanel title="Git access → Dōjō role" note="derive-then-refine">
          <div style={{ display: "flex", flexDirection: "column" }}>
            {ROLE_MAP.map((r, i) => mobile ? (
              <div key={r.git} style={{ padding: "var(--space-3) var(--space-1)", borderBottom: i < ROLE_MAP.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{r.git}</div>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-1)" }}>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-faint)" }}>→</span>
                  <span style={{ fontSize: "var(--text-sm)", color: r.tone, fontWeight: 500 }}>{r.role}</span>
                </div>
              </div>
            ) : (
              <div key={r.git} style={{ display: "grid", gridTemplateColumns: "1fr auto 1fr", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-3) var(--space-1)", borderBottom: i < ROLE_MAP.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{r.git}</span>
                <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-faint)" }}>→</span>
                <span style={{ fontSize: "var(--text-sm)", color: r.tone, fontWeight: 500 }}>{r.role}</span>
              </div>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-3)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>任</span>
            <span>Auto-provisioned members are capped at <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>Read-only</b> — Maintainer and Org admin are only ever elevated by hand in Members &amp; roles.</span>
          </div>
        </IdPanel>
      </div>
    </div>
  );
}

window.DojoIdentity = DojoIdentity;
