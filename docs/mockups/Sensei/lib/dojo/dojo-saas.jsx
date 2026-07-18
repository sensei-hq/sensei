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
  display: "flex", alignItems: "center", justifyContent: "center", gap: "var(--space-2)",
  width: "100%", background: "var(--ink)", color: "var(--paper)", border: "none",
  borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", fontSize: "var(--text-sm)", fontWeight: 500,
  cursor: "pointer", fontFamily: "inherit",
};
const saasBtnGhost = {
  display: "flex", alignItems: "center", justifyContent: "center", gap: "var(--space-2)",
  width: "100%", background: "var(--paper)", color: "var(--ink)",
  border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", fontSize: "var(--text-sm)",
  cursor: "pointer", fontFamily: "inherit",
};
const saasField = {
  width: "100%", boxSizing: "border-box", background: "var(--paper)",
  border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-3)",
  fontSize: "var(--text-sm)", fontFamily: "inherit", color: "var(--ink)",
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
function DojoSignIn({ mobile = false }) {
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
    <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "var(--space-2)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{kanji}</span>
        {children}
      </div>
      <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, lineHeight: 1, color: "var(--ink)" }}>{value}</div>
      <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-2)", lineHeight: 1.35 }}>{label}</div>
    </div>
  );

  return (
    <div className="sensei" data-screen-label="SaaS · welcome back / sign-in" style={{
      width: "100%", height: "100%", display: "flex", flexDirection: mobile ? "column" : "row", overflow: mobile ? "auto" : "hidden", background: "var(--paper)",
    }}>
      {/* ── left · welcome back + insight into the Dōjō ── */}
      <div style={{
        width: mobile ? "100%" : "57%", flexShrink: 0, padding: mobile ? "var(--space-5) var(--space-5)" : "var(--space-7) var(--space-7)", display: "flex", flexDirection: "column",
        background: "linear-gradient(160deg, var(--accent-soft) 0%, var(--paper-soft) 60%)",
        borderRight: mobile ? "none" : "var(--hairline)", borderBottom: mobile ? "var(--hairline)" : "none", overflow: "auto",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-2xl)", color: "var(--accent)", lineHeight: 1 }}>結</span>
          <span className="display" style={{ fontSize: "var(--text-xl)", letterSpacing: "-0.01em" }}>Dōjō</span>
          <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-2)" }}>dojo.sensei-hq.com</span>
        </div>

        <div style={{ marginTop: "var(--space-7)" }}>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-mute)", marginBottom: "var(--space-2)" }}>Welcome back</div>
          <h1 className="display" style={{ fontSize: "var(--text-3xl)", fontWeight: 300, letterSpacing: "-0.02em", margin: 0, lineHeight: 1.08 }}>
            Your team kept<br/>learning while<br/>you were away.
          </h1>
          <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.6, margin: "var(--space-4) 0 0", maxWidth: 440 }}>
            A snapshot of <b style={{ fontWeight: 600 }}>Acme Corp's</b> shared mind since your last visit — remembered on this device. Sign in to step back in.
          </p>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 1fr 1fr", gap: "var(--space-3)", marginTop: mobile ? "var(--space-5)" : "var(--space-6)" }}>
          <Insight kanji="共" value={m.contribWeek} label="lessons shared this week"><Spark data={m.contribSpark} /></Insight>
          <Insight kanji="決" value={m.approvedWeek} label="approved & distributed" />
          <Insight kanji="盾" value={m.anonymized} label="anonymized from client work · 0 incidents" />
        </div>

        {/* latest approved teaching — a glimpse of substance, not just numbers */}
        <div style={{ marginTop: "var(--space-3)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)",
                      display: "flex", alignItems: "center", gap: "var(--space-3)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>守</span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-1)" }}>Just published · Company</div>
            <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>Never log refresh tokens, even at debug level</div>
          </div>
          <div style={{ textAlign: "right" }}>
            <div className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 300, color: "var(--success)" }}>+{Math.round(m.adoptionLift * 100)}pp</div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>first-try resolution</div>
          </div>
        </div>

        <div style={{ flex: 1 }} />
        <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-5)", lineHeight: 1.5 }}>
          Private to your org · governed · anonymized before anything leaves a client engagement.
        </div>
      </div>

      {/* ── right · sign-in options ── */}
      <div style={{ flex: 1, minWidth: 0, display: "flex", alignItems: "center", justifyContent: "center", padding: "var(--space-6)" }}>
        <div style={{ width: 364, maxWidth: "100%" }}>
          <h2 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: 1.1 }}>Sign in to continue</h2>
          <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)", lineHeight: 1.55, margin: "var(--space-2) 0 var(--space-5)" }}>
            GitHub brings your organizations and roles automatically. No GitHub? Use a magic link.
          </p>

          {/* primary · GitHub */}
          <button style={saasBtnPrimary}>
            <GhMark size={18} color="var(--paper)" /> Continue with GitHub
          </button>
          <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", textAlign: "center", marginTop: "var(--space-2)" }}>
            Derives your orgs &amp; roles from GitHub — and matches your repos.
          </div>

          {/* divider */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", margin: "var(--space-4) 0" }}>
            <span style={{ flex: 1, height: 1, background: "var(--paper-edge)" }} />
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", letterSpacing: ".1em" }}>OR</span>
            <span style={{ flex: 1, height: 1, background: "var(--paper-edge)" }} />
          </div>

          {/* magic link */}
          <label style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, display: "block", marginBottom: "var(--space-2)" }}>Work email</label>
          <input style={{ ...saasField, marginBottom: "var(--space-2)" }} placeholder="you@company.com" defaultValue="" />
          <button style={saasBtnGhost}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>鍵</span> Email me a magic link
          </button>
          <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", textAlign: "center", marginTop: "var(--space-2)" }}>
            For organizations not on GitHub.
          </div>

          {/* self-hosted */}
          <div style={{ marginTop: "var(--space-5)", paddingTop: "var(--space-4)", borderTop: "1px solid var(--paper-edge)" }}>
            {!selfHost ? (
              <button onClick={() => setSelfHost(true)} style={{
                display: "flex", alignItems: "center", gap: "var(--space-2)", width: "100%", justifyContent: "center",
                background: "none", border: "none", cursor: "pointer", fontFamily: "inherit",
                fontSize: "var(--text-sm)", color: "var(--ink-soft)",
              }}>
                <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>基</span>
                Connecting to a self-hosted Dōjō? <span style={{ color: "var(--accent)" }}>Enter its URL →</span>
              </button>
            ) : (
              <div>
                <label style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, display: "block", marginBottom: "var(--space-2)" }}>Self-hosted Dōjō URL</label>
                <div style={{ display: "flex", gap: "var(--space-2)" }}>
                  <input style={{ ...saasField, flex: 1 }} placeholder="dojo.yourcompany.com" defaultValue="dojo.acme.internal" />
                  <button style={{ ...saasBtnGhost, width: "auto", padding: "var(--space-3) var(--space-4)", whiteSpace: "nowrap" }}>Connect</button>
                </div>
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-2)", lineHeight: 1.5 }}>
                  Same sign-in — your server authenticates you through GitHub (or your email magic link) on its own domain.
                </div>
              </div>
            )}
          </div>

          <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", textAlign: "center", marginTop: "var(--space-5)", lineHeight: 1.5 }}>
            One sign-in for the hosted SaaS and any self-hosted Dōjō.
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── 2 · Your organizations (multi-membership picker) ───── */
function DojoOrgs({ onEnter, onCreate, mobile = false }) {
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
  const kindTone = { Employer: "var(--ink-soft)", Client: "var(--accent)", Community: "var(--success)", Personal: "var(--ink-mute)" };

  return (
    <div className="sensei" data-screen-label="SaaS · your organizations" style={{
      width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)",
    }}>
      {/* top bar */}
      <div style={{ height: 54, flexShrink: 0, display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "0 var(--space-5)", borderBottom: "var(--hairline)", background: "var(--paper)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: "var(--text-lg)", letterSpacing: "-0.01em" }}>Dōjō</span>
        {!mobile && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>dojo.sensei-hq.com</span>}
        <span style={{ flex: 1 }} />
        <Avatar name="Keiko Tanaka" size={28} />
        {!mobile && <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>Keiko Tanaka</span>}
        <button className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "none", border: "none", cursor: "pointer" }}>sign out</button>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-5) 0" : "var(--space-6) 0" }}>
        <div style={{ maxWidth: 820, margin: "0 auto", padding: mobile ? "0 var(--space-4)" : "0 var(--space-6)" }}>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-mute)", marginBottom: "var(--space-2)" }}>Signed in as keiko-t · via GitHub</div>
          <h1 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, letterSpacing: "-0.02em", margin: 0, lineHeight: 1.1 }}>Your organizations</h1>
          <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, margin: "var(--space-2) 0 var(--space-5)", maxWidth: 560 }}>
            Every Dōjō you belong to — the ones you run and the ones you're a member of. Open one to work in it (you can switch anytime from the top bar), or manage the ones you administer.
          </p>

          {/* your Dōjōs — grouped by relationship */}
          {(() => {
            const administers = o => /admin/i.test(o.role);
            const renderCard = o => {
              const isAdmin = administers(o);
              return (
              <div key={o.id} style={{
                display: "grid", gridTemplateColumns: mobile ? "auto 1fr" : "auto 1fr auto", gap: "var(--space-4)", alignItems: "center",
                background: "var(--paper-soft)", border: o.last ? "1px solid var(--accent)" : "var(--hairline)",
                borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)",
              }}>
                <div style={{ width: 46, height: 46, borderRadius: "var(--radius-lg)", background: "var(--paper)", border: "var(--hairline)",
                              display: "flex", alignItems: "center", justifyContent: "center" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>{o.kanji}</span>
                </div>
                <div style={{ minWidth: 0 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
                    <span className="display" style={{ fontSize: "var(--text-lg)" }}>{o.name}</span>
                    <DojoChip tone={kindTone[o.kind]} soft="var(--paper-mute)">{o.kind}</DojoChip>
                    {o.last && <DojoChip tone="var(--accent)" soft="var(--accent-soft)">last opened</DojoChip>}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", marginTop: "var(--space-2)", flexWrap: "wrap" }}>
                    <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>
                      {o.host === "self"
                        ? <span><span style={{ color: "var(--ink-faint)" }}>self-hosted · </span>{o.url}</span>
                        : <span><span style={{ color: "var(--ink-faint)" }}>sensei-hq.com/</span>{o.url}</span>}
                    </span>
                    <DojoChip tone={o.host === "self" ? "var(--ink-soft)" : "var(--ink-mute)"} soft="var(--paper-mute)">
                      {o.host === "self" ? "基 self-hosted" : "雲 SaaS"}
                    </DojoChip>
                  </div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-5)", flexWrap: "wrap",
                  justifyContent: mobile ? "space-between" : "flex-end",
                  gridColumn: mobile ? "1 / -1" : "auto", marginTop: mobile ? "var(--space-1)" : 0,
                  paddingTop: mobile ? "var(--space-3)" : 0, borderTop: mobile ? "1px solid var(--paper-edge)" : "none" }}>
                  <div style={{ textAlign: mobile ? "left" : "right" }}>
                    <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{o.role}</div>
                    <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{o.from}</div>
                  </div>
                  <div style={{ textAlign: "right", minWidth: 64 }}>
                    <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{o.members} {o.members === 1 ? "member" : "members"}</div>
                    {isAdmin && o.pending > 0
                      ? <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", marginTop: "var(--space-1)" }}>{o.pending} to triage</div>
                      : <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>up to date</div>}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                    {isAdmin && (
                      <button style={{ ...saasBtnGhost, width: "auto", padding: "var(--space-2) var(--space-3)", whiteSpace: "nowrap" }}>
                        <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>調</span> Manage
                      </button>
                    )}
                    <button onClick={onEnter} style={{
                      display: "inline-flex", alignItems: "center", gap: "var(--space-2)", background: o.last ? "var(--ink)" : "var(--paper)",
                      color: o.last ? "var(--paper)" : "var(--ink)", border: o.last ? "none" : "var(--hairline)",
                      borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-4)", fontSize: "var(--text-sm)", fontWeight: 500, cursor: "pointer", fontFamily: "inherit",
                    }}>
                      Enter <span style={{ fontSize: "var(--text-xs)" }}>→</span>
                    </button>
                  </div>
                </div>
              </div>
              );
            };
            const admin = orgs.filter(administers);
            const member = orgs.filter(o => !administers(o));
            const group = (kanji, label, list) => list.length ? (
              <div style={{ marginBottom: "var(--space-4)" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{kanji}</span>
                  <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{label}</span>
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{list.length}</span>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>{list.map(renderCard)}</div>
              </div>
            ) : null;
            return <React.Fragment>{group("長", "You administer", admin)}{group("群", "You're a member of", member)}</React.Fragment>;
          })()}

          {/* create or join another */}
          <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 1fr", gap: "var(--space-3)", marginTop: "var(--space-4)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "var(--space-4) var(--space-4)",
                          background: "var(--paper)", border: "1px dashed var(--ink-faint)", borderRadius: "var(--radius-lg)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>開</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>Create a Dōjō</div>
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>Start another shared mind — for a new team, client, or side project.</div>
              </div>
              <button onClick={onCreate} style={{ ...saasBtnGhost, width: "auto", padding: "var(--space-3) var(--space-4)", whiteSpace: "nowrap" }}>Create</button>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "var(--space-4) var(--space-4)",
                          background: "var(--paper)", border: "1px dashed var(--ink-faint)", borderRadius: "var(--radius-lg)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--ink-mute)" }}>迎</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>Join another</div>
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>New GitHub orgs appear automatically. Have an invite code?</div>
              </div>
              <input style={{ ...saasField, width: 130 }} placeholder="invite code" defaultValue="" />
              <button style={{ ...saasBtnGhost, width: "auto", padding: "var(--space-3) var(--space-4)", whiteSpace: "nowrap" }}>Join</button>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-5)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, maxWidth: 640 }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>鍵</span>
            <span>Roles are derived from your GitHub org &amp; repo access, then refined inside each Dōjō. Organizations not on GitHub are namespaced <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>other/&lt;org&gt;</b> and sign in by magic link; self-hosted Dōjōs keep their own URL.</span>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { DojoSignIn, DojoOrgs, DojoOrgsEmpty, DojoCreate });

