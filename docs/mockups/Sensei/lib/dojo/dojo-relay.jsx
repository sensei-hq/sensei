// Dōjō · Relay — the away-from-keyboard surface, now on BOTH desktop and phone.
//
// Relay is no longer mobile-only. Its features live in one place as
// width-adaptive "bodies" that render identically in the desktop console's
// main area (wide) and inside a phone frame (narrow):
//   Projects · Inbox · Watch progress · Approve · Decision · Chat
//
// Consumed two ways, both consistent:
//   · Desktop — DojoRoleShell injects a "Relay · you" nav group at the top of
//     every role's rail; selecting one renders <RelayArea view=… wide/>.
//   · Mobile  — the DojoMobile* wrappers put the same body in a MobileFrame
//     with the Projects/Inbox/Chat/More bottom tabs.
//
// Architecture: the local daemon holds a live line to the Dōjō service
// (Supabase realtime), so these reach a running session with no pairing.
// Reuses DojoChip/OriginChip/DojoHead from dojo-shared.jsx. Token-only.

const { useState: mbS } = React;

/* ═══ data · per-user, across every Dōjō ═════════════════════ */
const flagMeta = {
  gate:    { tone: "var(--accent)",  soft: "var(--accent-soft)", edge: "var(--accent-edge)",  label: "decision waiting", cta: "Answer the decision", ck: "決" },
  approve: { tone: "var(--accent)",  soft: "var(--accent-soft)", edge: "var(--accent-edge)",  label: "approval waiting", cta: "Review the command", ck: "認" },
  doing:   { tone: "var(--success)", soft: "var(--success-soft)", edge: "var(--success-edge)", label: "on track" },
  stall:   { tone: "var(--warning)", soft: "var(--warning-soft)", edge: "var(--warning-edge)", label: "stalled", cta: "Nudge to continue", ck: "促" },
  done:    { tone: "var(--ink-mute)",   soft: "var(--paper-mute)", edge: "var(--paper-edge)",      label: "done" },
};
const kindTone = window.DOJO_KIND_TONE;
const DojoTag = (props) => React.createElement(window.DojoKindTag, props);
const MB_PROJECTS = [
  { id: "auth",   kanji: "鍵", name: "lumen-auth",     dojo: "Acme Corp", kind: "Employer", phase: 2, of: 4, pct: 47, now: "refresh-token store", flag: "approve", note: "approve migration" },
  { id: "bill",   kanji: "円", name: "billing-svc",    dojo: "Acme Corp", kind: "Employer", phase: 3, of: 5, pct: 61, now: "webhook retry tests", flag: "doing",   note: "running" },
  { id: "portal", kanji: "客", name: "initech-portal", dojo: "Initech",   kind: "Client",   phase: 1, of: 3, pct: 22, now: "session strategy",   flag: "gate",    note: "1 decision" },
  { id: "tele",   kanji: "測", name: "telemetry",      dojo: "No Dōjō · local",  kind: "Solo",     phase: 1, of: 3, pct: 18, now: "ingest schema",      flag: "stall",   note: "quiet 22m" },
];
const INBOX = [
  { id: "i1", kind: "approve",  k: "認", tone: "var(--accent)",  title: "Run prod migration", proj: "lumen-auth", dojo: "Acme Corp", w: "2m" },
  { id: "i2", kind: "decision", k: "決", tone: "var(--accent)",  title: "Which session strategy?", proj: "initech-portal", dojo: "Initech", w: "18m" },
  { id: "i3", kind: "stall",    k: "促", tone: "var(--warning)", title: "Ingest schema — quiet 22m", proj: "telemetry", dojo: "No Dōjō · local", w: "22m" },
];

/* ═══ small shared pieces ════════════════════════════════════ */
const Live = (props) => React.createElement(window.DojoLive, props);
function Bar({ pct, tone }) {
  return (
    <div style={{ height: 6, borderRadius: "var(--radius-sm)", background: "var(--paper-mute)", overflow: "hidden" }}>
      <div style={{ width: pct + "%", height: "100%", background: tone || "var(--ink)", borderRadius: "var(--radius-sm)" }} />
    </div>
  );
}

