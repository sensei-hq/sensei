// Dōjō · SaaS entry screens (web).
// The pre-console moments now that Dōjō has a hosted SaaS (dojo.sensei-hq.com)
// alongside optional self-hosted servers:
//   · DojoSignIn — welcome-back / sign-in with insight cards into the org's
//     shared mind. GitHub is the primary login (derives orgs + roles); orgs
//     not on GitHub use a magic link; self-hosted Dōjōs sign in the same way
//     at their own URL.
//   · DojoOrgs — "your organizations": the multi-membership picker. Encodes the
//     stable URL scheme (github/<org> · other/<org> · self-hosted independent
//     URL) and the GitHub-derived role per org.
// Reuses DojoChip (dojo-console.jsx) and Avatar/StatusDot (primitives.jsx).

const { useState: saasS } = React;

const saasBtnPrimary = {
  display: "flex", alignItems: "center", justifyContent: "center", gap: 10,
  width: "100%", background: "var(--ink)", color: "var(--paper)", border: "none",
  borderRadius: 9, padding: "13px 18px", fontSize: 14, fontWeight: 500,
  cursor: "pointer", fontFamily: "inherit",
};
const saasBtnGhost = {
  display: "flex", alignItems: "center", justifyContent: "center", gap: 9,
  width: "100%", background: "var(--paper)", color: "var(--ink)",
  border: "var(--hairline)", borderRadius: 9, padding: "12px 18px", fontSize: 13.5,
  cursor: "pointer", fontFamily: "inherit",
};
const saasField = {
  width: "100%", boxSizing: "border-box", background: "var(--paper)",
  border: "var(--hairline)", borderRadius: 9, padding: "11px 13px",
  fontSize: 13.5, fontFamily: "inherit", color: "var(--ink)",
};

// GitHub mark (monochrome, inherits color).
function GhMark({ size = 18, color = "currentColor" }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill={color} aria-hidden="true">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"/>
    </svg>
  );
}

