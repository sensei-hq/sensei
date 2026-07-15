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
  gate:    { tone: "var(--accent)",  soft: "var(--accent-soft)",  label: "decision waiting" },
  approve: { tone: "var(--accent)",  soft: "var(--accent-soft)",  label: "approval waiting" },
  doing:   { tone: "var(--success)", soft: "var(--success-soft)", label: "on track" },
  stall:   { tone: "var(--warning)", soft: "var(--warning-soft)", label: "stalled" },
  done:    { tone: "var(--ink-3)",   soft: "var(--paper-3)",      label: "done" },
};
const kindTone = { Employer: "var(--ink-2)", Client: "var(--accent)", Community: "var(--success)", Personal: "var(--ink-3)" };
const kindKanji = { Employer: "社", Client: "客", Community: "群", Personal: "己" };
function DojoTag({ p }) {
  const tone = kindTone[p.kind] || "var(--ink-3)";
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 10.5, color: tone,
      background: `color-mix(in oklch, ${tone} 12%, transparent)`, border: `1px solid color-mix(in oklch, ${tone} 28%, transparent)`,
      borderRadius: 999, padding: "2px 8px", whiteSpace: "nowrap" }}>
      <span className="kanji" style={{ fontSize: 11 }}>{kindKanji[p.kind] || "結"}</span>{p.dojo}
    </span>
  );
}
const MB_PROJECTS = [
  { id: "auth",   kanji: "鍵", name: "lumen-auth",     dojo: "Acme Corp", kind: "Employer", phase: 2, of: 4, pct: 47, now: "refresh-token store", flag: "approve", note: "approve migration" },
  { id: "bill",   kanji: "円", name: "billing-svc",    dojo: "Acme Corp", kind: "Employer", phase: 3, of: 5, pct: 61, now: "webhook retry tests", flag: "doing",   note: "running" },
  { id: "portal", kanji: "客", name: "initech-portal", dojo: "Initech",   kind: "Client",   phase: 1, of: 3, pct: 22, now: "session strategy",   flag: "gate",    note: "1 decision" },
  { id: "tele",   kanji: "測", name: "telemetry",      dojo: "Personal",  kind: "Personal", phase: 1, of: 3, pct: 18, now: "ingest schema",      flag: "stall",   note: "quiet 22m" },
];
const INBOX = [
  { id: "i1", kind: "approve",  k: "認", tone: "var(--accent)",  title: "Run prod migration", proj: "lumen-auth", dojo: "Acme Corp", w: "2m" },
  { id: "i2", kind: "decision", k: "決", tone: "var(--accent)",  title: "Which session strategy?", proj: "initech-portal", dojo: "Initech", w: "18m" },
  { id: "i3", kind: "stall",    k: "促", tone: "var(--warning)", title: "Ingest schema — quiet 22m", proj: "telemetry", dojo: "Personal", w: "22m" },
];

/* ═══ small shared pieces ════════════════════════════════════ */
function Live() {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 10.5, color: "var(--success)",
                  background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: 999, padding: "3px 9px" }}>
      <span style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--success)" }} />live
    </span>
  );
}
function Bar({ pct, tone }) {
  return (
    <div style={{ height: 6, borderRadius: 3, background: "var(--paper-3)", overflow: "hidden" }}>
      <div style={{ width: pct + "%", height: "100%", background: tone || "var(--ink)", borderRadius: 3 }} />
    </div>
  );
}