/* ─── 3 · First time · no Dōjōs yet ──────────────────────── */
// The zero-state of DojoOrgs. A brand-new user belongs to nothing, so the
// page's job is to offer the two ways in: create one for your team, or join
// one that already exists. Sensei's calm voice — the emptiness is the content.
function DojoOrgsEmpty({ onCreate, onJoin, mobile = false }) {
  const [showJoin, setShowJoin] = saasS(false);
  const paths = [
    { id: "create", kanji: "開", title: "Create a Dōjō", lead: "Set one up for your team.",
      body: "You become its first steward. Link a GitHub org and your teammates join automatically as they connect; sensei fills it as everyone works.",
      cta: "Create a Dōjō", primary: true },
    { id: "join", kanji: "迎", title: "Join a Dōjō", lead: "Someone already made one.",
      body: "New GitHub orgs you belong to appear here on their own. For a client or non-GitHub org, enter the invite code you were given.",
      cta: "I have an invite code", primary: false },
  ];
  return (
    <div className="sensei" data-screen-label="SaaS · first entry (create / join)" style={{
      width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)",
    }}>
      {/* top bar — matches DojoOrgs */}
      <div style={{ height: 54, flexShrink: 0, display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "0 var(--space-5)", borderBottom: "var(--hairline)", background: "var(--paper)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: "var(--text-lg)", letterSpacing: "-0.01em" }}>Dōjō</span>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>dojo.sensei-hq.com</span>
        <span style={{ flex: 1 }} />
        <Avatar name="Rin Saito" size={28} />
        <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>Rin Saito</span>
        <button className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "none", border: "none", cursor: "pointer" }}>sign out</button>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-5) 0" : "var(--space-7) 0" }}>
        <div style={{ maxWidth: 760, margin: "0 auto", padding: mobile ? "0 var(--space-4)" : "0 var(--space-6)", textAlign: "center" }}>
          {/* the empty mark */}
          <span className="kanji" style={{ fontSize: "var(--text-4xl)", color: "var(--ink-faint)", lineHeight: 1 }}>空</span>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".2em", textTransform: "uppercase", color: "var(--ink-mute)", margin: "var(--space-5) 0 var(--space-2)" }}>Welcome to Dōjō · signed in as rin-saito</div>
          <h1 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, letterSpacing: "-0.02em", margin: 0, lineHeight: 1.1 }}>
            You don’t belong to a Dōjō yet.
          </h1>
          <p style={{ fontSize: "var(--text-base)", color: "var(--ink-soft)", lineHeight: 1.6, margin: "var(--space-3) auto 0", maxWidth: 500 }}>
            A Dōjō is where a team’s sessions consolidate — one shared mind that remembers what everyone has learned. Start one, or step into one that exists.
          </p>

          {/* two paths — stacked row cards */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)", margin: "var(--space-6) 0 0", textAlign: "left" }}>
            {paths.map(p => (
              <div key={p.id} style={{
                display: "flex", alignItems: "center", gap: "var(--space-4)", flexWrap: "wrap",
                background: "var(--paper-soft)", border: p.primary ? "1px solid var(--accent)" : "var(--hairline)",
                borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-5)",
              }}>
                <span className="kanji" style={{ fontSize: "var(--text-2xl)", color: p.primary ? "var(--accent)" : "var(--ink-mute)", lineHeight: 1, flexShrink: 0 }}>{p.kanji}</span>
                <div style={{ flex: 1, minWidth: 240 }}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: "var(--space-2)", flexWrap: "wrap" }}>
                    <span className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 400, letterSpacing: "-0.01em" }}>{p.title}</span>
                    <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", fontWeight: 500 }}>{p.lead}</span>
                  </div>
                  <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)", lineHeight: 1.55, margin: "var(--space-1) 0 0", maxWidth: 460 }}>{p.body}</p>
                </div>
                {p.id === "create" ? (
                  <button onClick={onCreate} style={{ ...saasBtnPrimary, width: "auto", flexShrink: 0 }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--paper)" }}>開</span> {p.cta}
                  </button>
                ) : !showJoin ? (
                  <button onClick={() => setShowJoin(true)} style={{ ...saasBtnGhost, width: "auto", flexShrink: 0 }}>{p.cta}</button>
                ) : (
                  <div style={{ display: "flex", gap: "var(--space-2)", flexShrink: 0 }}>
                    <input style={{ ...saasField, width: 150 }} placeholder="invite code" defaultValue="" />
                    <button onClick={onJoin} style={{ ...saasBtnGhost, width: "auto", padding: "var(--space-3) var(--space-4)", whiteSpace: "nowrap" }}>Join</button>
                  </div>
                )}
              </div>
            ))}
          </div>

          <div style={{ display: "inline-flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-5)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, maxWidth: 560, textAlign: "left" }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)", flexShrink: 0 }}>基</span>
            <span>Prefer to keep everything in-house? A Dōjō can be self-hosted on your own domain — you’ll choose when you create it. Until then, sensei keeps learning locally on your machine.</span>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ─── 4 · Create a Dōjō ──────────────────────────────────── */
