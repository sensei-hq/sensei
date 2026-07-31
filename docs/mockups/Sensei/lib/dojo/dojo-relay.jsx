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
    <div className="rounded-sm bg-paper-mute overflow-hidden" style={{ height: 6 }}>
      <div className="h-full rounded-sm" style={{ width: pct + "%", background: tone || "var(--ink)" }} />
    </div>
  );
}

/* ═══ 1 · PROJECTS body (adaptive) ═══════════════════════════ */
function RelayProjectsBody({ wide = false, onOpen }) {
  const needs = MB_PROJECTS.filter(p => ["gate", "approve", "stall"].includes(p.flag));
  const rest = MB_PROJECTS.filter(p => !["gate", "approve", "stall"].includes(p.flag));
  const ordered = [...needs, ...rest];
  const pad = wide ? "var(--space-6)" : "var(--space-4)";
  return (
    <div className="h-full overflow-auto" style={{ padding: pad }}>
      <div className="flex items-center gap-2 mb-3" >
        <span className="kanji text-sm text-ink-mute" >場</span>
        <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Active projects</span>
        <span className="mono text-xs text-ink-faint" >{MB_PROJECTS.length}</span>
        <span className="flex-1" />
        {needs.length > 0 && <span className="mono text-xs text-accent" >{needs.length} need you</span>}
      </div>
      <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
        {ordered.map((p, i) => {
          const f = flagMeta[p.flag];
          const acts = p.flag === "approve" || p.flag === "gate" || p.flag === "stall";
          return (
            <div className="grid gap-3 items-center pr-4" key={p.id} style={{
 gridTemplateColumns: wide ? "3px auto 1fr 150px auto" : "3px auto 1fr auto",
 borderTop: i === 0 ? "none" : "1px solid var(--paper-edge)" }}>
              <span className="self-stretch" style={{ background: acts ? f.tone : "transparent" }} />
              <span className="kanji text-lg" style={{ color: f.tone, lineHeight: 1, padding: "var(--space-3) 0 var(--space-3) var(--space-3)" }}>{p.kanji}</span>
              <div className="min-w-0 py-3 px-0" >
                <div className="flex items-center gap-2 flex-wrap" >
                  <span className="mono text-sm text-ink" >{p.name}</span>
                  <DojoTag p={p} />
                </div>
                <div className="text-xs text-ink-mute mt-1" >
                  <span className="font-semibold" style={{ color: f.tone }}>{f.label}</span> · {p.flag === "stall" ? "Paused" : "Now"}: {p.now}
                </div>
              </div>
              {wide && (
                <div>
                  <Bar pct={p.pct} tone={p.flag === "stall" ? "var(--warning)" : f.tone} />
                  <div className="mono text-xs text-ink-faint mt-1" >phase {p.phase}/{p.of} · {p.pct}%</div>
                </div>
              )}
              {acts
                ? <button className="cursor-pointer inline-flex items-center gap-1 whitespace-nowrap border-0 rounded py-1 px-3 text-xs font-medium" onClick={() => onOpen && onOpen(p.flag)} style={{ justifySelf: "end", fontFamily: "inherit",
 background: p.flag === "stall" ? "var(--paper)" : "var(--ink)", color: p.flag === "stall" ? "var(--ink)" : "var(--paper)",
 boxShadow: p.flag === "stall" ? "inset 0 0 0 1px var(--paper-edge)" : "none" }}>
                    <span className="kanji text-accent" >{f.ck}</span>{f.cta}
                  </button>
                : <button className="cursor-pointer border-0 text-ink-faint text-sm" onClick={() => onOpen && onOpen(p.flag)} title="Watch progress" style={{ justifySelf: "end", fontFamily: "inherit", background: "none" }}>→</button>}
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
    <div className="flex flex-col gap-2" >
      {INBOX.map(it => {
        const on = sel === it.id;
        return (
          <button className="flex items-center gap-3 w-full text-left cursor-pointer rounded-lg py-3 px-4" key={it.id} onClick={() => setSel(it.id)} style={{
 background: on ? "var(--paper-mute)" : "var(--paper-soft)", border: on ? "1px solid var(--accent)" : "var(--hairline)", fontFamily: "inherit" }}>
            <span className="rounded-lg shrink-0 bg-paper-mute flex items-center justify-center" style={{ width: 38, height: 38 }}>
              <span className="kanji text-lg" style={{ color: it.tone }}>{it.k}</span>
            </span>
            <div className="flex-1 min-w-0" >
              <div className="flex items-center gap-2" >
                <span className="text-xs uppercase" style={{ letterSpacing: ".1em", color: it.tone, fontWeight: 700 }}>{it.kind === "stall" ? "stalled" : it.kind}</span>
                <span className="mono text-xs text-ink-faint" >{it.w}</span>
              </div>
              <div className="text-sm text-ink mt-1" >{it.title}</div>
              <div className="mono text-xs text-ink-mute mt-1" >{it.proj} · {it.dojo}</div>
            </div>
            <span className="text-sm text-ink-faint" >→</span>
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
    <p className="text-sm text-ink-mute" style={{ lineHeight: 1.5, margin: "0 0 var(--space-3)" }}>
      Answer when you can — other tracks keep moving. Pushed live through your Dōjō.
    </p>
  );
  if (!wide) return <div className="h-full overflow-auto p-4" >{intro}<RelayInboxList sel={sel} setSel={setSel} /></div>;
  return (
    <div className="flex h-full min-h-0" >
      <div className="shrink-0 border-r overflow-auto p-6" style={{ width: 380 }}>
        {intro}<RelayInboxList sel={sel} setSel={setSel} />
      </div>
      <div className="flex-1 min-w-0 overflow-auto" >
        {item.kind === "approve" ? <RelayApproveBody wide /> : item.kind === "decision" ? <RelayDecisionBody wide /> : <RelayStallBody />}
      </div>
    </div>
  );
}

/* ═══ 3 · APPROVE body ═══════════════════════════════════════ */
function RelayApproveBody({ wide = false }) {
  const [done, setDone] = mbS(false);
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "var(--space-6)" : "var(--space-4) var(--space-4)" }}>
      <div className="flex items-center gap-2 mb-3" >
        <span className="kanji text-xl text-accent" >認</span>
        <div>
          <div className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>sensei wants to run</div>
          <div className="text-xs text-ink-mute mt-1" >lumen-auth · Acme Corp · paused, waiting on you</div>
        </div>
      </div>
      <div className="bg-paper-mute border border-paper-edge rounded-lg py-3 px-3" >
        <div className="text-xs uppercase text-ink-faint font-semibold mb-2" style={{ letterSpacing: ".12em" }}>The command</div>
        <div className="mono text-sm text-ink" style={{ lineHeight: 1.6, wordBreak: "break-all" }}>psql $DB &lt; migrations/003_refresh_tokens.sql</div>
      </div>
      <div className="flex gap-2 flex-wrap" style={{ margin: "var(--space-2) 0 var(--space-4)" }}>
        <DojoChip tone="var(--warning)" soft="var(--warning-soft)">writes to prod</DojoChip>
        <DojoChip tone="var(--ink-soft)">creates 2 tables</DojoChip>
        <DojoChip tone="var(--ink-soft)">reversible · down migration</DojoChip>
      </div>
      <div className="mb-2 text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".12em" }}>Why</div>
      <p className="text-sm text-ink-soft" style={{ lineHeight: 1.6, margin: "0 0 var(--space-4)" }}>
        The refresh-token store needs its tables before the new session flow can be tested. Adds <span className="mono text-xs" >refresh_tokens</span> and <span className="mono text-xs" >token_families</span>, with a matching down migration.
      </p>
      <div className="bg-paper-soft border border-paper-edge rounded-lg py-3 px-3 mb-4" >
        <div className="flex items-center gap-2 text-xs text-ink-mute" >
          <span className="kanji text-sm text-accent" >透</span>
          Reached you through your Dōjō — the daemon is holding the session open in real time.
        </div>
      </div>
      {done ? (
        <div className="flex items-center gap-3 bg-success-soft rounded-lg py-3 px-4" style={{ border: "1px solid var(--success-edge)" }}>
          <span className="kanji text-lg text-success" >済</span>
          <div><div className="text-sm text-ink font-semibold" >Approved · session resumed</div>
            <div className="text-xs text-ink-mute mt-1" >sensei is running the migration now — you'll be pinged if anything else needs you.</div></div>
        </div>
      ) : (
      <div className="flex gap-2" style={{ flexDirection: wide ? "row" : "column" }}>
        <button className="flex-1 p-3 rounded-lg border-0 cursor-pointer bg-ink text-paper text-sm font-semibold flex items-center justify-center gap-2" onClick={() => setDone(true)} style={{ fontFamily: "inherit" }}>
          <span className="kanji text-sm text-accent" >許</span> Approve &amp; continue
        </button>
        <button className="py-3 px-4 rounded-lg border border-paper-edge cursor-pointer bg-paper text-ink-soft text-sm" style={{ flex: wide ? "0 0 auto" : 1, fontFamily: "inherit" }}>Deny</button>
        <button className="py-3 px-4 rounded-lg border border-paper-edge cursor-pointer bg-paper text-ink-soft text-sm" style={{ flex: wide ? "0 0 auto" : 1, fontFamily: "inherit" }}>Ask a question</button>
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
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "var(--space-6)" : "var(--space-4) var(--space-4)" }}>
      <div className="text-xs uppercase text-ink-mute font-semibold mb-2" style={{ letterSpacing: ".14em" }}>sensei is asking · initech-portal</div>
      <div className="text-lg text-ink font-semibold mb-4" style={{ lineHeight: 1.35 }}>Which session strategy should sensei use?</div>
      <div className="flex flex-col gap-2" >
        {opts.map((o, i) => {
          const s = i === choice;
          return (
            <button className="flex items-center gap-3 w-full text-left cursor-pointer rounded-lg py-3 px-4" key={i} onClick={() => setChoice(i)} style={{
 background: s ? "var(--accent-soft)" : "var(--paper-soft)", fontFamily: "inherit",
 border: s ? "1px solid var(--accent-edge)" : "var(--hairline)" }}>
              <span className="rounded-full shrink-0 flex items-center justify-center" style={{ width: 20, height: 20, border: s ? "none" : "2px solid var(--paper-edge)", background: s ? "var(--accent)" : "transparent" }}>
                {s && <span className="text-paper text-xs" >✓</span>}
              </span>
              <span className="text-base text-ink" ><span style={{ fontWeight: s ? 600 : 500 }}>{o[0]}</span><span className="text-ink-mute font-normal" > · {o[1]}</span></span>
            </button>
          );
        })}
      </div>
      <div className="flex items-center gap-2 my-4 mx-1" >
        <span className="flex-1 bg-paper-edge" style={{ height: 1 }} /><span className="mono text-xs text-ink-faint" >OR</span><span className="flex-1 bg-paper-edge" style={{ height: 1 }} />
      </div>
      <div className="bg-paper border border-paper-edge rounded-lg py-3 px-3 text-sm text-ink-mute" style={{ minHeight: 46 }}>Type your own answer…</div>
      <div className="flex items-center gap-2 mt-3 text-xs text-ink-mute" >
        <span className="kanji text-xs text-accent" >透</span> Reached you through your Dōjō · other tracks keep moving
      </div>
      {sent ? (
        <div className="flex items-center gap-3 mt-4 bg-success-soft rounded-lg py-3 px-4" style={{ border: "1px solid var(--success-edge)" }}>
          <span className="kanji text-base text-success" >済</span>
          <div className="text-sm text-ink" ><b className="font-semibold" >Answer sent</b> — sensei is continuing the track.</div>
        </div>
      ) : (
        <button className="w-full mt-4 p-3 rounded-lg border-0 cursor-pointer bg-accent text-paper text-base font-semibold" onClick={() => setSent(true)} style={{ fontFamily: "inherit" }}>Send answer</button>
      )}
    </div>
  );
}

/* ═══ 5 · STALL / nudge body ═════════════════════════════════ */
function RelayStallBody({ wide = false }) {
  const [nudged, setNudged] = mbS(false);
  return (
    <div style={{ maxWidth: wide ? 640 : "none", margin: wide ? "0 auto" : 0, padding: wide ? "var(--space-6)" : "var(--space-4) var(--space-4)" }}>
      <div className="bg-warning-soft rounded-lg p-4" style={{ border: "1px solid var(--warning-edge)" }}>
        <div className="flex items-center gap-2" >
          <span className="kanji text-xl text-warning" >静</span>
          <div><div className="text-base font-semibold text-ink" >This track has gone quiet</div>
            <div className="text-xs text-ink-soft mt-1" >telemetry · No Dōjō · local · no activity 22m · waiting on API rate limit</div></div>
        </div>
        <div className="text-sm text-ink-mute mt-3" >sensei will retry on its own in ~8m. You can nudge it now.</div>
      </div>
      {nudged ? (
        <div className="flex items-center gap-3 mt-4 bg-success-soft rounded-lg py-3 px-4" style={{ border: "1px solid var(--success-edge)" }}>
          <span className="kanji text-lg text-success" >済</span>
          <div><div className="text-sm text-ink font-semibold" >Nudged · track resumed</div>
            <div className="text-xs text-ink-mute mt-1" >sensei picked the work back up — you'll be pinged if it stalls again.</div></div>
        </div>
      ) : (
      <div className="flex gap-2 mt-4" style={{ flexDirection: wide ? "row" : "column" }}>
        <button className="flex-1 p-3 rounded-lg border-0 cursor-pointer bg-ink text-paper text-sm font-semibold" onClick={() => setNudged(true)} style={{ fontFamily: "inherit" }}>Nudge to continue</button>
        <button className="flex-1 p-3 rounded-lg border border-paper-edge cursor-pointer bg-paper text-ink-soft text-sm" style={{ fontFamily: "inherit" }}>Pause track</button>
        <button className="py-3 px-4 rounded-lg border border-paper-edge cursor-pointer bg-paper text-ink-soft text-sm" style={{ flex: wide ? "0 0 auto" : 1, fontFamily: "inherit" }}>View logs</button>
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
    <div className="flex flex-col gap-2" >
      {phases.map(ph => (
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" key={ph.n} >
          <div className="flex items-center gap-2 py-3 px-3" style={{ borderBottom: ph.items.length ? "var(--hairline)" : "none" }}>
            <span className="mono text-xs text-ink-faint" >{ph.n}</span>
            <span className="flex-1 text-sm font-semibold" style={{ color: ph.state === "next" ? "var(--ink-soft)" : "var(--ink)" }}>{ph.name}</span>
            <span className="inline-flex items-center gap-1 text-xs" style={{ color: ph.state === "doing" ? "var(--success)" : "var(--ink-mute)" }}>
              <span className="rounded-full" style={{ width: 6, height: 6, background: ph.state === "doing" ? "var(--success)" : ph.state === "done" ? "var(--ink-mute)" : "var(--paper-edge)" }} />
              {ph.state === "doing" ? "in progress" : ph.state}
            </span>
          </div>
          {ph.items.map((it, i) => {
            const done = it.kind === "done", gate = it.kind === "gate", doing = it.kind === "doing";
            return (
              <div className="flex items-start gap-3 py-2 px-3" key={i} style={{ borderBottom: i < ph.items.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
                <span className="mt-1 shrink-0 flex items-center justify-center" style={{ width: 18, height: 18, borderRadius: gate ? "var(--radius-sm)" : "50%",
 background: done ? "var(--ink)" : gate ? "var(--accent-soft)" : "transparent",
 border: done ? "none" : `2px solid ${gate ? "var(--accent)" : doing ? "var(--success)" : "var(--paper-edge)"}` }}>
                  {done && <span className="text-paper text-xs" >✓</span>}
                  {gate && <span className="kanji text-xs text-accent" >鍵</span>}
                  {doing && <span className="rounded-full bg-success" style={{ width: 7, height: 7 }} />}
                </span>
                <div className="flex-1" >
                  <div className="text-sm" style={{ color: done ? "var(--ink-mute)" : "var(--ink)", fontWeight: gate || doing ? 600 : 400 }}>{it.label}</div>
                  {it.note && <div className="text-xs mt-1" style={{ color: gate ? "var(--accent)" : doing ? "var(--success)" : "var(--ink-mute)" }}>{it.note}</div>}
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
      <div className="flex items-center gap-2 mb-2" >
        <span className="kanji text-sm text-ink-mute" >刻</span>
        <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Activity</span>
        <span className="flex-1" /><span className="mono text-xs text-accent" >full log →</span>
      </div>
      <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
        {feed.map((e, i) => (
          <div className="grid gap-3 items-center py-3 px-3" key={i} style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: i < feed.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
            <span className="kanji text-sm text-center" style={{ color: e.tone, width: 16 }}>{e.k}</span>
            <span className="text-sm text-ink-soft" >{e.t}</span>
            <span className="mono text-xs text-ink-faint" >{e.w}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
function RelayWatchBody({ wide = false }) {
  return (
    <div className="h-full overflow-auto" style={{ padding: wide ? "var(--space-6)" : "var(--space-4)" }}>
      <div className="text-ink font-normal" style={{ fontFamily: "var(--font-display, serif)", fontSize: wide ? "var(--text-xl)" : "var(--text-lg)", letterSpacing: "-0.01em" }}>OAuth &amp; session hardening</div>
      <div className="mono text-xs text-ink-mute mt-1" >lumen-auth · Acme Corp · auto</div>
      <div className="flex items-center justify-between" style={{ margin: "var(--space-3) 0 var(--space-1)", maxWidth: 520 }}>
        <span className="mono text-xs text-ink-soft" >Phase 2 of 4</span>
        <span className="mono text-xs text-ink-soft" >47%</span>
      </div>
      <div style={{ maxWidth: 520 }}><Bar pct={47} /></div>
      <div className="grid mt-4 items-start" style={{ gridTemplateColumns: wide ? "1.3fr 1fr" : "1fr", gap: wide ? "var(--space-6)" : "var(--space-4)" }}>
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
    <div className="h-full flex flex-col min-h-0" >
      <div className="flex-1 overflow-auto flex flex-col gap-3" style={{ padding: wide ? "var(--space-6)" : "var(--space-4)" }}>
        <div className="w-full flex flex-col gap-3" style={{ maxWidth: wide ? 720 : "none", margin: wide ? "0 auto" : 0 }}>
          {thread.map((m, i) => {
            const me = m.who === "you";
            return (
              <div className="flex flex-col gap-1" key={i} style={{ alignItems: me ? "flex-end" : "flex-start" }}>
                {!me && <span className="kanji text-xs text-accent ml-1" >先生</span>}
                <div className="py-2 px-3 rounded-lg text-sm" style={{ maxWidth: "82%", background: me ? "var(--ink)" : "var(--paper-soft)", color: me ? "var(--paper)" : "var(--ink)",
 border: me ? "none" : "var(--hairline)", borderBottomRightRadius: me ? 3 : "var(--radius-lg)", borderBottomLeftRadius: me ? "var(--radius-lg)" : 3, lineHeight: 1.5 }}>{m.t}</div>
              </div>
            );
          })}
        </div>
      </div>
      <div className="shrink-0 border-t" style={{ padding: wide ? "var(--space-3) var(--space-6)" : "var(--space-3) var(--space-4)" }}>
        <div className="flex gap-2 items-center" style={{ maxWidth: wide ? 720 : "none", margin: wide ? "0 auto" : 0 }}>
          <div className="flex-1 bg-paper border border-paper-edge rounded-full py-3 px-4 text-sm text-ink-mute" >Message sensei…</div>
          <button className="rounded-full border-0 cursor-pointer bg-ink text-paper text-base shrink-0" style={{ width: 42, height: 42 }}>↑</button>
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
    <div className="h-full overflow-auto" style={{ padding: wide ? "var(--space-6)" : "var(--space-4)" }}>
      <div className="mono text-xs text-ink-faint mb-3" >lumen-auth · Acme Corp · newest first</div>
      <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" style={{ maxWidth: wide ? 720 : "none" }}>
        {rows.map((e, i) => (
          <div className="grid gap-3 items-center py-3 px-4" key={i} style={{ gridTemplateColumns: "52px auto 1fr", borderBottom: i < rows.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
            <span className="mono text-xs text-ink-faint" >{e.t}</span>
            <span className="kanji text-base text-center" style={{ color: e.tone, width: 18 }}>{e.k}</span>
            <span className="text-sm text-ink" >{e.txt}</span>
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
    <div className="h-full flex flex-col items-center justify-center gap-3 text-center" style={{ padding: wide ? "var(--space-12)" : "var(--space-8) var(--space-6)" }}>
      <span className="kanji text-4xl" style={{ color: s.tone, lineHeight: 1 }}>{s.k}</span>
      <div className="display text-xl font-normal text-ink" style={{ letterSpacing: "-0.01em" }}>{s.title}</div>
      <div className="text-sm text-ink-soft" >{s.sub}</div>
      <p className="text-sm text-ink-mute" style={{ lineHeight: 1.6, margin: "var(--space-1) 0 var(--space-1)", maxWidth: 360 }}>{s.body}</p>
      <span className="inline-flex items-center gap-1 text-xs rounded-full py-1 px-2" style={{ color: off ? "var(--warning)" : "var(--ink-mute)",
 background: off ? "var(--warning-soft)" : "var(--paper-mute)", border: off ? "1px solid var(--warning-edge)" : "var(--hairline)" }}>
        <span className="rounded-full" style={{ width: 6, height: 6, background: off ? "var(--warning)" : "var(--ink-faint)" }} />{off ? "offline · through your Dōjō" : "session closed"}
      </span>
      <button className="mt-2 py-3 px-4 rounded-lg border-0 cursor-pointer bg-ink text-paper text-sm font-medium" style={{ fontFamily: "inherit" }}>{s.cta}</button>
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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      {Head && <Head kanji={m.kanji} eyebrow={m.eyebrow} title={m.title} sub={m.sub} right={<Live />} />}
      <div className="flex-1 min-h-0" style={{ overflow: flush ? "hidden" : "auto" }}>{body}</div>
    </div>
  );
}

/* ═══ MOBILE wrappers — same bodies, phone frame + tabs ══════ */
function MobileFrame({ children, label }) {
  return (
    <div className="sensei bg-paper shadow-lg overflow-hidden relative flex flex-col" data-screen-label={label} style={{ width: 390, height: 844, borderRadius: 44,
 border: "1px solid var(--paper-edge)" }}>
      <div className="shrink-0 flex items-center justify-between py-0 px-6 text-sm text-ink font-semibold" style={{ height: 44 }}>
        <span className="mono">9:41</span><span className="inline-flex gap-1 items-center" style={{ opacity: 0.8 }}><span className="text-xs" >●●●</span><span className="text-xs" >◗</span></span>
      </div>
      <div className="flex-1 min-h-0 flex flex-col" >{children}</div>
      <div className="shrink-0 flex items-center justify-center" style={{ height: 22 }}>
        <span className="rounded-sm bg-ink-faint" style={{ width: 128, height: 5, opacity: 0.5 }} />
      </div>
    </div>
  );
}
function MHead({ title, sub, back, right, live }) {
  return (
    <div className="shrink-0 border-b" style={{ padding: "var(--space-1) var(--space-4) var(--space-3)" }}>
      <div className="flex items-center gap-2" >
        {back && <span className="mono text-sm text-accent" >←</span>}
        <span className="kanji text-lg text-accent" style={{ lineHeight: 1 }}>結</span>
        <div className="flex-1 min-w-0" >
          <div className="text-base text-ink font-semibold" style={{ lineHeight: 1.1 }}>{title}</div>
          {sub && <div className="mono text-xs text-ink-faint mt-1" >{sub}</div>}
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
  return <MobileFrame label="Mobile · Projects (home)"><MHead title="Projects" sub="everything running for you · your Dōjōs + solo" live /><div className="flex-1 min-h-0" ><RelayProjectsBody wide={false} /></div><MTabs active="projects" /></MobileFrame>;
}
function DojoMobileInbox() {
  return <MobileFrame label="Mobile · Inbox"><MHead title="Inbox" sub="what needs you · across Dōjōs" live /><div className="flex-1 min-h-0" ><RelayInboxBody wide={false} /></div><MTabs active="inbox" /></MobileFrame>;
}
function DojoMobileProject() {
  return <MobileFrame label="Mobile · Watch progress"><MHead title="lumen-auth" sub="Acme Corp · auto" back live /><div className="flex-1 min-h-0" ><RelayWatchBody wide={false} /></div></MobileFrame>;
}
function DojoMobileApprove() {
  return <MobileFrame label="Mobile · Approve action"><MHead title="Approval" sub="lumen-auth · Acme Corp" back live /><div className="flex-1 min-h-0 overflow-auto" ><RelayApproveBody wide={false} /></div></MobileFrame>;
}
function DojoMobileDecision() {
  return <MobileFrame label="Mobile · Decision"><MHead title="Decision" sub="initech-portal · Initech" back live /><div className="flex-1 min-h-0 overflow-auto" ><RelayDecisionBody wide={false} /></div></MobileFrame>;
}
function DojoMobileChat() {
  return <MobileFrame label="Mobile · Chat with sensei"><MHead title="sensei" sub="lumen-auth · live session" back live /><div className="flex-1 min-h-0" ><RelayChatBody wide={false} /></div></MobileFrame>;
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
      <div className="flex-1 overflow-auto py-4 px-4" >
        <div className="text-xs uppercase text-ink-mute font-semibold mb-2" style={{ letterSpacing: ".14em" }}>Consoles · this Dōjō</div>
        <div className="flex flex-col gap-2 mb-6" >
          {consoles.map((c, i) => (
            <div className="flex items-center gap-3 bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" key={i} style={{ opacity: c.role ? 1 : 0.55 }}>
              <span className="kanji text-xl text-accent text-center" style={{ width: 24 }}>{c.k}</span>
              <div className="flex-1 min-w-0" ><div className="text-sm text-ink" >{c.name} console</div><div className="text-xs text-ink-mute mt-1" >{c.sub}</div></div>
              {c.role ? <span className="text-sm text-ink-faint" >→</span> : <span className="mono text-xs text-ink-faint" >no access</span>}
            </div>
          ))}
        </div>
        <div className="text-xs uppercase text-ink-mute font-semibold mb-2" style={{ letterSpacing: ".14em" }}>Switch Dōjō</div>
        <div className="flex flex-col gap-2" >
          {orgs.map(([k, n, r], i) => (
            <div className="flex items-center gap-3 rounded-lg py-3 px-4" key={i} style={{ background: i === 0 ? "var(--paper-soft)" : "transparent", border: i === 0 ? "1px solid var(--accent)" : "var(--hairline)" }}>
              <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{k}</span>
              <span className="flex-1 text-sm text-ink" >{n}</span>
              <span className="mono text-xs text-ink-mute" >{r}</span>
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
      <div className="flex-1 overflow-auto py-6 px-4" >
        <p className="text-sm text-ink-soft" style={{ lineHeight: 1.6, margin: "0 0 var(--space-6)" }}>No device pairing, no separate install. The daemon keeps a live line to the Dōjō; your phone just opens the Dōjō and picks up the session in real time.</p>
        <div className="flex flex-col" >
          {steps.map((st, i) => (
            <React.Fragment key={i}>
              <div className="flex items-center gap-3 bg-paper-soft border border-paper-edge rounded-lg py-4 px-4" >
                <span className="kanji text-2xl text-accent text-center" style={{ width: 32 }}>{st.k}</span>
                <div className="flex-1" ><div className="text-base text-ink font-semibold" >{st.t}</div><div className="mono text-xs text-ink-mute mt-1" >{st.s}</div><div className="text-xs text-ink-soft mt-1" >{st.note}</div></div>
              </div>
              {i < steps.length - 1 && <div className="flex flex-col items-center py-1 px-0" ><span className="text-lg text-success" >↕</span><span className="mono text-xs text-success" >live · realtime</span></div>}
            </React.Fragment>
          ))}
        </div>
        <div className="flex items-start gap-2 mt-6 bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" >
          <span className="kanji text-base text-ink-mute" >携</span>
          <span className="text-xs text-ink-mute" style={{ lineHeight: 1.55 }}>The native Relay app still works too — it stays for push notifications and offline. This browser experience mirrors it.</span>
        </div>
      </div>
    </MobileFrame>
  );
}

function DojoMobileLogs() {
  return <MobileFrame label="Mobile · Session log"><MHead title="Session log" sub="lumen-auth · Acme Corp" back /><div className="flex-1 min-h-0" ><RelayLogsBody wide={false} /></div></MobileFrame>;
}

function DojoMobileConn({ kind = "offline" }) {
  return <MobileFrame label={"Mobile · Relay " + kind}><MHead title="Relay" sub="lumen-auth · Acme Corp" back /><div className="flex-1 min-h-0" ><RelayConnState kind={kind} /></div></MobileFrame>;
}

function MobileMoreBody({ onOpen }) {
  const links = [["承", "Approve a gated action", "approve"], ["決", "Answer a decision", "decision"], ["録", "Session log", "logs"]];
  const orgs = [["社", "Acme Corp", "Admin"], ["客", "Globex", "Maintainer"], ["客", "Initech", "Lead"], ["己", "Personal", "Owner"]];
  return (
    <div className="flex-1 overflow-auto p-4" >
      <div className="zs-eyebrow font-semibold mb-2" >Away from keyboard</div>
      <div className="flex flex-col gap-2 mb-6" >
        {links.map(([k, n, id]) => (
          <button className="flex items-center gap-3 text-left cursor-pointer bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" key={id} onClick={() => onOpen(id)} style={{ fontFamily: "inherit" }}>
            <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{k}</span>
            <span className="flex-1 text-sm text-ink" >{n}</span>
            <span className="text-sm text-ink-faint" >→</span>
          </button>
        ))}
      </div>
      <div className="zs-eyebrow font-semibold mb-2" >Switch Dōjō</div>
      <div className="flex flex-col gap-2" >
        {orgs.map(([k, n, r], i) => (
          <div className="flex items-center gap-3 rounded-lg py-3 px-4" key={i} style={{ background: i === 0 ? "var(--paper-soft)" : "transparent", border: i === 0 ? "1px solid var(--accent)" : "var(--hairline)" }}>
            <span className="kanji text-lg text-accent text-center" style={{ width: 22 }}>{k}</span>
            <span className="flex-1 text-sm text-ink" >{n}</span>
            <span className="mono text-xs text-ink-mute" >{r}</span>
          </div>
        ))}
      </div>
      <div className="mono mt-6 text-xs text-ink-faint flex items-center gap-1" >
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
      <div className="flex-1 min-h-0 flex flex-col" >{body}</div>
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