/* ═══ 1 · PROJECTS body (adaptive) ═══════════════════════════ */
function RelayProjectsBody({ wide = false, onOpen }) {
  const needs = MB_PROJECTS.filter(p => ["gate", "approve", "stall"].includes(p.flag));
  const pad = wide ? 28 : 16;
  return (
    <div style={{ height: "100%", overflow: "auto", padding: pad }}>
      {/* needs you */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>要</span>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Needs you</span>
        <span className="mono" style={{ fontSize: 11, color: "var(--accent)" }}>{needs.length}</span>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: wide ? "repeat(auto-fill, minmax(300px, 1fr))" : "1fr", gap: 12, marginBottom: 26 }}>
        {needs.map(p => {
          const f = flagMeta[p.flag];
          return (
            <div key={p.id} style={{ background: f.soft, border: `1px solid color-mix(in oklch, ${f.tone} 30%, transparent)`, borderRadius: 14, padding: "14px 16px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <span className="kanji" style={{ fontSize: 18, color: f.tone }}>{p.kanji}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className="mono" style={{ fontSize: 12.5, color: "var(--ink)" }}>{p.name}</div>
                  <div style={{ marginTop: 4 }}><DojoTag p={p} /></div>
                </div>
                <span style={{ fontSize: 10.5, fontWeight: 600, color: f.tone }}>{f.label}</span>
              </div>
              <button onClick={onOpen} style={{ width: "100%", marginTop: 12, padding: "10px", borderRadius: 9, border: "none", cursor: "pointer",
                    background: p.flag === "stall" ? "var(--paper)" : "var(--ink)", color: p.flag === "stall" ? "var(--ink)" : "var(--paper)",
                    boxShadow: p.flag === "stall" ? "inset 0 0 0 1px var(--edge)" : "none",
                    fontSize: 13, fontWeight: 500, fontFamily: "inherit", display: "flex", alignItems: "center", justifyContent: "center", gap: 7 }}>
                <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>{p.flag === "approve" ? "認" : p.flag === "gate" ? "決" : "促"}</span>
                {p.flag === "approve" ? "Review the command" : p.flag === "gate" ? "Answer the decision" : "Nudge to continue"}
              </button>
            </div>
          );
        })}
      </div>
      {/* all active */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 14, color: "var(--ink-3)" }}>場</span>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Active</span>
        <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{MB_PROJECTS.length}</span>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: wide ? "repeat(auto-fill, minmax(300px, 1fr))" : "1fr", gap: 12 }}>
        {MB_PROJECTS.map(p => {
          const f = flagMeta[p.flag];
          return (
            <button key={p.id} onClick={onOpen} style={{ textAlign: "left", cursor: "pointer", background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 14, padding: "14px 16px", fontFamily: "inherit" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <span className="kanji" style={{ fontSize: 17, color: "var(--ink-3)" }}>{p.kanji}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className="mono" style={{ fontSize: 12.5, color: "var(--ink)" }}>{p.name}</div>
                  <div style={{ marginTop: 5 }}><DojoTag p={p} /></div>
                </div>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 10.5, color: f.tone }}>
                  <span style={{ width: 6, height: 6, borderRadius: "50%", background: f.tone }} />{f.label}
                </span>
              </div>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", margin: "11px 0 6px" }}>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>Phase {p.phase} of {p.of}</span>
                <span className="mono" style={{ fontSize: 11, color: "var(--ink-3)" }}>{p.pct}%</span>
              </div>
              <Bar pct={p.pct} tone={p.flag === "stall" ? "var(--warning)" : "var(--ink)"} />
              <div style={{ fontSize: 11.5, color: "var(--ink-2)", marginTop: 9 }}>{p.flag === "stall" ? "Paused · " : "Now · "}{p.now}</div>
            </button>
          );
        })}
      </div>
    </div>
  );
}