/* ═══ 1 · PROJECTS body (adaptive) ═══════════════════════════ */
function RelayProjectsBody({ wide = false, onOpen }) {
  const needs = MB_PROJECTS.filter(p => ["gate", "approve", "stall"].includes(p.flag));
  const rest = MB_PROJECTS.filter(p => !["gate", "approve", "stall"].includes(p.flag));
  const ordered = [...needs, ...rest];
  const pad = wide ? "var(--space-5)" : "var(--space-4)";
  return (
    <div style={{ height: "100%", overflow: "auto", padding: pad }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>場</span>
        <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Active projects</span>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{MB_PROJECTS.length}</span>
        <span style={{ flex: 1 }} />
        {needs.length > 0 && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>{needs.length} need you</span>}
      </div>
      <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
        {ordered.map((p, i) => {
          const f = flagMeta[p.flag];
          const acts = p.flag === "approve" || p.flag === "gate" || p.flag === "stall";
          return (
            <div key={p.id} style={{ display: "grid",
              gridTemplateColumns: wide ? "3px auto 1fr 150px auto" : "3px auto 1fr auto", gap: "var(--space-3)", alignItems: "center",
              borderTop: i === 0 ? "none" : "1px solid var(--paper-edge)", paddingRight: "var(--space-4)" }}>
              <span style={{ alignSelf: "stretch", background: acts ? f.tone : "transparent" }} />
              <span className="kanji" style={{ fontSize: "var(--text-lg)", color: f.tone, lineHeight: 1, padding: "var(--space-3) 0 var(--space-3) var(--space-3)" }}>{p.kanji}</span>
              <div style={{ minWidth: 0, padding: "var(--space-3) 0" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
                  <span className="mono" style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{p.name}</span>
                  <DojoTag p={p} />
                </div>
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>
                  <span style={{ color: f.tone, fontWeight: 600 }}>{f.label}</span> · {p.flag === "stall" ? "Paused" : "Now"}: {p.now}
                </div>
              </div>
              {wide && (
                <div>
                  <Bar pct={p.pct} tone={p.flag === "stall" ? "var(--warning)" : f.tone} />
                  <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>phase {p.phase}/{p.of} · {p.pct}%</div>
                </div>
              )}
              {acts
                ? <button onClick={() => onOpen && onOpen(p.flag)} style={{ justifySelf: "end", cursor: "pointer", fontFamily: "inherit",
                    display: "inline-flex", alignItems: "center", gap: "var(--space-1)", whiteSpace: "nowrap",
                    background: p.flag === "stall" ? "var(--paper)" : "var(--ink)", color: p.flag === "stall" ? "var(--ink)" : "var(--paper)",
                    boxShadow: p.flag === "stall" ? "inset 0 0 0 1px var(--paper-edge)" : "none",
                    border: "none", borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-3)", fontSize: "var(--text-xs)", fontWeight: 500 }}>
                    <span className="kanji" style={{ color: "var(--accent)" }}>{f.ck}</span>{f.cta}
                  </button>
                : <button onClick={() => onOpen && onOpen(p.flag)} title="Watch progress" style={{ justifySelf: "end", cursor: "pointer", fontFamily: "inherit", background: "none", border: "none", color: "var(--ink-faint)", fontSize: "var(--text-sm)" }}>→</button>}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/* ═══ 2 · INBOX body (adaptive · two-pane when wide) ═════════ */
function RelayInboxList({ sel, setSel }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
      {INBOX.map(it => {
        const on = sel === it.id;
        return (
          <button key={it.id} onClick={() => setSel(it.id)} style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", width: "100%", textAlign: "left", cursor: "pointer",
                background: on ? "var(--paper-mute)" : "var(--paper-soft)", border: on ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", fontFamily: "inherit" }}>
            <span style={{ width: 38, height: 38, borderRadius: "var(--radius-lg)", flexShrink: 0, background: "var(--paper-mute)", display: "flex", alignItems: "center", justifyContent: "center" }}>
              <span className="kanji" style={{ fontSize: "var(--text-lg)", color: it.tone }}>{it.k}</span>
            </span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em", textTransform: "uppercase", color: it.tone, fontWeight: 700 }}>{it.kind === "stall" ? "stalled" : it.kind}</span>
                <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{it.w}</span>
              </div>
              <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", marginTop: "var(--space-1)" }}>{it.title}</div>
              <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>{it.proj} · {it.dojo}</div>
            </div>
            <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-faint)" }}>→</span>
          </button>
        );
      })}
    </div>
  );
}
function RelayInboxBody({ wide = false }) {
  const [sel, setSel] = mbS(INBOX[0].id);
  const item = INBOX.find(x => x.id === sel) || INBOX[0];
  const intro = (
    <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)", lineHeight: 1.5, margin: "0 0 var(--space-3)" }}>
      Answer when you can — other tracks keep moving. Pushed live through your Dōjō.
    </p>
  );
  if (!wide) return <div style={{ height: "100%", overflow: "auto", padding: "var(--space-4)" }}>{intro}<RelayInboxList sel={sel} setSel={setSel} /></div>;
  return (
    <div style={{ display: "flex", height: "100%", minHeight: 0 }}>
      <div style={{ width: 380, flexShrink: 0, borderRight: "var(--hairline)", overflow: "auto", padding: "var(--space-5)" }}>
        {intro}<RelayInboxList sel={sel} setSel={setSel} />
      </div>
      <div style={{ flex: 1, minWidth: 0, overflow: "auto" }}>
        {item.kind === "approve" ? <RelayApproveBody wide /> : item.kind === "decision" ? <RelayDecisionBody wide /> : <RelayStallBody />}
      </div>
    </div>
  );
}

/* ═══ 3 · APPROVE body ═══════════════════════════════════════ */
function RelayApproveBody({ wide = false }) {
  const [done, setDone] = mbS(false);
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "var(--space-5)" : "var(--space-4) var(--space-4)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)" }}>認</span>
        <div>
          <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>sensei wants to run</div>
          <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>lumen-auth · Acme Corp · paused, waiting on you</div>
        </div>
      </div>
      <div style={{ background: "var(--paper-mute)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-3)" }}>
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, marginBottom: "var(--space-2)" }}>The command</div>
        <div className="mono" style={{ fontSize: "var(--text-sm)", color: "var(--ink)", lineHeight: 1.6, wordBreak: "break-all" }}>psql $DB &lt; migrations/003_refresh_tokens.sql</div>
      </div>
      <div style={{ display: "flex", gap: "var(--space-2)", margin: "var(--space-2) 0 var(--space-4)", flexWrap: "wrap" }}>
        <DojoChip tone="var(--warning)" soft="var(--warning-soft)">writes to prod</DojoChip>
        <DojoChip tone="var(--ink-soft)">creates 2 tables</DojoChip>
        <DojoChip tone="var(--ink-soft)">reversible · down migration</DojoChip>
      </div>
      <div style={{ marginBottom: "var(--space-2)", fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Why</div>
      <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.6, margin: "0 0 var(--space-4)" }}>
        The refresh-token store needs its tables before the new session flow can be tested. Adds <span className="mono" style={{ fontSize: "var(--text-xs)" }}>refresh_tokens</span> and <span className="mono" style={{ fontSize: "var(--text-xs)" }}>token_families</span>, with a matching down migration.
      </p>
      <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-3)", marginBottom: "var(--space-4)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>透</span>
          Reached you through your Dōjō — the daemon is holding the session open in real time.
        </div>
      </div>
      {done ? (
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--success)" }}>済</span>
          <div><div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>Approved · session resumed</div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>sensei is running the migration now — you'll be pinged if anything else needs you.</div></div>
        </div>
      ) : (
      <div style={{ display: "flex", flexDirection: wide ? "row" : "column", gap: "var(--space-2)" }}>
        <button onClick={() => setDone(true)} style={{ flex: 1, padding: "var(--space-3)", borderRadius: "var(--radius-lg)", border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-sm)", fontWeight: 600, fontFamily: "inherit", display: "flex", alignItems: "center", justifyContent: "center", gap: "var(--space-2)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>許</span> Approve &amp; continue
        </button>
        <button style={{ flex: wide ? "0 0 auto" : 1, padding: "var(--space-3) var(--space-4)", borderRadius: "var(--radius-lg)", border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-sm)", fontFamily: "inherit" }}>Deny</button>
        <button style={{ flex: wide ? "0 0 auto" : 1, padding: "var(--space-3) var(--space-4)", borderRadius: "var(--radius-lg)", border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-sm)", fontFamily: "inherit" }}>Ask a question</button>
      </div>
      )}
    </div>
  );
}

/* ═══ 4 · DECISION body ══════════════════════════════════════ */
function RelayDecisionBody({ wide = false }) {
  const [choice, setChoice] = mbS(2);
  const [sent, setSent] = mbS(false);
  const opts = [["Stateless JWT", "simplest"], ["Server sessions", "revocable"], ["Hybrid — JWT + refresh store", "recommended"]];
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "var(--space-5)" : "var(--space-4) var(--space-4)" }}>
      <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-2)" }}>sensei is asking · initech-portal</div>
      <div style={{ fontSize: "var(--text-lg)", color: "var(--ink)", fontWeight: 600, lineHeight: 1.35, marginBottom: "var(--space-4)" }}>Which session strategy should sensei use?</div>
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
        {opts.map((o, i) => {
          const s = i === choice;
          return (
            <button key={i} onClick={() => setChoice(i)} style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", width: "100%", textAlign: "left", cursor: "pointer",
                  background: s ? "var(--accent-soft)" : "var(--paper-soft)", fontFamily: "inherit",
                  border: s ? "1px solid var(--accent-edge)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
              <span style={{ width: 20, height: 20, borderRadius: "50%", flexShrink: 0, border: s ? "none" : "2px solid var(--paper-edge)", background: s ? "var(--accent)" : "transparent", display: "flex", alignItems: "center", justifyContent: "center" }}>
                {s && <span style={{ color: "var(--paper)", fontSize: "var(--text-xs)" }}>✓</span>}
              </span>
              <span style={{ fontSize: "var(--text-base)", color: "var(--ink)" }}><span style={{ fontWeight: s ? 600 : 500 }}>{o[0]}</span><span style={{ color: "var(--ink-mute)", fontWeight: 400 }}> · {o[1]}</span></span>
            </button>
          );
        })}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", margin: "var(--space-4) var(--space-1)" }}>
        <span style={{ flex: 1, height: 1, background: "var(--paper-edge)" }} /><span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>OR</span><span style={{ flex: 1, height: 1, background: "var(--paper-edge)" }} />
      </div>
      <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-3)", fontSize: "var(--text-sm)", color: "var(--ink-mute)", minHeight: 46 }}>Type your own answer…</div>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginTop: "var(--space-3)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>透</span> Reached you through your Dōjō · other tracks keep moving
      </div>
      {sent ? (
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", marginTop: "var(--space-4)", background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--success)" }}>済</span>
          <div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}><b style={{ fontWeight: 600 }}>Answer sent</b> — sensei is continuing the track.</div>
        </div>
      ) : (
        <button onClick={() => setSent(true)} style={{ width: "100%", marginTop: "var(--space-4)", padding: "var(--space-3)", borderRadius: "var(--radius-lg)", border: "none", cursor: "pointer", background: "var(--accent)", color: "var(--paper)", fontSize: "var(--text-base)", fontWeight: 600, fontFamily: "inherit" }}>Send answer</button>
      )}
    </div>
  );
}

