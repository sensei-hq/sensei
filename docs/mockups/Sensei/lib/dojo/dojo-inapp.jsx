// Dōjō · in-app integration touchpoints (mocks).
// These are Observatory (desktop) screens — the moments where the app meets
// the company Dōjō: join, connect, bind, share, the global↔company toggle,
// and the downstream lane. Standalone mocks for review; wired later.
// Reuses globals from primitives.jsx (TauriChrome, Kanji, Avatar, StatusDot)
// and dojo-console.jsx (DojoChip, OriginChip).

const { useState: iaS } = React;

function InappFrame({ label, title, children, embedded }) {
  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label={label} >
      {!embedded && <TauriChrome title={title} />}
      <div className="flex-1 min-h-0 overflow-auto flex flex-col" >
        {children}
      </div>
    </div>
  );
}

function IaHead({ kanji, eyebrow, title, sub, right, mobile = false }) {
  return (
    <div className="flex items-start border-b shrink-0" style={{ gap: mobile ? "var(--space-3)" : "var(--space-4)", flexWrap: mobile ? "wrap" : "nowrap",
 padding: mobile ? "var(--space-4) var(--space-4) var(--space-3)" : "var(--space-6) var(--space-8) var(--space-4)" }}>
      <span className="kanji text-accent shrink-0" style={{ fontSize: mobile ? "var(--text-2xl)" : "var(--text-3xl)", lineHeight: 1 }}>{kanji}</span>
      <div className="flex-1" style={{ minWidth: mobile ? 180 : 0 }}>
        <div className="text-xs uppercase text-ink-mute mb-1" style={{ letterSpacing: ".18em" }}>{eyebrow}</div>
        <h1 className="display text-xl font-normal m-0" style={{ letterSpacing: "-0.015em", lineHeight: 1.05 }}>{title}</h1>
        {sub && <p className="text-sm text-ink-soft" style={{ lineHeight: 1.55, margin: "var(--space-1) 0 0", maxWidth: 700 }}>{sub}</p>}
      </div>
      {right && <div className="shrink-0" style={{ width: mobile ? "100%" : "auto" }}>{right}</div>}
    </div>
  );
}

const btnPrimary = { background: "var(--ink)", color: "var(--paper)", border: "none", borderRadius: "var(--radius-lg)",
  padding: "var(--space-2) var(--space-4)", fontSize: "var(--text-sm)", fontWeight: 500, cursor: "pointer", fontFamily: "inherit",
  display: "inline-flex", alignItems: "center", gap: "var(--space-2)" };
const btnGhost = { background: "var(--paper-mute)", color: "var(--ink-soft)", border: "none", borderRadius: "var(--radius-lg)",
  padding: "var(--space-2) var(--space-4)", fontSize: "var(--text-sm)", cursor: "pointer", fontFamily: "inherit" };