/* ═══ 2 · INBOX body (adaptive · two-pane when wide) ═════════ */
function RelayInboxList({ sel, setSel }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {INBOX.map(it => {
        const on = sel === it.id;
        return (
          <button key={it.id} onClick={() => setSel(it.id)} style={{ display: "flex", alignItems: "center", gap: 12, width: "100%", textAlign: "left", cursor: "pointer",
                background: on ? "var(--paper-3)" : "var(--paper-2)", border: on ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: 13, padding: "13px 15px", fontFamily: "inherit" }}>
            <span style={{ width: 38, height: 38, borderRadius: 10, flexShrink: 0, background: "var(--paper-3)", display: "flex", alignItems: "center", justifyContent: "center" }}>
              <span className="kanji" style={{ fontSize: 18, color: it.tone }}>{it.k}</span>
            </span>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
                <span style={{ fontSize: 10, letterSpacing: ".1em", textTransform: "uppercase", color: it.tone, fontWeight: 700 }}>{it.kind === "stall" ? "stalled" : it.kind}</span>
                <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{it.w}</span>
              </div>
              <div style={{ fontSize: 14, color: "var(--ink)", marginTop: 2 }}>{it.title}</div>
              <div className="mono" style={{ fontSize: 10.5, color: "var(--ink-3)", marginTop: 2 }}>{it.proj} · {it.dojo}</div>
            </div>
            <span style={{ fontSize: 14, color: "var(--ink-4)" }}>→</span>
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
    <p style={{ fontSize: 12.5, color: "var(--ink-3)", lineHeight: 1.5, margin: "0 0 14px" }}>
      Answer when you can — other tracks keep moving. Pushed live through your Dōjō.
    </p>
  );
  if (!wide) return <div style={{ height: "100%", overflow: "auto", padding: 16 }}>{intro}<RelayInboxList sel={sel} setSel={setSel} /></div>;
  return (
    <div style={{ display: "flex", height: "100%", minHeight: 0 }}>
      <div style={{ width: 380, flexShrink: 0, borderRight: "var(--hairline)", overflow: "auto", padding: 22 }}>
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
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "28px" : "18px 16px" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 9, marginBottom: 14 }}>
        <span className="kanji" style={{ fontSize: 20, color: "var(--accent)" }}>認</span>
        <div>
          <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>sensei wants to run</div>
          <div style={{ fontSize: 11.5, color: "var(--ink-3)", marginTop: 2 }}>lumen-auth · Acme Corp · paused, waiting on you</div>
        </div>
      </div>
      <div style={{ background: "var(--paper-3)", border: "var(--hairline)", borderRadius: 11, padding: "13px 14px" }}>
        <div style={{ fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600, marginBottom: 7 }}>The command</div>
        <div className="mono" style={{ fontSize: 12.5, color: "var(--ink)", lineHeight: 1.6, wordBreak: "break-all" }}>psql $DB &lt; migrations/003_refresh_tokens.sql</div>
      </div>
      <div style={{ display: "flex", gap: 8, margin: "10px 0 18px", flexWrap: "wrap" }}>
        <DojoChip tone="var(--warning)" soft="var(--warning-soft)">writes to prod</DojoChip>
        <DojoChip tone="var(--ink-2)">creates 2 tables</DojoChip>
        <DojoChip tone="var(--ink-2)">reversible · down migration</DojoChip>
      </div>
      <div style={{ marginBottom: 8, fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Why</div>
      <p style={{ fontSize: 13.5, color: "var(--ink-2)", lineHeight: 1.6, margin: "0 0 18px" }}>
        The refresh-token store needs its tables before the new session flow can be tested. Adds <span className="mono" style={{ fontSize: 12 }}>refresh_tokens</span> and <span className="mono" style={{ fontSize: 12 }}>token_families</span>, with a matching down migration.
      </p>
      <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 11, padding: "12px 14px", marginBottom: 18 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, color: "var(--ink-3)" }}>
          <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>透</span>
          Reached you through your Dōjō — the daemon is holding the session open in real time.
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: wide ? "row" : "column", gap: 9 }}>
        <button style={{ flex: 1, padding: "13px", borderRadius: 11, border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: 14, fontWeight: 600, fontFamily: "inherit", display: "flex", alignItems: "center", justifyContent: "center", gap: 8 }}>
          <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>許</span> Approve &amp; continue
        </button>
        <button style={{ flex: wide ? "0 0 auto" : 1, padding: "13px 18px", borderRadius: 11, border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-2)", fontSize: 13.5, fontFamily: "inherit" }}>Deny</button>
        <button style={{ flex: wide ? "0 0 auto" : 1, padding: "13px 18px", borderRadius: 11, border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-2)", fontSize: 13.5, fontFamily: "inherit" }}>Ask a question</button>
      </div>
    </div>
  );
}

/* ═══ 4 · DECISION body ══════════════════════════════════════ */
function RelayDecisionBody({ wide = false }) {
  const [choice, setChoice] = mbS(2);
  const opts = [["Stateless JWT", "simplest"], ["Server sessions", "revocable"], ["Hybrid — JWT + refresh store", "recommended"]];
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "28px" : "18px 16px" }}>
      <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 8 }}>sensei is asking · initech-portal</div>
      <div style={{ fontSize: 18, color: "var(--ink)", fontWeight: 600, lineHeight: 1.35, marginBottom: 18 }}>Which session strategy should sensei use?</div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {opts.map((o, i) => {
          const s = i === choice;
          return (
            <button key={i} onClick={() => setChoice(i)} style={{ display: "flex", alignItems: "center", gap: 12, width: "100%", textAlign: "left", cursor: "pointer",
                  background: s ? "var(--accent-soft)" : "var(--paper-2)", fontFamily: "inherit",
                  border: s ? "1px solid color-mix(in oklch, var(--accent) 45%, transparent)" : "var(--hairline)", borderRadius: 13, padding: "14px 15px" }}>
              <span style={{ width: 20, height: 20, borderRadius: "50%", flexShrink: 0, border: s ? "none" : "2px solid var(--edge)", background: s ? "var(--accent)" : "transparent", display: "flex", alignItems: "center", justifyContent: "center" }}>
                {s && <span style={{ color: "var(--paper)", fontSize: 11 }}>✓</span>}
              </span>
              <span style={{ fontSize: 14.5, color: "var(--ink)" }}><span style={{ fontWeight: s ? 600 : 500 }}>{o[0]}</span><span style={{ color: "var(--ink-3)", fontWeight: 400 }}> · {o[1]}</span></span>
            </button>
          );
        })}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 10, margin: "16px 2px" }}>
        <span style={{ flex: 1, height: 1, background: "var(--edge)" }} /><span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>OR</span><span style={{ flex: 1, height: 1, background: "var(--edge)" }} />
      </div>
      <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: 13, padding: "13px 14px", fontSize: 13.5, color: "var(--ink-3)", minHeight: 46 }}>Type your own answer…</div>
      <button style={{ width: "100%", marginTop: 16, padding: "14px", borderRadius: 11, border: "none", cursor: "pointer", background: "var(--accent)", color: "#fff", fontSize: 15, fontWeight: 600, fontFamily: "inherit" }}>Send answer</button>
    </div>
  );
}