/* ═══ 5 · STALL / nudge body ═════════════════════════════════ */
function RelayStallBody({ wide = false }) {
  const [nudged, setNudged] = mbS(false);
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "var(--space-5)" : "var(--space-4) var(--space-4)" }}>
      <div style={{ background: "var(--warning-soft)", border: "1px solid var(--warning-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-4)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--warning)" }}>静</span>
          <div><div style={{ fontSize: "var(--text-base)", fontWeight: 600, color: "var(--ink)" }}>This track has gone quiet</div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", marginTop: "var(--space-1)" }}>telemetry · No Dōjō · local · no activity 22m · waiting on API rate limit</div></div>
        </div>
        <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)", marginTop: "var(--space-3)" }}>sensei will retry on its own in ~8m. You can nudge it now.</div>
      </div>
      {nudged ? (
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", marginTop: "var(--space-4)", background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--success)" }}>済</span>
          <div><div style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>Nudged · track resumed</div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>sensei picked the work back up — you'll be pinged if it stalls again.</div></div>
        </div>
      ) : (
      <div style={{ display: "flex", flexDirection: wide ? "row" : "column", gap: "var(--space-2)", marginTop: "var(--space-4)" }}>
        <button onClick={() => setNudged(true)} style={{ flex: 1, padding: "var(--space-3)", borderRadius: "var(--radius-lg)", border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-sm)", fontWeight: 600, fontFamily: "inherit" }}>Nudge to continue</button>
        <button style={{ flex: 1, padding: "var(--space-3)", borderRadius: "var(--radius-lg)", border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-sm)", fontFamily: "inherit" }}>Pause track</button>
        <button style={{ flex: wide ? "0 0 auto" : 1, padding: "var(--space-3) var(--space-4)", borderRadius: "var(--radius-lg)", border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-soft)", fontSize: "var(--text-sm)", fontFamily: "inherit" }}>View logs</button>
      </div>
      )}
    </div>
  );
}