// The setup form the "Create" path opens. Name it, choose hosting (SaaS vs
// self-hosted), and decide who joins (a GitHub org auto-fills members, or start
// solo and invite by hand). The creator becomes the first Org admin.
function DojoCreate({ onBack, onCreate, mobile = false }) {
  const [name, setName] = saasS("Acme Corp");
  const [host, setHost] = saasS("saas");
  const [who, setWho] = saasS("org");
  const [plan, setPlan] = saasS("free");
  const slug = (name || "your-dojo").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  const planMeta = {
    free:    { kanji: "無", price: "Free", priceSub: "forever", line: "Public, open-source, or personal — opt-in public knowledge or a solo Dōjō. Unlimited members, full governance & Relay.", tone: "var(--success)" },
    private: { kanji: "組", price: "Per seat", priceSub: "/ mo", line: "Private, shared with your team. Billed per active contributor — read-only is free.", tone: "var(--accent)" },
  };

  const Choice = ({ active, onClick, kanji, title, sub, meta }) => (
    <button onClick={onClick} style={{
      display: "flex", alignItems: "flex-start", gap: "var(--space-3)", width: "100%", textAlign: "left", cursor: "pointer",
      background: active ? "var(--paper-soft)" : "var(--paper)", fontFamily: "inherit",
      border: active ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)",
    }}>
      <span className="kanji" style={{ fontSize: "var(--text-xl)", color: active ? "var(--accent)" : "var(--ink-mute)", lineHeight: 1.1, flexShrink: 0 }}>{kanji}</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
          <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 500 }}>{title}</span>
          {meta && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{meta}</span>}
        </div>
        <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, marginTop: "var(--space-1)" }}>{sub}</div>
      </div>
      <span style={{ width: 17, height: 17, borderRadius: "50%", flexShrink: 0, marginTop: "var(--space-1)",
        border: active ? "none" : "2px solid var(--paper-edge)", background: active ? "var(--accent)" : "transparent",
        display: "flex", alignItems: "center", justifyContent: "center" }}>
        {active && <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--paper)" }} />}
      </span>
    </button>
  );
  const Label = ({ children }) => (
    <label style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, display: "block", marginBottom: "var(--space-2)" }}>{children}</label>
  );

  return (
    <div className="sensei" data-screen-label="SaaS · create a Dōjō" style={{
      width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)",
    }}>
      {/* top bar */}
      <div style={{ height: 54, flexShrink: 0, display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "0 var(--space-5)", borderBottom: "var(--hairline)", background: "var(--paper)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: "var(--text-lg)", letterSpacing: "-0.01em" }}>Dōjō</span>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>dojo.sensei-hq.com</span>
        <span style={{ flex: 1 }} />
        <button onClick={onBack} className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "none", border: "none", cursor: "pointer" }}>← back</button>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-5) 0" : "var(--space-6) 0" }}>
        <div style={{ maxWidth: 560, margin: "0 auto", padding: mobile ? "0 var(--space-4)" : "0 var(--space-6)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-2xl)", color: "var(--accent)", lineHeight: 1 }}>開</span>
          <h1 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, letterSpacing: "-0.02em", margin: "var(--space-4) 0 0", lineHeight: 1.1 }}>Create a Dōjō</h1>
          <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, margin: "var(--space-2) 0 var(--space-6)" }}>
            One shared mind for your team. You’ll be its first steward — refine roles and scopes once it’s open.
          </p>

          {/* name */}
          <Label>Dōjō name</Label>
          <input value={name} onChange={e => setName(e.target.value)} style={saasField} placeholder="Your team or company" />
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", margin: "var(--space-2) 0 var(--space-5)" }}>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
              {host === "saas"
                ? <span><span style={{ color: "var(--ink-faint)" }}>sensei-hq.com/github/</span>{slug}</span>
                : <span><span style={{ color: "var(--ink-faint)" }}>dojo.</span>{slug}<span style={{ color: "var(--ink-faint)" }}>.internal</span></span>}
            </span>
            <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--success)" }}>空</span>
            <span style={{ fontSize: "var(--text-xs)", color: "var(--success)" }}>available</span>
          </div>

          {/* hosting */}
          <Label>Where it lives</Label>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", marginBottom: "var(--space-5)" }}>
            <Choice active={host === "saas"} onClick={() => setHost("saas")} kanji="雲"
              title="Hosted SaaS" meta="recommended"
              sub="Runs on dojo.sensei-hq.com. Nothing to operate; sign-in and backups handled for you." />
            <Choice active={host === "self"} onClick={() => setHost("self")} kanji="基"
              title="Self-hosted" meta="your domain"
              sub="Deploy the Dōjō service + web inside your own VPC. You keep every byte; connect over your URL." />
          </div>

          {/* who joins */}
          <Label>Who joins</Label>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", marginBottom: "var(--space-5)" }}>
            <Choice active={who === "org"} onClick={() => setWho("org")} kanji="社"
              title="Link a GitHub organization"
              sub="Teammates join automatically as they connect; roles derive from their repo access. You can narrow scopes later." />
            <Choice active={who === "solo"} onClick={() => setWho("solo")} kanji="己"
              title="Start solo, invite by hand"
              sub="Begin with just you and send invite codes when you’re ready. Good for non-GitHub teams and clients." />
          </div>

          {/* visibility & plan */}
          <Label>Visibility &amp; plan</Label>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", marginBottom: "var(--space-4)" }}>
            <Choice active={plan === "free"} onClick={() => setPlan("free")} kanji="無"
              title="Free · public, OSS or personal" meta="free forever"
              sub="Public / open-source knowledge, or a personal solo Dōjō for your own projects across every linked email. Unlimited, full governance & Relay — no charge." />
            <Choice active={plan === "private"} onClick={() => setPlan("private")} kanji="組"
              title="Private · shared with a team" meta="paid · per active contributor"
              sub="Private scopes and knowledge for a company or team. Billed per active contributor; read-only members are free." />
          </div>
          {/* live plan summary */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--paper-soft)", border: "var(--hairline)",
                borderLeft: `3px solid ${planMeta[plan].tone}`, borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-5)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-xl)", color: planMeta[plan].tone }}>{planMeta[plan].kanji}</span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.5 }}>{planMeta[plan].line}</div>
            </div>
            <div style={{ textAlign: "right", flexShrink: 0 }}>
              <div className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 300, lineHeight: 1, color: planMeta[plan].tone }}>{planMeta[plan].price}</div>
              <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{planMeta[plan].priceSub}</div>
            </div>
          </div>

          <button onClick={onCreate} style={saasBtnPrimary}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--paper)" }}>結</span> {plan === "private" ? "Create Dōjō · start free trial" : "Create Dōjō · free"}
          </button>
          <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-4)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)", flexShrink: 0 }}>先</span>
            <span>You’ll open as <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>Org admin</b>. It starts empty — sensei fills it as your team works. <span style={{ fontStyle: "italic" }}>Still listening.</span></span>
          </div>
        </div>
      </div>
    </div>
  );
}