/* ═══ 5 · STALL / nudge body ═════════════════════════════════ */
function RelayStallBody({ wide = false }) {
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "28px" : "18px 16px" }}>
      <div style={{ background: "var(--warning-soft)", border: "1px solid color-mix(in oklch, var(--warning) 32%, transparent)", borderRadius: 13, padding: "16px" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span className="kanji" style={{ fontSize: 20, color: "var(--warning)" }}>静</span>
          <div><div style={{ fontSize: 15, fontWeight: 600, color: "var(--ink)" }}>This track has gone quiet</div>
            <div style={{ fontSize: 12, color: "var(--ink-2)", marginTop: 2 }}>telemetry · Personal · no activity 22m · waiting on API rate limit</div></div>
        </div>
        <div style={{ fontSize: 12.5, color: "var(--ink-3)", marginTop: 12 }}>sensei will retry on its own in ~8m. You can nudge it now.</div>
      </div>
      <div style={{ display: "flex", flexDirection: wide ? "row" : "column", gap: 9, marginTop: 16 }}>
        <button style={{ flex: 1, padding: "13px", borderRadius: 11, border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: 14, fontWeight: 600, fontFamily: "inherit" }}>Nudge to continue</button>
        <button style={{ flex: 1, padding: "13px", borderRadius: 11, border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-2)", fontSize: 13.5, fontFamily: "inherit" }}>Pause track</button>
        <button style={{ flex: wide ? "0 0 auto" : 1, padding: "13px 18px", borderRadius: 11, border: "var(--hairline)", cursor: "pointer", background: "var(--paper)", color: "var(--ink-2)", fontSize: 13.5, fontFamily: "inherit" }}>View logs</button>
      </div>
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
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {phases.map(ph => (
        <div key={ph.n} style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 13, overflow: "hidden" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "12px 14px", borderBottom: ph.items.length ? "var(--hairline)" : "none" }}>
            <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{ph.n}</span>
            <span style={{ flex: 1, fontSize: 14, color: ph.state === "next" ? "var(--ink-2)" : "var(--ink)", fontWeight: 600 }}>{ph.name}</span>
            <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 10.5, color: ph.state === "doing" ? "var(--success)" : "var(--ink-3)" }}>
              <span style={{ width: 6, height: 6, borderRadius: "50%", background: ph.state === "doing" ? "var(--success)" : ph.state === "done" ? "var(--ink-3)" : "var(--edge)" }} />
              {ph.state === "doing" ? "in progress" : ph.state}
            </span>
          </div>
          {ph.items.map((it, i) => {
            const done = it.kind === "done", gate = it.kind === "gate", doing = it.kind === "doing";
            return (
              <div key={i} style={{ display: "flex", alignItems: "flex-start", gap: 11, padding: "10px 14px", borderBottom: i < ph.items.length - 1 ? "1px solid var(--edge)" : "none" }}>
                <span style={{ width: 18, height: 18, marginTop: 1, borderRadius: gate ? 5 : "50%", flexShrink: 0,
                      background: done ? "var(--ink)" : gate ? "var(--accent-soft)" : "transparent",
                      border: done ? "none" : `2px solid ${gate ? "var(--accent)" : doing ? "var(--success)" : "var(--edge)"}`,
                      display: "flex", alignItems: "center", justifyContent: "center" }}>
                  {done && <span style={{ color: "var(--paper)", fontSize: 10 }}>✓</span>}
                  {gate && <span className="kanji" style={{ fontSize: 9, color: "var(--accent)" }}>鍵</span>}
                  {doing && <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--success)" }} />}
                </span>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: 13.5, color: done ? "var(--ink-3)" : "var(--ink)", fontWeight: gate || doing ? 600 : 400 }}>{it.label}</div>
                  {it.note && <div style={{ fontSize: 11.5, color: gate ? "var(--accent)" : doing ? "var(--success)" : "var(--ink-3)", marginTop: 2 }}>{it.note}</div>}
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
    { k: "書", t: "Wrote store.ts · 42 lines", w: "14:08", tone: "var(--ink-3)" },
    { k: "試", t: "12 tests pass · 0 fail", w: "14:05", tone: "var(--success)" },
    { k: "認", t: "Paused for approval · prod migration", w: "14:04", tone: "var(--accent)" },
  ];
  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
        <span className="kanji" style={{ fontSize: 13, color: "var(--ink-3)" }}>刻</span>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Activity</span>
        <span style={{ flex: 1 }} /><span className="mono" style={{ fontSize: 10.5, color: "var(--accent)" }}>full log →</span>
      </div>
      <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 13, overflow: "hidden" }}>
        {feed.map((e, i) => (
          <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 11, alignItems: "center", padding: "11px 14px", borderBottom: i < feed.length - 1 ? "1px solid var(--edge)" : "none" }}>
            <span className="kanji" style={{ fontSize: 14, color: e.tone, width: 16, textAlign: "center" }}>{e.k}</span>
            <span style={{ fontSize: 12.5, color: "var(--ink-2)" }}>{e.t}</span>
            <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)" }}>{e.w}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
function RelayWatchBody({ wide = false }) {
  return (
    <div style={{ height: "100%", overflow: "auto", padding: wide ? 28 : 16 }}>
      <div style={{ fontFamily: "var(--font-display, serif)", fontSize: wide ? 24 : 19, color: "var(--ink)", fontWeight: 400, letterSpacing: "-0.01em" }}>OAuth &amp; session hardening</div>
      <div className="mono" style={{ fontSize: 11.5, color: "var(--ink-3)", marginTop: 3 }}>lumen-auth · Acme Corp · auto</div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", margin: "14px 0 6px", maxWidth: 520 }}>
        <span className="mono" style={{ fontSize: 11, color: "var(--ink-2)" }}>Phase 2 of 4</span>
        <span className="mono" style={{ fontSize: 11, color: "var(--ink-2)" }}>47%</span>
      </div>
      <div style={{ maxWidth: 520 }}><Bar pct={47} /></div>
      <div style={{ display: "grid", gridTemplateColumns: wide ? "1.3fr 1fr" : "1fr", gap: wide ? 22 : 20, marginTop: 20, alignItems: "start" }}>
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
      <div style={{ flex: 1, overflow: "auto", padding: wide ? "24px" : "16px", display: "flex", flexDirection: "column", gap: 12 }}>
        <div style={{ width: "100%", maxWidth: wide ? 720 : "none", margin: wide ? "0 auto" : 0, display: "flex", flexDirection: "column", gap: 12 }}>
          {thread.map((m, i) => {
            const me = m.who === "you";
            return (
              <div key={i} style={{ display: "flex", flexDirection: "column", alignItems: me ? "flex-end" : "flex-start", gap: 3 }}>
                {!me && <span className="kanji" style={{ fontSize: 11, color: "var(--accent)", marginLeft: 4 }}>先生</span>}
                <div style={{ maxWidth: "82%", padding: "10px 13px", borderRadius: 15, background: me ? "var(--ink)" : "var(--paper-2)", color: me ? "var(--paper)" : "var(--ink)",
                      border: me ? "none" : "var(--hairline)", borderBottomRightRadius: me ? 5 : 15, borderBottomLeftRadius: me ? 15 : 5, fontSize: 13.5, lineHeight: 1.5 }}>{m.t}</div>
              </div>
            );
          })}
        </div>
      </div>
      <div style={{ flexShrink: 0, borderTop: "var(--hairline)", padding: wide ? "12px 24px" : "12px 16px" }}>
        <div style={{ maxWidth: wide ? 720 : "none", margin: wide ? "0 auto" : 0, display: "flex", gap: 9, alignItems: "center" }}>
          <div style={{ flex: 1, background: "var(--paper)", border: "var(--hairline)", borderRadius: 22, padding: "11px 15px", fontSize: 13.5, color: "var(--ink-3)" }}>Message sensei…</div>
          <button style={{ width: 42, height: 42, borderRadius: "50%", border: "none", cursor: "pointer", background: "var(--ink)", color: "var(--paper)", fontSize: 15, flexShrink: 0 }}>↑</button>
        </div>
      </div>
    </div>
  );
}

/* ═══ DESKTOP dispatcher — headered wide area ════════════════ */
const RELAY_META = {
  projects: { kanji: "場", eyebrow: "Relay · you", title: "Active projects", sub: "Everything running for you, across every Dōjō. Approvals, decisions and stalled tracks rise to the top — pushed live through your Dōjō." },
  inbox:    { kanji: "決", eyebrow: "Relay · you", title: "Inbox", sub: "Decisions, approvals and nudges waiting on you — across every Dōjō. Answer when you can; other tracks keep moving." },
  watch:    { kanji: "観", eyebrow: "Relay · you", title: "Watch progress", sub: "Phases, what's done · doing · next, and the live activity feed for a running track." },
  chat:     { kanji: "話", eyebrow: "Relay · you", title: "Chat with sensei", sub: "Talk to a running session mid-flight — ask a question, steer a decision, or give the go-ahead." },
};
function RelayArea({ view = "projects", wide = true, onOpen }) {
  const m = RELAY_META[view] || RELAY_META.projects;
  const Head = window.DojoHead;
  const body = view === "inbox" ? <RelayInboxBody wide={wide} />
    : view === "watch" ? <RelayWatchBody wide={wide} />
    : view === "chat" ? <RelayChatBody wide={wide} />
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
      border: "1px solid var(--edge)", boxShadow: "var(--shadow-lg, 0 20px 60px rgba(0,0,0,.18))", overflow: "hidden", position: "relative", display: "flex", flexDirection: "column" }}>
      <div style={{ flexShrink: 0, height: 44, display: "flex", alignItems: "center", justifyContent: "space-between", padding: "0 26px", fontSize: 13, color: "var(--ink)", fontWeight: 600 }}>
        <span className="mono">9:41</span><span style={{ display: "inline-flex", gap: 5, alignItems: "center", opacity: 0.8 }}><span style={{ fontSize: 11 }}>●●●</span><span style={{ fontSize: 11 }}>◗</span></span>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>{children}</div>
      <div style={{ flexShrink: 0, height: 22, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <span style={{ width: 128, height: 5, borderRadius: 3, background: "var(--ink-4)", opacity: 0.5 }} />
      </div>
    </div>
  );
}
function MHead({ title, sub, back, right, live }) {
  return (
    <div style={{ flexShrink: 0, padding: "6px 18px 12px", borderBottom: "var(--hairline)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        {back && <span className="mono" style={{ fontSize: 13, color: "var(--accent)" }}>←</span>}
        <span className="kanji" style={{ fontSize: 19, color: "var(--accent)", lineHeight: 1 }}>結</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 15, color: "var(--ink)", fontWeight: 600, lineHeight: 1.1 }}>{title}</div>
          {sub && <div className="mono" style={{ fontSize: 10.5, color: "var(--ink-4)", marginTop: 1 }}>{sub}</div>}
        </div>
        {live && <Live />}{right}
      </div>
    </div>
  );
}
const MOBILE_TABS = [
  { id: "projects", kanji: "場", label: "Projects" },
  { id: "inbox", kanji: "決", label: "Inbox", badge: 3 },
  { id: "chat", kanji: "話", label: "Chat" },
  { id: "more", kanji: "⋯", label: "More" },
];
function MTabs({ active }) {
  return (
    <div style={{ flexShrink: 0, display: "grid", gridTemplateColumns: "repeat(4,1fr)", borderTop: "var(--hairline)", background: "var(--paper)" }}>
      {MOBILE_TABS.map(t => {
        const on = active === t.id;
        return (
          <div key={t.id} style={{ position: "relative", display: "flex", flexDirection: "column", alignItems: "center", gap: 3, padding: "8px 4px 6px", color: on ? "var(--ink)" : "var(--ink-3)" }}>
            <span className="kanji" style={{ fontSize: 17, color: on ? "var(--accent)" : "var(--ink-3)" }}>{t.kanji}</span>
            <span style={{ fontSize: 10, fontWeight: on ? 600 : 400 }}>{t.label}</span>
            {t.badge != null && <span className="mono" style={{ position: "absolute", top: 4, left: "calc(50% + 6px)", fontSize: 9, fontWeight: 600, color: "var(--paper)", background: "var(--accent)", borderRadius: 10, padding: "0 5px", lineHeight: "14px" }}>{t.badge}</span>}
          </div>
        );
      })}
    </div>
  );
}