/* ═══ 6 · WATCH progress body (adaptive) ═════════════════════ */
function WatchPhases() {
  const phases = [
    { n: "01", name: "Foundation", state: "done", items: [] },
    { n: "02", name: "Session hardening", state: "doing", items: [
      { label: "Rotate signing keys", kind: "done" },
      { label: "Refresh-token store", kind: "doing", note: "writing store.ts · 14:08" },
      { label: "Run prod migration", kind: "gate", note: "approval waiting" },
    ]},
    { n: "03", name: "Rollout", state: "next", items: [] },
  ];
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
      {phases.map(ph => (
        <div key={ph.n} style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-3)", borderBottom: ph.items.length ? "var(--hairline)" : "none" }}>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{ph.n}</span>
            <span style={{ flex: 1, fontSize: "var(--text-sm)", color: ph.state === "next" ? "var(--ink-soft)" : "var(--ink)", fontWeight: 600 }}>{ph.name}</span>
            <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: ph.state === "doing" ? "var(--success)" : "var(--ink-mute)" }}>
              <span style={{ width: 6, height: 6, borderRadius: "50%", background: ph.state === "doing" ? "var(--success)" : ph.state === "done" ? "var(--ink-mute)" : "var(--paper-edge)" }} />
              {ph.state === "doing" ? "in progress" : ph.state}
            </span>
          </div>
          {ph.items.map((it, i) => {
            const done = it.kind === "done", gate = it.kind === "gate", doing = it.kind === "doing";
            return (
              <div key={i} style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-3)", padding: "var(--space-2) var(--space-3)", borderBottom: i < ph.items.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <span style={{ width: 18, height: 18, marginTop: "var(--space-1)", borderRadius: gate ? "var(--radius-sm)" : "50%", flexShrink: 0,
                      background: done ? "var(--ink)" : gate ? "var(--accent-soft)" : "transparent",
                      border: done ? "none" : `2px solid ${gate ? "var(--accent)" : doing ? "var(--success)" : "var(--paper-edge)"}`,
                      display: "flex", alignItems: "center", justifyContent: "center" }}>
                  {done && <span style={{ color: "var(--paper)", fontSize: "var(--text-xs)" }}>✓</span>}
                  {gate && <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>鍵</span>}
                  {doing && <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />}
                </span>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: "var(--text-sm)", color: done ? "var(--ink-mute)" : "var(--ink)", fontWeight: gate || doing ? 600 : 400 }}>{it.label}</div>
                  {it.note && <div style={{ fontSize: "var(--text-xs)", color: gate ? "var(--accent)" : doing ? "var(--success)" : "var(--ink-mute)", marginTop: "var(--space-1)" }}>{it.note}</div>}
                </div>
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}
function WatchActivity() {
  const feed = [
    { k: "書", t: "Wrote store.ts · 42 lines", w: "14:08", tone: "var(--ink-mute)" },
    { k: "試", t: "12 tests pass · 0 fail", w: "14:05", tone: "var(--success)" },
    { k: "認", t: "Paused for approval · prod migration", w: "14:04", tone: "var(--accent)" },
  ];
  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>刻</span>
        <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Activity</span>
        <span style={{ flex: 1 }} /><span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>full log →</span>
      </div>
      <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
        {feed.map((e, i) => (
          <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-3) var(--space-3)", borderBottom: i < feed.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: e.tone, width: 16, textAlign: "center" }}>{e.k}</span>
            <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>{e.t}</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{e.w}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
function RelayWatchBody({ wide = false }) {
  return (
    <div style={{ height: "100%", overflow: "auto", padding: wide ? "var(--space-5)" : "var(--space-4)" }}>
      <div style={{ fontFamily: "var(--font-display, serif)", fontSize: wide ? "var(--text-xl)" : "var(--text-lg)", color: "var(--ink)", fontWeight: 400, letterSpacing: "-0.01em" }}>OAuth &amp; session hardening</div>
      <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>lumen-auth · Acme Corp · auto</div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", margin: "var(--space-3) 0 var(--space-1)", maxWidth: 520 }}>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>Phase 2 of 4</span>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>47%</span>
      </div>
      <div style={{ maxWidth: 520 }}><Bar pct={47} /></div>
      <div style={{ display: "grid", gridTemplateColumns: wide ? "1.3fr 1fr" : "1fr", gap: wide ? "var(--space-5)" : "var(--space-4)", marginTop: "var(--space-4)", alignItems: "start" }}>
        <WatchPhases /><WatchActivity />
      </div>
    </div>
  );
}

/* ═══ 7 · CHAT body (adaptive) ═══════════════════════════════ */
function RelayChatBody({ wide = false }) {
  const thread = [
    { who: "sensei", t: "Paused on lumen-auth — the refresh-token store needs a prod migration before I can test the new flow." },
    { who: "you", t: "does the down migration drop the tables cleanly?" },
    { who: "sensei", t: "Yes — 003_refresh_tokens.down.sql drops both tables and the index, no data kept. Safe to roll back." },
    { who: "you", t: "ok, go ahead and run it" },
    { who: "sensei", t: "Running the migration now. I'll pick the session flow tests back up and ping you if anything else needs a decision." },
  ];
  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", minHeight: 0 }}>
      <div style={{ flex: 1, overflow: "auto", padding: wide ? "var(--space-5)" : "var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
        <div style={{ width: "100%", maxWidth: wide ? 720 : "none", margin: wide ? "0 auto" : 0, display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
          {thread.map((m, i) => {
            const me = m.who === "you";
            return (
              <div key={i} style={{ display: "flex", flexDirection: "column", alignItems: me ? "flex-end" : "flex-start", gap: "var(--space-1)" }}>
                {!me && <span className="kanji" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", marginLeft: "var(--space-1)" }}>先生</span>}
                <div style={{ maxWidth: "82%", padding: "var(--space-2) var(--space-3)", borderRadius: "var(--radius-lg)", background: me ? "var(--ink)" : "var(--paper-soft)", color: me ? "var(--paper)" : "var(--ink)",
                      border: me ? "none" : "var(--hairline)", borderBottomRightRadius: me ? 3 : "var(--radius-lg)", borderBottomLeftRadius: me ? "var(--radius-lg)" : 3, fontSize: "var(--text-sm)", lineHeight: 1.5 }}>{m.t}</div>
              </div>
            );
          })}
        </div>
      </div>
      <div style={{ flexShrink: 0, borderTop: "var(--hairline)", padding: wide ? "var(--space-3) var(--space-5)" : "var(--space-3) var(--space-4)" }}>
        <div style={{ maxWidth: wide ? 720 : "none", margin: wide ? "0 auto" : 0, display: "flex", gap: "var(--space-2)", alignItems: "center" }}>
          <div style={{ flex: 1, background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-full)", padding: "var(--space-3) var(--space-4)", fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>Message sensei…</div>
          <button style={{ width: 42, height: 42, borderRadius: "50%", border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-base)", flexShrink: 0 }}>↑</button>
        </div>
      </div>
    </div>
  );
}

/* ═══ SESSION LOG body (adaptive) ════════════════════════════ */
function RelayLogsBody({ wide = false }) {
  const rows = [
    { t: "14:08", k: "書", tone: "var(--ink-mute)", txt: "Wrote store.ts · 42 lines" },
    { t: "14:05", k: "試", tone: "var(--success)", txt: "12 tests pass · 0 fail" },
    { t: "14:04", k: "認", tone: "var(--accent)", txt: "Paused for approval · prod migration" },
    { t: "14:01", k: "案", tone: "var(--ink-mute)", txt: "Planned refresh-token store schema" },
    { t: "13:58", k: "讀", tone: "var(--ink-mute)", txt: "Read token_families migration history" },
    { t: "13:55", k: "始", tone: "var(--ink-mute)", txt: "Started phase 2 · session hardening" },
  ];
  return (
    <div style={{ height: "100%", overflow: "auto", padding: wide ? "var(--space-5)" : "var(--space-4)" }}>
      <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginBottom: "var(--space-3)" }}>lumen-auth · Acme Corp · newest first</div>
      <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden", maxWidth: wide ? 720 : "none" }}>
        {rows.map((e, i) => (
          <div key={i} style={{ display: "grid", gridTemplateColumns: "52px auto 1fr", gap: "var(--space-3)", alignItems: "center", padding: "var(--space-3) var(--space-4)", borderBottom: i < rows.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{e.t}</span>
            <span className="kanji" style={{ fontSize: "var(--text-base)", color: e.tone, width: 18, textAlign: "center" }}>{e.k}</span>
            <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{e.txt}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

const RELAY_CONN = {
  offline: { k: "絶", title: "Connection lost", sub: "Can't reach this session right now.", body: "The daemon on your machine isn't answering through the Dōjō. Your place is saved — actions resume the moment it reconnects.", cta: "Retry now", tone: "var(--warning)" },
  ended:   { k: "了", title: "Session ended", sub: "This track has finished.", body: "sensei closed the session — nothing more needs you here. The outcome and every step are in the log.", cta: "View session log", tone: "var(--ink-mute)" },
  asleep:  { k: "眠", title: "Daemon asleep", sub: "Your machine is idle.", body: "The daemon is sleeping to save power. Wake it to resume now, or it picks up on its own when you're back at the keyboard.", cta: "Wake daemon", tone: "var(--ink-mute)" },
};
function RelayConnState({ kind = "offline", wide = false }) {
  const s = RELAY_CONN[kind] || RELAY_CONN.offline;
  const off = kind !== "ended";
  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: "var(--space-3)", textAlign: "center", padding: wide ? "var(--space-7)" : "var(--space-6) var(--space-5)" }}>
      <span className="kanji" style={{ fontSize: "var(--text-4xl)", color: s.tone, lineHeight: 1 }}>{s.k}</span>
      <div className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 400, letterSpacing: "-0.01em", color: "var(--ink)" }}>{s.title}</div>
      <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>{s.sub}</div>
      <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)", lineHeight: 1.6, margin: "var(--space-1) 0 var(--space-1)", maxWidth: 360 }}>{s.body}</p>
      <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: off ? "var(--warning)" : "var(--ink-mute)",
        background: off ? "var(--warning-soft)" : "var(--paper-mute)", border: off ? "1px solid var(--warning-edge)" : "var(--hairline)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-2)" }}>
        <span style={{ width: 6, height: 6, borderRadius: "50%", background: off ? "var(--warning)" : "var(--ink-faint)" }} />{off ? "offline · through your Dōjō" : "session closed"}
      </span>
      <button style={{ marginTop: "var(--space-2)", padding: "var(--space-3) var(--space-4)", borderRadius: "var(--radius-lg)", border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: "var(--text-sm)", fontWeight: 500, fontFamily: "inherit" }}>{s.cta}</button>
    </div>
  );
}

/* ═══ DESKTOP dispatcher — headered wide area ════════════════ */
const RELAY_META = {
  projects: { kanji: "場", eyebrow: "Relay · you", title: "Active projects", sub: "Everything running for you — across every Dōjō and your own solo work, no Dōjō required. Approvals, decisions and stalled tracks rise to the top, pushed live. Relay is free · never gated by plan." },
  inbox:    { kanji: "決", eyebrow: "Relay · you", title: "Inbox", sub: "Decisions, approvals and nudges waiting on you — across every Dōjō and solo projects alike. Answer when you can; other tracks keep moving." },
  watch:    { kanji: "観", eyebrow: "Relay · you", title: "Watch progress", sub: "Phases, what's done · doing · next, and the live activity feed for a running track." },
  chat:     { kanji: "話", eyebrow: "Relay · you", title: "Chat with sensei", sub: "Talk to a running session mid-flight — ask a question, steer a decision, or give the go-ahead." },
  logs:     { kanji: "録", eyebrow: "Relay · you", title: "Session log", sub: "Every step sensei took on this track — timestamped, from first action to now." },
  approve:  { kanji: "認", eyebrow: "Relay · you", title: "Approve a gated action", sub: "The exact command sensei wants to run — approve, edit, or decline." },
  decision: { kanji: "決", eyebrow: "Relay · you", title: "A decision", sub: "Pick an option so the run can continue — or type your own." },
  stall:    { kanji: "促", eyebrow: "Relay · you", title: "Nudge a stalled track", sub: "This track is paused waiting on you — nudge it back into motion." },
};
function RelayArea({ view = "projects", wide = true, onOpen }) {
  const m = RELAY_META[view] || RELAY_META.projects;
  const Head = window.DojoHead;
  const body = view === "inbox" ? <RelayInboxBody wide={wide} />
    : view === "watch" ? <RelayWatchBody wide={wide} />
    : view === "chat" ? <RelayChatBody wide={wide} />
    : view === "logs" ? <RelayLogsBody wide={wide} />
    : view === "approve" ? <RelayApproveBody wide={wide} />
    : view === "decision" ? <RelayDecisionBody wide={wide} />
    : view === "stall" ? <RelayStallBody wide={wide} />
    : <RelayProjectsBody wide={wide} onOpen={onOpen} />;
  // chat & inbox manage their own scroll; projects/watch scroll inside body
  const flush = view === "chat" || view === "inbox";
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      {Head && <Head kanji={m.kanji} eyebrow={m.eyebrow} title={m.title} sub={m.sub} right={<Live />} />}
      <div style={{ flex: 1, minHeight: 0, overflow: flush ? "hidden" : "auto" }}>{body}</div>
    </div>
  );
}

/* ═══ MOBILE wrappers — same bodies, phone frame + tabs ══════ */
function MobileFrame({ children, label }) {
  return (
    <div className="sensei" data-screen-label={label} style={{ width: 390, height: 844, borderRadius: 44, background: "var(--paper)",
      border: "1px solid var(--paper-edge)", boxShadow: "var(--shadow-lg)", overflow: "hidden", position: "relative", display: "flex", flexDirection: "column" }}>
      <div style={{ flexShrink: 0, height: 44, display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0 var(--space-5)", fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>
        <span className="mono">9:41</span><span style={{ display: "inline-flex", gap: "var(--space-1)", alignItems: "center", opacity: 0.8 }}><span style={{ fontSize: "var(--text-xs)" }}>●●●</span><span style={{ fontSize: "var(--text-xs)" }}>◗</span></span>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>{children}</div>
      <div style={{ flexShrink: 0, height: 22, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <span style={{ width: 128, height: 5, borderRadius: "var(--radius-sm)", background: "var(--ink-faint)", opacity: 0.5 }} />
      </div>
    </div>
  );
}
function MHead({ title, sub, back, right, live }) {
  return (
    <div style={{ flexShrink: 0, padding: "var(--space-1) var(--space-4) var(--space-3)", borderBottom: "var(--hairline)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
        {back && <span className="mono" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>←</span>}
        <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", lineHeight: 1 }}>結</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: "var(--text-base)", color: "var(--ink)", fontWeight: 600, lineHeight: 1.1 }}>{title}</div>
          {sub && <div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>{sub}</div>}
        </div>
        {live && <Live />}{right}
      </div>
    </div>
  );
}
const MOBILE_TABS = window.DOJO_MOBILE_TABS;
function MTabs({ active }) {
  return <window.DojoTabBar tabs={MOBILE_TABS} active={active} />;
}

function DojoMobileProjects() {
  return <MobileFrame label="Mobile · Projects (home)"><MHead title="Projects" sub="everything running for you · your Dōjōs + solo" live /><div style={{ flex: 1, minHeight: 0 }}><RelayProjectsBody wide={false} /></div><MTabs active="projects" /></MobileFrame>;
}
function DojoMobileInbox() {
  return <MobileFrame label="Mobile · Inbox"><MHead title="Inbox" sub="what needs you · across Dōjōs" live /><div style={{ flex: 1, minHeight: 0 }}><RelayInboxBody wide={false} /></div><MTabs active="inbox" /></MobileFrame>;
}
function DojoMobileProject() {
  return <MobileFrame label="Mobile · Watch progress"><MHead title="lumen-auth" sub="Acme Corp · auto" back live /><div style={{ flex: 1, minHeight: 0 }}><RelayWatchBody wide={false} /></div></MobileFrame>;
}
function DojoMobileApprove() {
  return <MobileFrame label="Mobile · Approve action"><MHead title="Approval" sub="lumen-auth · Acme Corp" back live /><div style={{ flex: 1, minHeight: 0, overflow: "auto" }}><RelayApproveBody wide={false} /></div></MobileFrame>;
}
function DojoMobileDecision() {
  return <MobileFrame label="Mobile · Decision"><MHead title="Decision" sub="initech-portal · Initech" back live /><div style={{ flex: 1, minHeight: 0, overflow: "auto" }}><RelayDecisionBody wide={false} /></div></MobileFrame>;
}
function DojoMobileChat() {
  return <MobileFrame label="Mobile · Chat with sensei"><MHead title="sensei" sub="lumen-auth · live session" back live /><div style={{ flex: 1, minHeight: 0 }}><RelayChatBody wide={false} /></div></MobileFrame>;
}
function DojoMobileMore() {
  const consoles = [
    { k: "全", name: "Admin", sub: "Overview · Members · Scopes", role: true },
    { k: "門", name: "Maintainer", sub: "Triage · Knowledge", role: true },
    { k: "客", name: "Lead", sub: "Clients · Audit" },
    { k: "弟", name: "Developer", sub: "Teams · Contributions" },
  ];
  const orgs = [["社", "Acme Corp", "Org admin"], ["客", "Initech", "Contributor"], ["群", "Rust Guild", "Read-only"], ["己", "Personal", "Owner"]];
  return (
    <MobileFrame label="Mobile · More (consoles)">
      <MHead title="More" sub="Rin Saito · rin-saito" />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4) var(--space-4)" }}>
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-2)" }}>Consoles · this Dōjō</div>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", marginBottom: "var(--space-5)" }}>
          {consoles.map((c, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", opacity: c.role ? 1 : 0.55 }}>
              <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)", width: 24, textAlign: "center" }}>{c.k}</span>
              <div style={{ flex: 1, minWidth: 0 }}><div style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>{c.name} console</div><div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>{c.sub}</div></div>
              {c.role ? <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-faint)" }}>→</span> : <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>no access</span>}
            </div>
          ))}
        </div>
        <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-2)" }}>Switch Dōjō</div>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
          {orgs.map(([k, n, r], i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: i === 0 ? "var(--paper-soft)" : "transparent", border: i === 0 ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 22, textAlign: "center" }}>{k}</span>
              <span style={{ flex: 1, fontSize: "var(--text-sm)", color: "var(--ink)" }}>{n}</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{r}</span>
            </div>
          ))}
        </div>
      </div>
      <MTabs active="more" />
    </MobileFrame>
  );
}
function DojoMobileConnect() {
  const steps = [
    { k: "机", t: "Daemon", s: "on your Mac", note: "holds the session open" },
    { k: "結", t: "Dōjō service", s: "Supabase realtime", note: "relays live, no pairing" },
    { k: "携", t: "Your phone", s: "Dōjō in the browser", note: "approve · decide · watch" },
  ];
  return (
    <MobileFrame label="Mobile · How it connects">
      <MHead title="How Relay works now" sub="through your Dōjō" />
      <div style={{ flex: 1, overflow: "auto", padding: "var(--space-5) var(--space-4)" }}>
        <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.6, margin: "0 0 var(--space-5)" }}>No device pairing, no separate install. The daemon keeps a live line to the Dōjō; your phone just opens the Dōjō and picks up the session in real time.</p>
        <div style={{ display: "flex", flexDirection: "column" }}>
          {steps.map((st, i) => (
            <React.Fragment key={i}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
                <span className="kanji" style={{ fontSize: "var(--text-2xl)", color: "var(--accent)", width: 32, textAlign: "center" }}>{st.k}</span>
                <div style={{ flex: 1 }}><div style={{ fontSize: "var(--text-base)", color: "var(--ink)", fontWeight: 600 }}>{st.t}</div><div className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "var(--space-1)" }}>{st.s}</div><div style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", marginTop: "var(--space-1)" }}>{st.note}</div></div>
              </div>
              {i < steps.length - 1 && <div style={{ display: "flex", flexDirection: "column", alignItems: "center", padding: "var(--space-1) 0" }}><span style={{ fontSize: "var(--text-lg)", color: "var(--success)" }}>↕</span><span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--success)" }}>live · realtime</span></div>}
            </React.Fragment>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", marginTop: "var(--space-5)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--ink-mute)" }}>携</span>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.55 }}>The native Relay app still works too — it stays for push notifications and offline. This browser experience mirrors it.</span>
        </div>
      </div>
    </MobileFrame>
  );
}

function DojoMobileLogs() {
  return <MobileFrame label="Mobile · Session log"><MHead title="Session log" sub="lumen-auth · Acme Corp" back /><div style={{ flex: 1, minHeight: 0 }}><RelayLogsBody wide={false} /></div></MobileFrame>;
}

function DojoMobileConn({ kind = "offline" }) {
  return <MobileFrame label={"Mobile · Relay " + kind}><MHead title="Relay" sub="lumen-auth · Acme Corp" back /><div style={{ flex: 1, minHeight: 0 }}><RelayConnState kind={kind} /></div></MobileFrame>;
}

function MobileMoreBody({ onOpen }) {
  const links = [["承", "Approve a gated action", "approve"], ["決", "Answer a decision", "decision"], ["録", "Session log", "logs"]];
  const orgs = [["社", "Acme Corp", "Admin"], ["客", "Globex", "Maintainer"], ["客", "Initech", "Lead"], ["己", "Personal", "Owner"]];
  return (
    <div style={{ flex: 1, overflow: "auto", padding: "var(--space-4)" }}>
      <div className="zs-eyebrow font-semibold" style={{ marginBottom: "var(--space-2)" }}>Away from keyboard</div>
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", marginBottom: "var(--space-5)" }}>
        {links.map(([k, n, id]) => (
          <button key={id} onClick={() => onOpen(id)} style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", textAlign: "left", cursor: "pointer", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", fontFamily: "inherit" }}>
            <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 22, textAlign: "center" }}>{k}</span>
            <span style={{ flex: 1, fontSize: "var(--text-sm)", color: "var(--ink)" }}>{n}</span>
            <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-faint)" }}>→</span>
          </button>
        ))}
      </div>
      <div className="zs-eyebrow font-semibold" style={{ marginBottom: "var(--space-2)" }}>Switch Dōjō</div>
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
        {orgs.map(([k, n, r], i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", background: i === 0 ? "var(--paper-soft)" : "transparent", border: i === 0 ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-lg)", color: "var(--accent)", width: 22, textAlign: "center" }}>{k}</span>
            <span style={{ flex: 1, fontSize: "var(--text-sm)", color: "var(--ink)" }}>{n}</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{r}</span>
          </div>
        ))}
      </div>
      <div className="mono" style={{ marginTop: "var(--space-5)", fontSize: "var(--text-xs)", color: "var(--ink-faint)", display: "flex", alignItems: "center", gap: "var(--space-1)" }}>
        <span className="kanji">結</span>Dōjō v0.4.2
      </div>
    </div>
  );
}
/* One WIRED mobile experience — bottom tabs + drill-in/back move between the
   same Relay bodies. Replaces the pile of standalone DojoMobile* artboards. */