/* ─── 1 · Welcome back / sign-in ─────────────────────────── */
function DojoSignIn() {
  const D = window.DOJO, m = D.metrics;
  const [selfHost, setSelfHost] = saasS(false);

  const Spark = ({ data, tone = "var(--accent)" }) => {
    const max = Math.max(...data), min = Math.min(...data);
    const pts = data.map((v, i) => {
      const x = (i / (data.length - 1)) * 64;
      const y = 22 - ((v - min) / (max - min || 1)) * 20;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(" ");
    return (
      <svg width="64" height="24" viewBox="0 0 64 24" style={{ overflow: "visible" }}>
        <polyline points={pts} fill="none" stroke={tone} strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  };

  const Insight = ({ kanji, value, label, children }) => (
    <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: 13, padding: "15px 16px" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 9 }}>
        <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>{kanji}</span>
        {children}
      </div>
      <div className="display" style={{ fontSize: 30, fontWeight: 300, lineHeight: 1, color: "var(--ink)" }}>{value}</div>
      <div style={{ fontSize: 11, color: "var(--ink-3)", marginTop: 7, lineHeight: 1.35 }}>{label}</div>
    </div>
  );

  return (
    <div className="sensei" data-screen-label="SaaS · welcome back / sign-in" style={{
      width: "100%", height: "100%", display: "flex", overflow: "hidden", background: "var(--paper)",
    }}>
      {/* ── left · welcome back + insight into the Dōjō ── */}
      <div style={{
        width: "57%", flexShrink: 0, padding: "44px 52px", display: "flex", flexDirection: "column",
        background: "linear-gradient(160deg, var(--accent-soft) 0%, var(--paper-2) 60%)",
        borderRight: "var(--hairline)", overflow: "auto",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span className="kanji" style={{ fontSize: 26, color: "var(--accent)", lineHeight: 1 }}>結</span>
          <span className="display" style={{ fontSize: 21, letterSpacing: "-0.01em" }}>Dōjō</span>
          <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)", background: "var(--paper)", border: "var(--hairline)", borderRadius: 20, padding: "3px 10px" }}>dojo.sensei-hq.com</span>
        </div>

        <div style={{ marginTop: 48 }}>
          <div style={{ fontSize: 12, letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-3)", marginBottom: 10 }}>Welcome back</div>
          <h1 className="display" style={{ fontSize: 42, fontWeight: 300, letterSpacing: "-0.02em", margin: 0, lineHeight: 1.08 }}>
            Your team kept<br/>learning while<br/>you were away.
          </h1>
          <p style={{ fontSize: 14, color: "var(--ink-2)", lineHeight: 1.6, margin: "16px 0 0", maxWidth: 440 }}>
            A snapshot of <b style={{ fontWeight: 600 }}>Acme Corp's</b> shared mind since your last visit — remembered on this device. Sign in to step back in.
          </p>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12, marginTop: 34 }}>
          <Insight kanji="共" value={m.contribWeek} label="lessons shared this week"><Spark data={m.contribSpark} /></Insight>
          <Insight kanji="決" value={m.approvedWeek} label="approved & distributed" />
          <Insight kanji="盾" value={m.dereferenced} label="anonymized from client work · 0 incidents" />
        </div>

        {/* latest approved teaching — a glimpse of substance, not just numbers */}
        <div style={{ marginTop: 14, background: "var(--paper)", border: "var(--hairline)", borderRadius: 13, padding: "15px 17px",
                      display: "flex", alignItems: "center", gap: 14 }}>
          <span className="kanji" style={{ fontSize: 22, color: "var(--accent)" }}>守</span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600, marginBottom: 3 }}>Just published · Company</div>
            <div style={{ fontSize: 14, color: "var(--ink)" }}>Never log refresh tokens, even at debug level</div>
          </div>
          <div style={{ textAlign: "right" }}>
            <div className="display" style={{ fontSize: 22, fontWeight: 300, color: "var(--success)" }}>+{Math.round(m.adoptionLift * 100)}pp</div>
            <div style={{ fontSize: 10, color: "var(--ink-3)" }}>first-try resolution</div>
          </div>
        </div>

        <div style={{ flex: 1 }} />
        <div style={{ fontSize: 11, color: "var(--ink-4)", marginTop: 28, lineHeight: 1.5 }}>
          Private to your org · governed · anonymized before anything leaves a client engagement.
        </div>
      </div>

      {/* ── right · sign-in options ── */}
      <div style={{ flex: 1, minWidth: 0, display: "flex", alignItems: "center", justifyContent: "center", padding: 40 }}>
        <div style={{ width: 364, maxWidth: "100%" }}>
          <h2 className="display" style={{ fontSize: 26, fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: 1.1 }}>Sign in to continue</h2>
          <p style={{ fontSize: 13, color: "var(--ink-3)", lineHeight: 1.55, margin: "8px 0 26px" }}>
            GitHub brings your organizations and roles automatically. No GitHub? Use a magic link.
          </p>

          {/* primary · GitHub */}
          <button style={saasBtnPrimary}>
            <GhMark size={18} color="var(--paper)" /> Continue with GitHub
          </button>
          <div style={{ fontSize: 11, color: "var(--ink-4)", textAlign: "center", marginTop: 7 }}>
            Derives your orgs &amp; roles from GitHub — and matches your repos.
          </div>

          {/* divider */}
          <div style={{ display: "flex", alignItems: "center", gap: 12, margin: "20px 0" }}>
            <span style={{ flex: 1, height: 1, background: "var(--edge)" }} />
            <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", letterSpacing: ".1em" }}>OR</span>
            <span style={{ flex: 1, height: 1, background: "var(--edge)" }} />
          </div>

          {/* magic link */}
          <label style={{ fontSize: 11, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, display: "block", marginBottom: 7 }}>Work email</label>
          <input style={{ ...saasField, marginBottom: 10 }} placeholder="you@company.com" defaultValue="" />
          <button style={saasBtnGhost}>
            <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>鍵</span> Email me a magic link
          </button>
          <div style={{ fontSize: 11, color: "var(--ink-4)", textAlign: "center", marginTop: 7 }}>
            For organizations not on GitHub.
          </div>

          {/* self-hosted */}
          <div style={{ marginTop: 22, paddingTop: 18, borderTop: "1px solid var(--edge)" }}>
            {!selfHost ? (
              <button onClick={() => setSelfHost(true)} style={{
                display: "flex", alignItems: "center", gap: 9, width: "100%", justifyContent: "center",
                background: "none", border: "none", cursor: "pointer", fontFamily: "inherit",
                fontSize: 12.5, color: "var(--ink-2)",
              }}>
                <span className="kanji" style={{ fontSize: 13, color: "var(--ink-3)" }}>基</span>
                Connecting to a self-hosted Dōjō? <span style={{ color: "var(--accent)" }}>Enter its URL →</span>
              </button>
            ) : (
              <div>
                <label style={{ fontSize: 11, letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, display: "block", marginBottom: 7 }}>Self-hosted Dōjō URL</label>
                <div style={{ display: "flex", gap: 8 }}>
                  <input style={{ ...saasField, flex: 1 }} placeholder="dojo.yourcompany.com" defaultValue="dojo.acme.internal" />
                  <button style={{ ...saasBtnGhost, width: "auto", padding: "12px 16px", whiteSpace: "nowrap" }}>Connect</button>
                </div>
                <div style={{ fontSize: 11, color: "var(--ink-4)", marginTop: 8, lineHeight: 1.5 }}>
                  Same sign-in — your server authenticates you through GitHub (or your email magic link) on its own domain.
                </div>
              </div>
            )}
          </div>

          <div style={{ fontSize: 11, color: "var(--ink-4)", textAlign: "center", marginTop: 24, lineHeight: 1.5 }}>
            One sign-in for the hosted SaaS and any self-hosted Dōjō.
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── 2 · Your organizations (multi-membership picker) ───── */
function DojoOrgs({ onEnter }) {
  // The membership model, with each org's stable URL + GitHub-derived role.
  // Acme self-hosts (independent URL); the rest are on the SaaS, namespaced
  // github/<org> for GitHub orgs and other/<org> for non-GitHub orgs.
  const orgs = [
    { id: "acme",   kanji: "社", name: "Acme Corp",  kind: "Employer",  host: "self", url: "dojo.acme.internal",
      role: "Org admin", from: "GitHub · org owner", members: 48, pending: 7, last: true },
    { id: "globex", kanji: "客", name: "Globex",     kind: "Client",    host: "saas", url: "github/globex",
      role: "Maintainer", from: "GitHub · repo admin", members: 12, pending: 2 },
    { id: "initech",kanji: "客", name: "Initech",    kind: "Client",    host: "saas", url: "other/initech",
      role: "Contributor", from: "Magic link · invited", members: 6, pending: 0 },
    { id: "rustco", kanji: "群", name: "Rust Guild", kind: "Community", host: "saas", url: "github/rust-guild",
      role: "Read-only", from: "GitHub · member", members: 410, pending: 0 },
    { id: "self",   kanji: "己", name: "Personal",   kind: "Personal",  host: "saas", url: "github/keiko-t",
      role: "Owner", from: "GitHub · you", members: 1, pending: 0 },
  ];
  const kindTone = { Employer: "var(--ink-2)", Client: "var(--accent)", Community: "var(--success)", Personal: "var(--ink-3)" };

  return (
    <div className="sensei" data-screen-label="SaaS · your organizations" style={{
      width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)",
    }}>
      {/* top bar */}
      <div style={{ height: 54, flexShrink: 0, display: "flex", alignItems: "center", gap: 12, padding: "0 22px", borderBottom: "var(--hairline)", background: "var(--paper)" }}>
        <span className="kanji" style={{ fontSize: 22, color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: 18, letterSpacing: "-0.01em" }}>Dōjō</span>
        <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>dojo.sensei-hq.com</span>
        <span style={{ flex: 1 }} />
        <Avatar name="Keiko Tanaka" size={28} />
        <span style={{ fontSize: 13, color: "var(--ink-2)" }}>Keiko Tanaka</span>
        <button className="mono" style={{ fontSize: 11, color: "var(--ink-3)", background: "none", border: "none", cursor: "pointer" }}>sign out</button>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: "36px 0" }}>
        <div style={{ maxWidth: 820, margin: "0 auto", padding: "0 32px" }}>
          <div style={{ fontSize: 11, letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-3)", marginBottom: 8 }}>Signed in as keiko-t · via GitHub</div>
          <h1 className="display" style={{ fontSize: 32, fontWeight: 300, letterSpacing: "-0.02em", margin: 0, lineHeight: 1.1 }}>Your organizations</h1>
          <p style={{ fontSize: 14, color: "var(--ink-2)", lineHeight: 1.55, margin: "8px 0 26px", maxWidth: 560 }}>
            You belong to {orgs.length} Dōjōs. Roles come from your GitHub access; pick one to open its console.
          </p>

          {/* org cards */}
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {orgs.map(o => (
              <div key={o.id} style={{
                display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 18, alignItems: "center",
                background: "var(--paper-2)", border: o.last ? "1px solid var(--accent)" : "var(--hairline)",
                borderRadius: 14, padding: "16px 20px",
              }}>
                <div style={{ width: 46, height: 46, borderRadius: 11, background: "var(--paper)", border: "var(--hairline)",
                              display: "flex", alignItems: "center", justifyContent: "center" }}>
                  <span className="kanji" style={{ fontSize: 24, color: "var(--accent)" }}>{o.kanji}</span>
                </div>
                <div style={{ minWidth: 0 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 9, flexWrap: "wrap" }}>
                    <span className="display" style={{ fontSize: 18 }}>{o.name}</span>
                    <DojoChip tone={kindTone[o.kind]} soft="var(--paper-3)">{o.kind}</DojoChip>
                    {o.last && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">last opened</DojoChip>}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 12, marginTop: 7, flexWrap: "wrap" }}>
                    <span className="mono" style={{ fontSize: 11.5, color: "var(--ink-2)" }}>
                      {o.host === "self"
                        ? <span><span style={{ color: "var(--ink-4)" }}>self-hosted · </span>{o.url}</span>
                        : <span><span style={{ color: "var(--ink-4)" }}>sensei-hq.com/</span>{o.url}</span>}
                    </span>
                    <DojoChip tone={o.host === "self" ? "var(--ink-2)" : "var(--ink-3)"} soft="var(--paper-3)">
                      {o.host === "self" ? "基 self-hosted" : "雲 SaaS"}
                    </DojoChip>
                  </div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 22 }}>
                  <div style={{ textAlign: "right" }}>
                    <div style={{ fontSize: 13, color: "var(--ink)" }}>{o.role}</div>
                    <div className="mono" style={{ fontSize: 10, color: "var(--ink-4)", marginTop: 2 }}>{o.from}</div>
                  </div>
                  <div style={{ textAlign: "right", minWidth: 64 }}>
                    <div className="mono" style={{ fontSize: 11.5, color: "var(--ink-2)" }}>{o.members} {o.members === 1 ? "member" : "members"}</div>
                    {o.pending > 0
                      ? <div className="mono" style={{ fontSize: 10.5, color: "var(--accent)", marginTop: 3 }}>{o.pending} to triage</div>
                      : <div className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 3 }}>up to date</div>}
                  </div>
                  <button onClick={onEnter} style={{
                    display: "inline-flex", alignItems: "center", gap: 7, background: o.last ? "var(--ink)" : "var(--paper)",
                    color: o.last ? "var(--paper)" : "var(--ink)", border: o.last ? "none" : "var(--hairline)",
                    borderRadius: 8, padding: "10px 18px", fontSize: 13, fontWeight: 500, cursor: "pointer", fontFamily: "inherit",
                  }}>
                    Enter <span style={{ fontSize: 12 }}>→</span>
                  </button>
                </div>
              </div>
            ))}
          </div>

          {/* join another */}
          <div style={{ display: "flex", alignItems: "center", gap: 14, marginTop: 18, padding: "16px 20px",
                        background: "var(--paper)", border: "1px dashed var(--ink-4)", borderRadius: 14 }}>
            <span className="kanji" style={{ fontSize: 20, color: "var(--ink-3)" }}>迎</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: 13.5, color: "var(--ink)" }}>Join another organization</div>
              <div style={{ fontSize: 11.5, color: "var(--ink-3)", marginTop: 2 }}>New GitHub orgs you join appear here automatically. Have an invite code for a client or non-GitHub org?</div>
            </div>
            <input style={{ ...saasField, width: 170 }} placeholder="invite code" defaultValue="" />
            <button style={{ ...saasBtnGhost, width: "auto", padding: "11px 18px", whiteSpace: "nowrap" }}>Join</button>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: 9, marginTop: 22, fontSize: 12, color: "var(--ink-3)", lineHeight: 1.5, maxWidth: 640 }}>
            <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>鍵</span>
            <span>Roles are derived from your GitHub org &amp; repo access, then refined inside each Dōjō. Organizations not on GitHub are namespaced <b style={{ fontWeight: 600, color: "var(--ink-2)" }}>other/&lt;org&gt;</b> and sign in by magic link; self-hosted Dōjōs keep their own URL.</span>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { DojoSignIn, DojoOrgs });