function DojoMobileProjects() {
  return <MobileFrame label="Mobile · Projects (home)"><MHead title="Projects" sub="everything running for you · 3 Dōjōs" live /><div style={{ flex: 1, minHeight: 0 }}><RelayProjectsBody wide={false} /></div><MTabs active="projects" /></MobileFrame>;
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
      <div style={{ flex: 1, overflow: "auto", padding: "16px 16px" }}>
        <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 10 }}>Consoles · this Dōjō</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 22 }}>
          {consoles.map((c, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 12, background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "13px 15px", opacity: c.role ? 1 : 0.55 }}>
              <span className="kanji" style={{ fontSize: 20, color: "var(--accent)", width: 24, textAlign: "center" }}>{c.k}</span>
              <div style={{ flex: 1, minWidth: 0 }}><div style={{ fontSize: 14, color: "var(--ink)" }}>{c.name} console</div><div style={{ fontSize: 11, color: "var(--ink-3)", marginTop: 1 }}>{c.sub}</div></div>
              {c.role ? <span style={{ fontSize: 14, color: "var(--ink-4)" }}>→</span> : <span className="mono" style={{ fontSize: 9.5, color: "var(--ink-4)" }}>no access</span>}
            </div>
          ))}
        </div>
        <div style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600, marginBottom: 10 }}>Switch Dōjō</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {orgs.map(([k, n, r], i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 12, background: i === 0 ? "var(--paper-2)" : "transparent", border: i === 0 ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: 12, padding: "12px 15px" }}>
              <span className="kanji" style={{ fontSize: 18, color: "var(--accent)", width: 22, textAlign: "center" }}>{k}</span>
              <span style={{ flex: 1, fontSize: 14, color: "var(--ink)" }}>{n}</span>
              <span className="mono" style={{ fontSize: 10.5, color: "var(--ink-3)" }}>{r}</span>
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
      <div style={{ flex: 1, overflow: "auto", padding: "22px 18px" }}>
        <p style={{ fontSize: 14, color: "var(--ink-2)", lineHeight: 1.6, margin: "0 0 24px" }}>No device pairing, no separate install. The daemon keeps a live line to the Dōjō; your phone just opens the Dōjō and picks up the session in real time.</p>
        <div style={{ display: "flex", flexDirection: "column" }}>
          {steps.map((st, i) => (
            <React.Fragment key={i}>
              <div style={{ display: "flex", alignItems: "center", gap: 14, background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 13, padding: "15px 16px" }}>
                <span className="kanji" style={{ fontSize: 26, color: "var(--accent)", width: 32, textAlign: "center" }}>{st.k}</span>
                <div style={{ flex: 1 }}><div style={{ fontSize: 15, color: "var(--ink)", fontWeight: 600 }}>{st.t}</div><div className="mono" style={{ fontSize: 11, color: "var(--ink-3)", marginTop: 2 }}>{st.s}</div><div style={{ fontSize: 12, color: "var(--ink-2)", marginTop: 4 }}>{st.note}</div></div>
              </div>
              {i < steps.length - 1 && <div style={{ display: "flex", flexDirection: "column", alignItems: "center", padding: "6px 0" }}><span style={{ fontSize: 18, color: "var(--success)" }}>↕</span><span className="mono" style={{ fontSize: 9.5, color: "var(--success)" }}>live · realtime</span></div>}
            </React.Fragment>
          ))}
        </div>
        <div style={{ display: "flex", alignItems: "flex-start", gap: 9, marginTop: 22, background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "13px 15px" }}>
          <span className="kanji" style={{ fontSize: 15, color: "var(--ink-3)" }}>携</span>
          <span style={{ fontSize: 12, color: "var(--ink-3)", lineHeight: 1.55 }}>The native Relay app still works too — it stays for push notifications and offline. This browser experience mirrors it.</span>
        </div>
      </div>
    </MobileFrame>
  );
}

Object.assign(window, {
  RelayArea, RELAY_META,
  RelayProjectsBody, RelayInboxBody, RelayWatchBody, RelayChatBody, RelayApproveBody, RelayDecisionBody, RelayStallBody,
  MobileFrame, DojoMobileProjects, DojoMobileProject, DojoMobileApprove,
  DojoMobileDecision, DojoMobileInbox, DojoMobileChat, DojoMobileMore, DojoMobileConnect,
});