function DojoMobileFlow({ start = "signin" }) {
  const [view, setView] = mbS(start);    // signin | app
  const [tab, setTab] = mbS("projects");  // projects | inbox | chat | more
  const [drill, setDrill] = mbS(null);   // null | project | approve | decision | logs
  const go = (id) => { setDrill(null); setTab(id); };
  if (view === "signin") {
    return (
      <MobileFrame label="Mobile flow · Sign in">
        <window.DojoSignIn mobile onContinue={() => setView("app")} />
      </MobileFrame>
    );
  }
  const HEAD = {
    projects: { title: "Projects", sub: "everything running for you · your Dōjōs + solo", live: true },
    inbox:    { title: "Inbox", sub: "what needs you · across Dōjōs", live: true },
    chat:     { title: "sensei", sub: "lumen-auth · live session", live: true },
    more:     { title: "More", sub: "Rin Saito · rin-saito" },
    project:  { title: "lumen-auth", sub: "Acme Corp · auto", back: true, live: true },
    approve:  { title: "Approval", sub: "lumen-auth · Acme Corp", back: true, live: true },
    decision: { title: "Decision", sub: "initech-portal · Initech", back: true, live: true },
    logs:     { title: "Session log", sub: "lumen-auth · Acme Corp", back: true },
    stall:    { title: "Nudge a stalled track", sub: "initech-portal · Initech", back: true, live: true },
  };
  const h = HEAD[drill || tab];
  const openFlag = (flag) => setDrill(flag === "approve" ? "approve" : flag === "gate" ? "decision" : flag === "stall" ? "stall" : "project");
  let body;
  if (drill === "project") body = <RelayWatchBody wide={false} />;
  else if (drill === "approve") body = <RelayApproveBody wide={false} />;
  else if (drill === "decision") body = <RelayDecisionBody wide={false} />;
  else if (drill === "stall") body = <RelayStallBody wide={false} />;
  else if (drill === "logs") body = <RelayLogsBody wide={false} />;
  else if (tab === "inbox") body = <RelayInboxBody wide={false} />;
  else if (tab === "chat") body = <RelayChatBody wide={false} />;
  else if (tab === "more") body = <MobileMoreBody onOpen={setDrill} />;
  else body = <RelayProjectsBody wide={false} onOpen={openFlag} />;
  return (
    <MobileFrame label="Mobile flow · Relay">
      <div onClick={h.back ? () => setDrill(null) : undefined} style={h.back ? { cursor: "pointer" } : undefined}>
        <MHead title={h.title} sub={h.sub} back={h.back} live={h.live} />
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>{body}</div>
      {!drill && <window.DojoTabBar tabs={MOBILE_TABS} active={tab} onNav={go} />}
    </MobileFrame>
  );
}

Object.assign(window, {
  RelayArea, RELAY_META, RelayLogsBody, RelayConnState, DojoMobileFlow,
  RelayProjectsBody, RelayInboxBody, RelayWatchBody, RelayChatBody, RelayApproveBody, RelayDecisionBody, RelayStallBody,
  MobileFrame, DojoMobileProjects, DojoMobileProject, DojoMobileApprove, DojoMobileLogs, DojoMobileConn,
  DojoMobileDecision, DojoMobileInbox, DojoMobileChat, DojoMobileMore, DojoMobileConnect,
});
