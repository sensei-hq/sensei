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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="鍵" eyebrow="Org · manage · identity" title="Identity & access"
        sub="Wire single sign-on so nothing is open access, let SCIM provision and deprovision members automatically, and map git access to Dōjō roles. Roles derive from git, then refine here."
        right={<DojoChip tone="var(--success)" soft="var(--success-soft)">● SSO active · Okta</DojoChip>} />

      <div className="flex-1 overflow-auto flex flex-col gap-4" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
        {/* SSO provider */}
        <IdPanel title="Single sign-on" note="OIDC / SAML" right={<DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">Test connection</DojoChip>}>
          <div className="grid gap-3 mb-3" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(190px, 1fr))" }}>
            {IDP_PRESETS.map(p => {
              const on = idp === p.id;
              return (
                <button className="flex items-center gap-2 text-left cursor-pointer rounded-lg py-3 px-3" key={p.id} onClick={() => setIdp(p.id)} style={{
 background: on ? "var(--paper)" : "transparent", border: on ? "1px solid var(--accent)" : "var(--hairline)", fontFamily: "inherit" }}>
                  <span className="kanji text-lg" style={{ color: on ? "var(--accent)" : "var(--ink-mute)" }}>{p.kanji}</span>
                  <div className="flex-1 min-w-0" >
                    <div className="text-sm text-ink font-medium" >{p.name}</div>
                    <div className="mono text-xs text-ink-faint mt-1" >{p.proto}</div>
                  </div>
                  {p.connected && <span className="rounded-full bg-success" style={{ width: 7, height: 7 }} />}
                </button>
              );
            })}
          </div>
          <div className="grid gap-2" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))" }}>
            {[["Metadata URL", "https://acme.okta.com/app/…/sso/saml/metadata"], ["ACS callback", "dojo.acme.internal/auth/saml/callback"]].map(([l, v]) => (
              <div key={l}>
                <div className="text-xs uppercase text-ink-faint font-semibold mb-1" style={{ letterSpacing: ".1em" }}>{l}</div>
                <div className="mono text-xs text-ink-soft bg-paper border border-paper-edge rounded-lg py-2 px-3" style={{ wordBreak: "break-all" }}>{v}</div>
              </div>
            ))}
          </div>
          <div className="flex items-center gap-2 mt-3 text-xs text-ink-mute" >
            <span className="kanji text-sm text-accent" >試</span>
            Run a test sign-in before saving — verified logins are required before SSO is enforced.
          </div>
        </IdPanel>

        {/* SCIM */}
        <IdPanel title="SCIM provisioning" note="auto provision · deprovision"
          right={<span className="inline-flex items-center rounded-lg p-1 cursor-pointer" onClick={() => setScim(s => !s)} style={{ width: 38, height: 20,
 background: scim ? "var(--accent)" : "var(--paper-mute)", justifyContent: scim ? "flex-end" : "flex-start" }}>
            <span className="rounded-full bg-paper" style={{ width: 16, height: 16 }} /></span>}>
          <div className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>
            {scim
              ? <span>On — members are created and disabled automatically from your IdP. Last sync <b className="font-semibold text-ink" >4 min ago</b> · 48 in sync.</span>
              : <span>Off — where SCIM isn't available, members are provisioned just-in-time on first connect at the git-derived role.</span>}
          </div>
        </IdPanel>

        {/* git-role mapping */}
        <IdPanel title="Git access → Dōjō role" note="derive-then-refine">
          <div className="flex flex-col" >
            {ROLE_MAP.map((r, i) => mobile ? (
              <div className="py-3 px-1" key={r.git} style={{ borderBottom: i < ROLE_MAP.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <div className="mono text-xs text-ink-mute" >{r.git}</div>
                <div className="flex items-center gap-2 mt-1" >
                  <span className="text-sm text-ink-faint" >→</span>
                  <span className="text-sm font-medium" style={{ color: r.tone }}>{r.role}</span>
                </div>
              </div>
            ) : (
              <div className="grid gap-3 items-center py-3 px-1" key={r.git} style={{ gridTemplateColumns: "1fr auto 1fr", borderBottom: i < ROLE_MAP.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="mono text-xs text-ink-mute" >{r.git}</span>
                <span className="text-sm text-ink-faint" >→</span>
                <span className="text-sm font-medium" style={{ color: r.tone }}>{r.role}</span>
              </div>
            ))}
          </div>
          <div className="flex items-start gap-2 mt-3 text-xs text-ink-mute" style={{ lineHeight: 1.5 }}>
            <span className="kanji text-sm text-accent" >任</span>
            <span>Auto-provisioned members are capped at <b className="font-semibold text-ink-soft" >Read-only</b> — Maintainer and Org admin are only ever elevated by hand in Members &amp; roles.</span>
          </div>
        </IdPanel>
      </div>
    </div>
  );
}

window.DojoIdentity = DojoIdentity;