/* ─── 1 · Bootstrap — first-run home (Dōjō optional) ─────── */
// NOT a gate. On first run the desktop app lands you on your OWN work — the
// projects sensei is already watching locally and whatever is running now — so
// you can work immediately, solo, without joining anything. A detected Dōjō is
// offered as a demoted, dismissible card on the side; joining stays optional
// and always available later from Today / Preferences.
function InappJoin({ onContinue }) {
  const [dismissed, setDismissed] = iaS(false);
  const soloFlag = {
    doing:   { tone: "var(--success)", label: "running",         note: "task running · 4m" },
    approve: { tone: "var(--accent)",  label: "approval waiting", note: "needs your ok" },
    stall:   { tone: "var(--warning)", label: "stalled",          note: "quiet 21m" },
  };
  const projects = [
    { k: "測", name: "telemetry-ingest", now: "ingest schema draft",       phase: "1/3", pct: 22,  flag: "doing"   },
    { k: "記", name: "field-notes",      now: "waiting: rename migration", phase: "2/3", pct: 55,  flag: "approve" },
    { k: "庫", name: "homelab-scripts",  now: "quiet since 21m",           phase: "1/2", pct: 30,  flag: "stall"   },
    { k: "頁", name: "personal-site",    now: "phase complete",            phase: "3/3", pct: 100, flag: null      },
  ];
  const running = projects.filter(p => p.flag === "doing").length;

  return (
    <InappFrame label="First run · your work (Dōjō optional)" title="Sensei  先生  ·  first run">
      <IaHead kanji="場" eyebrow="First run · working locally" title="Here's what's running."
        sub={<span>Sensei is already watching your projects on this machine — you don't need a Dōjō to work. <b className="font-semibold text-ink" >{running} task{running === 1 ? "" : "s"}</b> running now. Joining a Dōjō is optional; do it when you want to share.</span>}
        right={<div className="flex items-center gap-2" >
          <span className="inline-flex items-center gap-1 text-xs text-ink-mute bg-paper-soft border border-paper-edge rounded-full py-1 px-3 whitespace-nowrap" >
            <span className="kanji text-ink-soft" >己</span> solo</span>
          <button className="whitespace-nowrap" onClick={onContinue} style={{ ...btnPrimary }}>Open my workspace →</button>
        </div>} />

      <div className="flex-1 overflow-auto p-8 grid gap-6 items-start" style={{
 gridTemplateColumns: dismissed ? "1fr" : "minmax(0,1fr) 320px" }}>
        {/* your work — the actual landing */}
        <div className="min-w-0" >
          <div className="flex items-center gap-2 mb-3" >
            <span className="kanji text-sm text-ink-mute" >場</span>
            <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Your projects</span>
            <span className="mono text-xs text-ink-faint" >{projects.length}</span>
            <span className="flex-1" />
            <span className="mono text-xs text-ink-faint" >local · this machine</span>
          </div>
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
            {projects.map((p, i) => {
              const f = p.flag ? soloFlag[p.flag] : null;
              return (
                <div className="grid gap-3 items-center py-3 px-4" key={p.name} style={{ gridTemplateColumns: "auto 1fr 150px auto", borderBottom: i < projects.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <span className="kanji text-lg text-center" style={{ color: f ? f.tone : "var(--ink-mute)", lineHeight: 1, width: 22 }}>{p.k}</span>
                  <div className="min-w-0" >
                    <div className="flex items-center gap-2" >
                      <span className="mono text-sm text-ink" >{p.name}</span>
                      {p.flag === "doing" && <span className="rounded-full bg-success shrink-0" style={{ width: 6, height: 6 }} />}
                    </div>
                    <div className="text-xs text-ink-mute mt-1" >{p.now}</div>
                  </div>
                  <div>
                    <div className="rounded-sm bg-paper-mute overflow-hidden" style={{ height: 6 }}>
                      <div className="h-full" style={{ width: p.pct + "%", background: f ? f.tone : "var(--ink-mute)" }} />
                    </div>
                    <div className="mono text-xs text-ink-faint mt-1" >phase {p.phase} · {f ? f.note : "up to date"}</div>
                  </div>
                  {f ? <span className="text-xs font-semibold whitespace-nowrap" style={{ color: f.tone }}>{f.label}</span>
                     : <span className="mono text-xs text-ink-faint" >done</span>}
                </div>
              );
            })}
          </div>
          <div className="flex items-start gap-2 mt-3 text-xs text-ink-mute" style={{ lineHeight: 1.5, maxWidth: 620 }}>
            <span className="kanji text-sm text-accent shrink-0" >基</span>
            <span>Everything here stays on your machine until you choose to share it. <span className="italic" >Still listening.</span></span>
          </div>
        </div>

        {/* detected Dōjō — a demoted, dismissible OFFER, never a wall */}
        {!dismissed && (
          <aside className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4" >
            <div className="flex items-start gap-2" >
              <span className="kanji text-xl text-accent" style={{ lineHeight: 1 }}>結</span>
              <div className="flex-1 min-w-0" >
                <div className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>A Dōjō was detected · optional</div>
                <div className="text-sm text-ink mt-1" style={{ lineHeight: 1.4 }}>Join to inherit your team's standards on day one — and share what you learn back.</div>
              </div>
              <button className="border-0 cursor-pointer text-ink-faint text-base p-0 shrink-0" onClick={() => setDismissed(true)} title="Dismiss — stays in Preferences" style={{ background: "none", lineHeight: 1 }}>✕</button>
            </div>
            <div className="flex flex-col gap-2" style={{ margin: "var(--space-3) 0 var(--space-2)" }}>
              {[["社", "Acme Corp", "via SSO domain", "keiko@acme.com", true], ["客", "Globex", "via invite link", "engagement", false]].map(([k, n, sig, detail, primary]) => (
                <div className="flex items-center gap-2 py-2 px-3 rounded-lg" key={n} style={{
 background: primary ? "var(--accent-soft)" : "var(--paper)", border: primary ? "1px solid var(--accent-edge)" : "var(--hairline)" }}>
                  <span className="kanji text-base text-accent text-center" style={{ width: 20 }}>{k}</span>
                  <div className="flex-1 min-w-0" >
                    <div className="text-sm text-ink" >{n}</div>
                    <div className="mono text-xs text-ink-mute" >{sig} · {detail}</div>
                  </div>
                  <button onClick={onContinue} style={primary ? { ...btnPrimary, padding: "var(--space-1) var(--space-3)" } : { ...btnGhost, padding: "var(--space-1) var(--space-3)" }}>Join</button>
                </div>
              ))}
            </div>
            <div className="text-xs text-ink-faint" style={{ lineHeight: 1.5 }}>
              Authenticated by SSO · nothing is shared until you choose to. Highest-confidence signal first.
            </div>
          </aside>
        )}
      </div>
    </InappFrame>
  );
}

/* ─── 2 · Preferences — Connection pane ──────────────────── */
function InappConnection({ embedded }) {
  const memberships = [
    { k: "社", name: "Acme Corp", kind: "employer", scopes: "all your repos", on: true },
    { k: "客", name: "Globex", kind: "client", scopes: "lumen-auth · billing", on: true },
    { k: "客", name: "Initech", kind: "client", scopes: "initech-portal", on: true },
    { k: "群", name: "Rust Guild", kind: "community", scopes: "rust · tokio", on: false },
    { k: "己", name: "Personal", kind: "personal", scopes: "side projects", on: true },
  ];
  return (
    <InappFrame label="Preferences · Connection" title="Sensei  先生  ·  preferences" embedded={embedded}>
      <IaHead kanji="鍵" eyebrow="Preferences · connection" title="Connections"
        sub="Pair with a company-hosted Dōjō, authenticate, and choose which scopes you follow. You can belong to several at once."
        right={<button style={btnGhost}>+ Add connection</button>} />
      <div className="flex-1 overflow-auto p-8" >
        {/* connected server — Resolution (Authenticate): SSO = access, git = attribution */}
        <div className="bg-paper-soft rounded-lg py-4 px-4 mb-6" style={{ border: "1px solid var(--success-edge)" }}>
          <div className="flex items-center gap-3" >
            <span className="kanji text-xl text-accent" >結</span>
            <div className="flex-1" >
              <div className="text-sm text-ink" >dojo.acme.internal</div>
              <div className="mono text-xs text-ink-mute mt-1" >session refreshes silently · device-code for the CLI</div>
            </div>
            <span className="inline-flex items-center gap-1 text-xs text-success" >
              <StatusDot tone="success" /> connected
            </span>
          </div>
          <div className="grid gap-2 mt-3 pt-3" style={{ gridTemplateColumns: "1fr 1fr", borderTop: "1px solid var(--paper-edge)" }}>
            {[["鍵", "Identity · access", "Work SSO", "keiko@acme.com"], ["署", "Attribution only", "Linked git", "github.com/keiko-t"]].map(([k, role, kind, who]) => (
              <div className="flex items-center gap-2" key={role} >
                <span className="kanji text-base text-accent text-center" style={{ width: 20 }}>{k}</span>
                <div className="min-w-0" >
                  <div className="text-xs uppercase text-ink-faint font-semibold" style={{ letterSpacing: ".1em" }}>{role}</div>
                  <div className="text-sm text-ink mt-1" >{kind} <span className="mono text-xs text-ink-mute" >· {who}</span></div>
                </div>
              </div>
            ))}
          </div>
          <div className="flex items-center gap-2 mt-3 text-xs text-ink-mute" style={{ lineHeight: 1.5 }}>
            <span>SSO grants access; git only signs your attribution — it never grants access.</span>
            <span className="flex-1" />
            <button className="mono text-xs text-accent border-0 cursor-pointer whitespace-nowrap" style={{ background: "none" }}>Air-gapped? Use an offline token →</button>
          </div>
        </div>

        <div className="text-xs uppercase text-ink-mute font-semibold mb-2" style={{ letterSpacing: ".14em" }}>Your memberships</div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {memberships.map((m, i) => (
            <div className="grid gap-3 items-center py-3 px-4" key={m.name} style={{ gridTemplateColumns: "auto 1fr auto auto", borderBottom: i < memberships.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{m.k}</span>
              <div>
                <div className="text-sm text-ink flex items-center gap-2" >
                  {m.name}<DojoChip>{m.kind}</DojoChip>
                </div>
                <div className="mono text-xs text-ink-mute mt-1" >following · {m.scopes}</div>
              </div>
              <button className="mono text-xs text-accent border-0 cursor-pointer" style={{ background: "none" }}>scopes ▾</button>
              <span className="rounded-full relative inline-block" style={{ width: 36, height: 20, background: m.on ? "var(--ink)" : "var(--paper-mute)", transition: "background .15s" }}>
                <span className="absolute rounded-full bg-paper" style={{ top: 2, left: m.on ? 18 : 2, width: 16, height: 16, transition: "left .15s" }} />
              </span>
            </div>
          ))}
        </div>
        <div className="flex items-start gap-2 mt-3 text-xs text-ink-mute" style={{ lineHeight: 1.5, maxWidth: 720 }}>
          <span className="kanji text-sm text-accent" >客</span>
          <span>When a project belongs to a client engagement, that client connection takes precedence over your employer for what's shared and how it's attributed.</span>
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 3 · Project · About — bind to org ──────────────────── */
function InappBind() {
  return (
    <InappFrame label="Project · About — bind to org" title="Sensei  先生  ·  lumen-auth">
      <IaHead kanji="識" eyebrow="lumen-auth · about" title="Membership &amp; routing"
        sub="Bind this project to a connection so its findings route to the right Dōjō — and never pollute another. A project can span more than one client." />
      <div className="flex-1 overflow-auto p-8" style={{ maxWidth: 860 }}>
        {/* Resolution (Bind): explicit-confirm, multiple bindings, re-bind forward-only */}
        <div className="flex items-baseline mb-1" >
          <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Bindings</span>
          <span className="flex-1" />
          <button className="py-1 px-3 text-xs" style={{ ...btnGhost }}>+ Add binding</button>
        </div>
        <div className="text-xs text-ink-mute mb-3" style={{ lineHeight: 1.5 }}>A default is inferred from each git remote and <b className="font-semibold text-ink-soft" >confirmed at first scan</b> — never silently. Route different paths to different connections.</div>
        <div className="flex flex-col gap-2" >
          {/* confirmed binding */}
          <div className="flex items-center gap-3 bg-paper-soft rounded-lg py-3 px-4" style={{ border: "1px solid var(--accent)" }}>
            <span className="kanji text-xl text-accent" >客</span>
            <div className="flex-1 min-w-0" >
              <div className="text-sm text-ink" >Client · Acme</div>
              <div className="mono text-xs text-ink-mute mt-1" >repo root · github.com/acme/lumen-auth</div>
            </div>
            <DojoChip tone="var(--success)" soft="var(--success-soft)">✓ confirmed</DojoChip>
            <button className="mono text-xs text-ink-mute border-0 cursor-pointer" style={{ background: "none" }}>change ▾</button>
          </div>
          {/* inferred binding awaiting confirm */}
          <div className="flex items-center gap-3 bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" >
            <span className="kanji text-xl text-ink-mute" >客</span>
            <div className="flex-1 min-w-0" >
              <div className="text-sm text-ink" >Client · Globex</div>
              <div className="mono text-xs text-ink-mute mt-1" >path · /integrations/globex-sdk</div>
            </div>
            <DojoChip tone="var(--warning)" soft="var(--warning-soft)">inferred</DojoChip>
            <button className="py-2 px-3 text-xs" style={{ ...btnPrimary }}>Confirm</button>
          </div>
        </div>

        <div className="flex items-center gap-2 my-4 mx-0 py-3 px-4 bg-accent-soft rounded-lg" style={{ border: "1px solid var(--accent-edge)" }}>
          <span className="kanji text-base text-accent" >盾</span>
          <div className="text-sm text-ink" style={{ lineHeight: 1.55 }}>
            Findings route by the path they came from. Anything shared upstream from a client binding is <b className="font-semibold" >anonymized</b> — the lesson travels, the source is dropped.
          </div>
        </div>

        <div className="flex items-start gap-2 text-xs text-ink-mute" style={{ lineHeight: 1.5 }}>
          <span className="kanji text-sm text-ink-mute" >時</span>
          <span>Re-binding routes <b className="font-semibold text-ink-soft" >future findings only</b> — past shares stay where they went, and history is never re-routed.</span>
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 4 · Project — ready to share lane ──────────────────── */
function InappShare({ embedded, mobile = false }) {
  const items = [
    { k: "守", title: "Validate webhook signatures before parsing", type: "Guard", origin: "client", scope: "Client · Acme", attrib: "anonymized", on: true, conf: 0.91 },
    { k: "問", title: "Integration-test persona for auth flows", type: "Prompt", origin: "employer", scope: "Stack · React", attrib: "Aiko N.", on: true, conf: 0.84 },
    { k: "紋", title: "Refresh-token rotation on device re-pair", type: "Pattern", origin: "client", scope: "Client · Acme", attrib: "anonymized", on: false, conf: 0.78 },
  ];
  const forming = { k: "芽", title: "Local fix for the staging seed script", reason: "too project-specific · hasn't generalised", conf: 0.41 };
  const [on, setOn] = iaS(() => items.map(i => i.on));
  const shareCount = on.filter(Boolean).length;
  return (
    <InappFrame label="Project · ready to share" title="Sensei  先生  ·  lumen-auth · memories" embedded={embedded}>
      <IaHead mobile={mobile} kanji="共" eyebrow="lumen-auth · memories" title="Ready to share"
        sub="Lessons generalised cleanly enough to leave this project. Pick a scope; attribution and anonymization are applied automatically by origin."
        right={<button style={btnPrimary}><span className="kanji text-sm text-accent" >共</span> Share {shareCount} to Dōjō</button>} />
      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-8)" }}>
        {/* policy bar — Resolution (Share): org policy is the floor */}
        <div className="flex items-center gap-2 flex-wrap bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 mb-1" >
          <span className="kanji text-sm text-ink-mute" >規</span>
          <span className="text-xs text-ink-soft" >Org floor</span>
          <DojoChip tone="var(--success)" soft="var(--success-soft)">always share · test personas</DojoChip>
          <DojoChip tone="var(--warning)" soft="var(--warning-soft)">never share · infra notes</DojoChip>
          <span className="flex-1" />
          <button className="mono text-xs text-accent border-0 cursor-pointer" style={{ background: "none" }}>edit policy</button>
        </div>
        <div className="text-xs text-ink-mute mb-4" style={{ lineHeight: 1.5 }}>Your org's policy is the floor — you can be stricter per item, never looser.</div>

        {/* Resolution (A finding forms): only past the bar surfaces here */}
        <div className="text-xs text-ink-mute mb-2" style={{ lineHeight: 1.5 }}>
          <span className="kanji text-accent mr-1" >芽</span>
          Only lessons past the <b className="font-semibold text-ink-soft" >generalize + confidence bar</b> surface here — nothing is shareable by default.
        </div>

        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {items.map((it, i) => (
            <div className="grid gap-3 items-center cursor-pointer py-3 px-4" key={i} onClick={() => setOn(a => a.map((v, k) => k === i ? !v : v))} style={{ gridTemplateColumns: mobile ? "auto auto 1fr" : "auto auto 1fr auto auto", borderBottom: i < items.length - 1 ? "1px solid var(--paper-edge)" : "none", opacity: on[i] ? 1 : 0.55 }}>
              <span className="rounded-sm text-paper text-xs text-center" role="checkbox" aria-checked={on[i]} style={{ width: 18, height: 18, border: "1.5px solid " + (on[i] ? "var(--accent)" : "var(--ink-faint)"),
 background: on[i] ? "var(--accent)" : "transparent", lineHeight: "15px" }}>{on[i] ? "✓" : ""}</span>
              <span className="kanji text-lg text-accent text-center" style={{ width: 20 }}>{it.k}</span>
              <div className="min-w-0" >
                <div className="text-sm text-ink" >{it.title}</div>
                <div className="flex gap-2 mt-1 items-center flex-wrap" >
                  <DojoChip>{it.type}</DojoChip><OriginChip origin={it.origin} />
                  <span className="mono text-xs text-ink-faint" >{it.attrib === "anonymized" ? "anonymized" : it.attrib}</span>
                  <DojoChip tone="var(--success)" soft="var(--success-soft)">generalised · {Math.round(it.conf * 100)}%</DojoChip>
                  {it.origin === "client" && <button onClick={e => e.stopPropagation()} className="mono text-xs text-accent border-0 cursor-pointer p-0" style={{ background: "none" }}>preview redaction →</button>}
                </div>
              </div>
              <button className="items-center gap-1 bg-paper border border-paper-edge rounded py-1 px-2 cursor-pointer" onClick={e => e.stopPropagation()} style={{ display: mobile ? "none" : "inline-flex" }}>
                <span className="kanji text-xs text-accent" >{it.scope.startsWith("Client") ? "客" : "技"}</span>
                <span className="text-xs text-ink-soft" >{it.scope}</span>
                <span className="text-xs text-ink-faint" >▾</span>
              </button>
              {!mobile && <span className="mono text-xs text-ink-faint" >→ triage</span>}
            </div>
          ))}
        </div>
        {/* Resolution (A finding forms): below-the-bar items stay out of the lane */}
        <div className="mt-4" >
          <div className="text-xs uppercase text-ink-faint font-semibold mb-2" style={{ letterSpacing: ".12em" }}>Still forming — below the bar</div>
          <div className="flex items-center gap-3 bg-paper rounded-lg py-3 px-4" style={{ border: "1px dashed var(--ink-faint)", opacity: 0.85 }}>
            <span className="kanji text-lg text-ink-faint text-center" style={{ width: 20 }}>{forming.k}</span>
            <div className="flex-1 min-w-0" >
              <div className="text-sm text-ink-soft" >{forming.title}</div>
              <div className="mono text-xs text-ink-faint mt-1" >{forming.reason}</div>
            </div>
            <DojoChip>generalised · {Math.round(forming.conf * 100)}%</DojoChip>
          </div>
        </div>
        <div className="text-xs text-ink-mute mt-4" style={{ lineHeight: 1.5 }}>
          You'll see the redacted preview — raw vs dropped — before anything leaves. Shared items enter the org's triage queue; a maintainer approves before distribution, and you can recall a share until it's approved.
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 5b · Contributions — watch it travel ──────────────── */
// Resolution (Watch it travel): a status timeline per contribution, a decision
// notification with a reason on decline, and credit when adopted.
function InappTravel() {
  const steps = ["Queued", "Triaged", "Decided"];
  const contribs = [
    { k: "守", title: "Validate webhook signatures before parsing", scope: "Client · Acme", origin: "client", at: 2, outcome: "approved", note: "Approved by Keiko T. · adopted by 4 repos — credited to you internally.", when: "1d ago" },
    { k: "問", title: "Integration-test persona for auth flows", scope: "Stack · React", origin: "employer", at: 1, outcome: null, note: "In review with Sven K. · 2 ahead in the queue.", when: "3d ago" },
    { k: "紋", title: "Retry-on-429 for the billing client", scope: "Team · Payments", origin: "employer", at: 2, outcome: "declined", note: "Declined — duplicates an existing guard; merged into the canonical one.", when: "5d ago" },
  ];
  const Stat = ({ value, label }) => (
    <div className="flex-1" >
      <div className="display text-2xl font-light text-ink" >{value}</div>
      <div className="text-xs uppercase text-ink-mute mt-1" style={{ letterSpacing: ".1em" }}>{label}</div>
    </div>
  );
  return (
    <InappFrame label="Contributions · watch it travel" title="Sensei  先生  ·  contributions">
      <IaHead kanji="旅" eyebrow="contributions" title="Where your shares went"
        sub="Every contribution carries a status timeline — and tells you the decision, with a reason on decline and credit when it's adopted. No silence after sharing." />
      <div className="flex-1 overflow-auto p-8" >
        <div className="flex gap-6 bg-paper-soft border border-paper-edge rounded-lg py-4 px-4 mb-4" >
          <Stat value="12" label="Shared · 30d" />
          <Stat value="9" label="Approved" />
          <Stat value="27" label="Repos adopting" />
          <Stat value="1" label="Declined" />
        </div>
        <div className="flex flex-col gap-3" >
          {contribs.map((c, idx) => {
            const decTone = c.outcome === "approved" ? "var(--success)" : c.outcome === "declined" ? "var(--danger)" : "var(--accent)";
            const decSoft = c.outcome === "approved" ? "var(--success-soft)" : c.outcome === "declined" ? "var(--danger-soft)" : "var(--accent-soft)";
            return (
            <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4" key={idx} >
              <div className="grid gap-3 items-start" style={{ gridTemplateColumns: "auto 1fr auto" }}>
                <span className="kanji text-xl text-accent" >{c.k}</span>
                <div className="min-w-0" >
                  <div className="text-sm text-ink" >{c.title}</div>
                  <div className="flex gap-2 mt-1 items-center" >
                    <DojoChip>{c.scope}</DojoChip><OriginChip origin={c.origin} />
                  </div>
                </div>
                <span className="mono text-xs text-ink-faint" >{c.when}</span>
              </div>
              {/* status stepper */}
              <div className="flex items-center gap-2 flex-wrap" style={{ margin: "var(--space-3) 0 var(--space-3)" }}>
                {steps.map((s, i) => {
                  const done = i <= c.at;
                  const isFinal = i === steps.length - 1;
                  const tone = isFinal && c.outcome ? decTone : "var(--accent)";
                  const soft = isFinal && c.outcome ? decSoft : "var(--accent-soft)";
                  const label = isFinal && c.outcome ? (c.outcome === "approved" ? "Approved" : "Declined") : s;
                  return (
                    <React.Fragment key={s}>
                      <span className="mono text-xs py-1 px-2 rounded-full" style={{
 color: done ? tone : "var(--ink-faint)", background: done ? soft : "var(--paper-mute)",
 border: done ? "1px solid transparent" : "var(--hairline)" }}>{done ? "✓ " : ""}{label}</span>
                      {i < steps.length - 1 && <span className="text-xs" style={{ color: i < c.at ? "var(--accent)" : "var(--ink-faint)" }}>→</span>}
                    </React.Fragment>
                  );
                })}
              </div>
              <div className="flex items-start gap-2 pt-3" style={{ borderTop: "1px solid var(--paper-edge)" }}>
                <span className="kanji text-sm" style={{ color: decTone }}>{c.outcome === "declined" ? "返" : c.outcome === "approved" ? "果" : "待"}</span>
                <span className="text-sm text-ink-soft flex-1" style={{ lineHeight: 1.5 }}>{c.note}</span>
                {!c.outcome && (
                  <button className="shrink-0 text-xs text-ink-soft bg-paper border border-paper-edge rounded py-1 px-3 cursor-pointer inline-flex items-center gap-1 whitespace-nowrap" style={{ fontFamily: "inherit" }}>
                    <span className="kanji text-xs text-accent" >戻</span> Recall
                  </button>
                )}
              </div>
            </div>
            );
          })}
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 5 · Collective — global ↔ company toggle ───────────── */
function InappCollective() {
  const [src, setSrc] = iaS("dojo");
  const cats = [
    { name: "Patterns & rules", g: true, d: true },
    { name: "Guards & lints", g: true, d: true },
    { name: "Prompts & personas", g: false, d: true },
    { name: "Skills & agents", g: false, d: true },
  ];
  const isDojo = src === "dojo";
  return (
    <InappFrame label="Collective · global ↔ company" title="Sensei  先生  ·  collective intel">
      <IaHead kanji="群" eyebrow="collective intelligence" title="Where your knowledge flows"
        sub="Two destinations, each with its own controls — the public Collective, and your company Dōjō. They never share settings." />
      <div className="flex-1 overflow-auto p-8" style={{ maxWidth: 860 }}>
        {/* segmented toggle */}
        <div className="inline-flex bg-paper-mute rounded-lg p-1 mb-6" >
          {[["collective", "群 Public Collective"], ["dojo", "結 Acme Dōjō"]].map(([id, label]) => {
            const on = src === id;
            return (
              <button className="py-2 px-4 rounded border-0 cursor-pointer text-sm" key={id} onClick={() => setSrc(id)} style={{ fontFamily: "inherit",
 background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-mute)",
 boxShadow: on ? "var(--shadow-sm)" : "none" }}>{label}</button>
            );
          })}
        </div>

        <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-6" >
          <div className="flex items-center gap-2 mb-4" >
            <span className="kanji text-xl text-accent" >{isDojo ? "結" : "群"}</span>
            <div>
              <div className="text-base text-ink" >{isDojo ? "Acme Dōjō" : "Public Collective"}</div>
              <div className="text-xs text-ink-mute" >{isDojo ? "private · governed · attributed to you internally" : "public commons · anonymized"}</div>
            </div>
            <span className="flex-1" />
            <span className="inline-flex items-center gap-1 text-xs text-ink-soft" >
              <StatusDot tone={isDojo ? "accent" : "ink-3"} /> sharing {isDojo ? "on · weekly" : "on · review first"}
            </span>
          </div>
          <div className="text-xs uppercase text-ink-faint font-semibold mb-2" style={{ letterSpacing: ".12em" }}>Categories shared {isDojo ? "to this Dōjō" : "publicly"}</div>
          <div className="flex flex-col gap-1" >
            {cats.map(c => {
              const on = isDojo ? c.d : c.g;
              return (
                <div className="flex items-center gap-2 py-2 px-0" key={c.name} style={{ borderBottom: "1px solid var(--paper-edge)" }}>
                  <span className="text-sm text-ink flex-1" >{c.name}</span>
                  {isDojo && <span className="mono text-xs text-ink-faint" >scoped by binding</span>}
                  <span className="rounded-full relative inline-block" style={{ width: 36, height: 20, background: on ? "var(--ink)" : "var(--paper-mute)" }}>
                    <span className="absolute rounded-full bg-paper" style={{ top: 2, left: on ? 18 : 2, width: 16, height: 16 }} />
                  </span>
                </div>
              );
            })}
          </div>
          {isDojo && (
            <div className="text-xs text-ink-mute mt-3" style={{ lineHeight: 1.5 }}>
              Client-bound projects are anonymized automatically before anything leaves. Separate cadence and filters from the public Collective.
            </div>
          )}
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 6 · Today / Upgrades — from your Dōjō ──────────────── */
function InappDownstream({ embedded }) {
  const iconBtn = { background: "var(--paper-mute)", border: "none", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-2)", cursor: "pointer", fontFamily: "var(--font-kanji)", fontSize: "var(--text-sm)", color: "var(--ink-mute)", lineHeight: 1 };
  const dojo = [
    { k: "守", title: "Never log refresh tokens, even at debug", scope: "Company", by: "approved by Keiko T.", impact: "+ prevents a known leak class", sup: true },
    { k: "紋", title: "Idempotency key on money-moving mutations", scope: "Team · Payments", by: "approved by Marco D.", impact: "+ matches 6 of your repos" },
  ];
  const global = [
    { k: "技", title: "Skill: explain a slow query plan", scope: "Postgres" },
  ];
  return (
    <InappFrame label="Upgrades · from your Dōjō" title="Sensei  先生  ·  upgrades" embedded={embedded}>
      <IaHead kanji="贈" eyebrow="upgrades" title="What's arrived for you"
        sub="Approved practice from your Dōjō lands here first — attributed and scoped to where you work — kept distinct from the public Collective. Mute or pin any scope; when rules conflict, the more specific scope wins." />
      <div className="flex-1 overflow-auto p-8" >
        {/* Resolution (Receive downstream): precedence ladder */}
        <div className="flex items-center gap-2 bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 mb-4" >
          <span className="kanji text-sm text-accent" >序</span>
          <span className="text-xs text-ink-soft" >Precedence</span>
          <div className="flex items-center gap-2" >
            {["Org", "Team", "Global", "Personal"].map((s, i) => (
              <React.Fragment key={s}>
                <span className="mono text-xs" style={{ color: i < 2 ? "var(--accent)" : "var(--ink-mute)" }}>{s}</span>
                {i < 3 && <span className="text-ink-faint text-xs" >›</span>}
              </React.Fragment>
            ))}
          </div>
          <span className="flex-1" />
          <span className="text-xs text-ink-mute" >the more specific scope wins</span>
        </div>
        {/* Dōjō lane */}
        <div className="flex items-center gap-2 mb-3" >
          <span className="kanji text-base text-accent" >結</span>
          <span className="text-xs uppercase text-accent font-semibold" style={{ letterSpacing: ".12em" }}>From your Dōjō · Acme</span>
          <span className="mono text-xs text-ink-faint" >2 new</span>
        </div>
        <div className="flex flex-col gap-3 mb-6" >
          {dojo.map((u, i) => (
            <div className="bg-paper-soft rounded-lg py-4 px-4 grid gap-3 items-center" key={i} style={{ border: "1px solid var(--accent-edge)", gridTemplateColumns: "auto 1fr auto" }}>
              <span className="kanji text-xl text-accent" >{u.k}</span>
              <div>
                <div className="text-sm text-ink" >{u.title}</div>
                <div className="flex gap-2 mt-1 items-center flex-wrap" >
                  <DojoChip tone="var(--accent)" soft="var(--accent-soft)">{u.scope}</DojoChip>
                  <span className="mono text-xs text-ink-mute" >{u.by}</span>
                  <span className="text-xs text-success" >{u.impact}</span>
                  {u.sup && <DojoChip tone="var(--ink-soft)">supersedes a Collective rule</DojoChip>}
                </div>
              </div>
              <div className="flex gap-2 items-center" >
                <button title="Pin this scope" style={iconBtn}>留</button>
                <button title="Mute this scope" style={iconBtn}>消</button>
                <button className="py-2 px-3" style={{ ...btnPrimary }}>Adopt</button>
                <button className="py-2 px-3" style={{ ...btnGhost }}>Defer</button>
              </div>
            </div>
          ))}
        </div>

        {/* Global lane */}
        <div className="flex items-center gap-2 mb-3" >
          <span className="kanji text-base text-ink-mute" >群</span>
          <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".12em" }}>From the Collective · public</span>
        </div>
        <div className="flex flex-col gap-3" >
          {global.map((u, i) => (
            <div className="bg-paper-soft border border-paper-edge rounded-lg py-4 px-4 grid gap-3 items-center" key={i} style={{ gridTemplateColumns: "auto 1fr auto" }}>
              <span className="kanji text-xl text-ink-mute" >{u.k}</span>
              <div>
                <div className="text-sm text-ink" >{u.title}</div>
                <div className="mt-1" ><DojoChip>{u.scope}</DojoChip></div>
              </div>
              <div className="flex gap-2" >
                <button className="py-2 px-3" style={{ ...btnGhost }}>Review</button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </InappFrame>
  );
}

/* ─── 4b · Redaction preview — raw vs dropped, before it leaves ── */
// The headline contributor promise made operational: never a blind send. For a
// client-origin finding, show exactly what travels (the lesson + what·why·impact
// + anonymized example) and exactly what is dropped (client · repo · identifiers
// · source), line by line, before the share is confirmed.
function InappRedact({ embedded, mobile = false }) {
  const dropped = [
    { k: "客", label: "Client & engagement", raw: "Acme Corp · billing-integration", tone: "var(--accent)" },
    { k: "庫", label: "Repo & paths", raw: "github.com/acme/lumen-auth · /webhooks/billing.ts", tone: "var(--accent)" },
    { k: "名", label: "Identifiers", raw: "ACME_WEBHOOK_SECRET · tenant_id=acme_prod", tone: "var(--accent)" },
    { k: "源", label: "Source reference", raw: "session s-2891 · commit 4f9c1a", tone: "var(--accent)" },
  ];
  const kept = [
    { k: "守", label: "The rule", v: "Validate the webhook HMAC signature before parsing the body." },
    { k: "憶", label: "What · why · impact", v: "Parsing before verifying let a forged payload reach the handler — a spoofed event mutated state. Verify first; reject on mismatch." },
    { k: "例", label: "Example · anonymized", v: "verify_signature(header, secret) → parse(body)  ·  identifiers dropped" },
  ];
  return (
    <InappFrame label="Project · redaction preview (raw vs dropped)" title="Sensei  先生  ·  lumen-auth" embedded={embedded}>
      <IaHead mobile={mobile} kanji="盾" eyebrow="Ready to share · client-origin" title="Here's exactly what leaves — and what doesn't"
        sub="This finding came from client work, so it's anonymized before anything leaves your machine. Review what's dropped below; nothing is sent until you confirm."
        right={<span className="inline-flex items-center gap-1 text-xs text-accent bg-accent-soft rounded-full py-1 px-2" style={{ fontFamily: "var(--font-mono)", border: "1px solid var(--accent-edge)" }}>盾 anonymized</span>} />

      <div className="flex-1 overflow-auto" style={{ padding: mobile ? "var(--space-4) var(--space-4) var(--space-2)" : "var(--space-6) var(--space-8) var(--space-2)" }}>
        <div className="grid gap-4 items-start" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(300px, 1fr))" }}>
          {/* kept */}
          <div className="bg-paper-soft rounded-lg overflow-hidden" style={{ border: "1px solid var(--success-edge)" }}>
            <div className="flex items-center gap-2 py-3 px-4 border-b" >
              <span className="rounded-full bg-success" style={{ width: 7, height: 7 }} />
              <span className="text-xs uppercase text-success" style={{ letterSpacing: ".12em", fontWeight: 700 }}>Travels upstream</span>
            </div>
            <div style={{ padding: "var(--space-1) var(--space-4) var(--space-3)" }}>
              {kept.map((x, i) => (
                <div className="py-3 px-0" key={i} style={{ borderBottom: i < kept.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <div className="flex items-center gap-2 mb-1" >
                    <span className="kanji text-sm text-success" >{x.k}</span>
                    <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".1em" }}>{x.label}</span>
                  </div>
                  <div className="text-sm text-ink" style={{ lineHeight: 1.55 }}>{x.v}</div>
                </div>
              ))}
            </div>
          </div>
          {/* dropped */}
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
            <div className="flex items-center gap-2 py-3 px-4 border-b" >
              <span className="rounded-full bg-ink-faint" style={{ width: 7, height: 7 }} />
              <span className="text-xs uppercase text-ink-mute" style={{ letterSpacing: ".12em", fontWeight: 700 }}>Dropped — never leaves</span>
            </div>
            <div style={{ padding: "var(--space-1) var(--space-4) var(--space-3)" }}>
              {dropped.map((x, i) => (
                <div className="py-3 px-0" key={i} style={{ borderBottom: i < dropped.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                  <div className="flex items-center gap-2 mb-1" >
                    <span className="kanji text-sm text-ink-faint" >{x.k}</span>
                    <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".1em" }}>{x.label}</span>
                    <span className="flex-1" />
                    <span className="mono text-xs text-ink-faint uppercase" style={{ letterSpacing: ".06em" }}>dropped</span>
                  </div>
                  <div className="mono text-xs text-ink-mute" style={{ lineHeight: 1.5, textDecoration: "line-through", textDecorationColor: "var(--ink-faint)", wordBreak: "break-all" }}>{x.raw}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
        <div className="flex items-start gap-2 mt-4 bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" style={{ borderLeft: "3px solid var(--accent)" }}>
          <span className="kanji text-base text-accent" >盾</span>
          <span className="text-sm text-ink-soft" style={{ lineHeight: 1.55 }}>
            If a lesson can't stand without identifying context, it's <b className="font-semibold text-ink" >dropped automatically</b> rather than weakened — never sent half-anonymized.
          </span>
        </div>
      </div>

      <div className="shrink-0 border-t flex items-center gap-3 flex-wrap" style={{ padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-8)" }}>
        {!mobile && <span className="text-xs text-ink-mute" >Destination · <b className="font-semibold text-ink-soft" >Acme Corp Dōjō</b> · triage queue</span>}
        <span className="flex-1" />
        <button style={btnGhost}>Cancel</button>
        <button style={btnPrimary}><span className="kanji text-sm text-accent" >送</span> Confirm &amp; share anonymized</button>
      </div>
    </InappFrame>
  );
}

Object.assign(window, {
  InappJoin, InappConnection, InappBind, InappShare, InappRedact, InappTravel, InappCollective, InappDownstream,
});
