// Sessions — consolidated cross-project view with retrospective analysis.
//
// What this is: every session sensei has witnessed, across every project,
// with project / language / stack tags so the user can slice and find
// patterns. Plus retrospective cards that read like a project retro:
//   ▸ What's going well — high FTR, first-try-right streaks, adopted patterns
//   ▸ What's not        — recurring corrections, low-FTR projects, drift
//   ▸ Insights          — cross-project signals (e.g. "Rust auth is harder")
//
// Two layouts:
//   A · DIGEST    — Retro cards on top + filterable session list below
//   B · TIMELINE  — Calendar-style heatmap + grouped session feed
//
// Both share the same window.SESSIONS data fixture.

const { useState: ssS, useMemo: ssM } = React;

// ═══════════════════════════════════════════════════════════════════════
// Fixture: cross-project sessions with project/language/stack tags
// ═══════════════════════════════════════════════════════════════════════
window.SESSIONS = (function () {
  const proj = {
    "lumen-cloud":  { name: "lumen-cloud",  kanji: "雲", client: "Lumen", lang: "Rust",       stack: ["axum","postgres","rust"] },
    "lumen-auth":   { name: "lumen-auth",   kanji: "鍵", client: "Lumen", lang: "Rust",       stack: ["axum","oauth","rust"]    },
    "lumen-studio": { name: "lumen-studio", kanji: "工", client: "Lumen", lang: "Rust+TS",    stack: ["tauri","react","rust"]   },
    "koto-editor":  { name: "koto-editor",  kanji: "琴", client: "Koto",  lang: "TypeScript", stack: ["svelte@5","crdt","ts"]   },
    "tabi-sdk":     { name: "tabi-sdk",     kanji: "旅", client: "Tabi",  lang: "Rust",       stack: ["rust","sqlx","tonic"]    },
    "ginkgo":       { name: "ginkgo",       kanji: "銀", client: "Personal", lang: "TypeScript", stack: ["next","ts"]           }
  };

  const sessions = [
    { id: "s-2905", title: "Tighten response style — drop trailing summaries", project: "ginkgo",       when: "today",       time: "10:42", duration: "12m", corrections: 1, ftr: true,  agent: "claude-code", outcome: "shipped" },
    { id: "s-2904", title: "Wire ApiError envelope through billing handlers",  project: "lumen-cloud",  when: "today",       time: "09:12", duration: "1h 04m", corrections: 2, ftr: false, agent: "claude-code", outcome: "shipped" },
    { id: "s-2903", title: "Migrate let → $state in editor toolbar",           project: "koto-editor",  when: "today",       time: "08:30", duration: "44m", corrections: 4, ftr: false, agent: "cursor",      outcome: "shipped" },
    { id: "s-2902", title: "OAuth device-flow polling backoff",                project: "lumen-auth",   when: "yesterday",   time: "16:55", duration: "1h 22m", corrections: 3, ftr: false, agent: "claude-code", outcome: "abandoned" },
    { id: "s-2901", title: "Adapter for Stripe webhook ingest",                project: "lumen-cloud",  when: "yesterday",   time: "14:08", duration: "58m", corrections: 0, ftr: true,  agent: "claude-code", outcome: "shipped" },
    { id: "s-2899", title: "CRDT move-op ordering edge case",                  project: "koto-editor",  when: "yesterday",   time: "11:01", duration: "2h 11m", corrections: 6, ftr: false, agent: "cursor",      outcome: "shipped" },
    { id: "s-2895", title: "inFlightMutex on token refresh",                   project: "lumen-auth",   when: "2 days ago",  time: "15:20", duration: "33m", corrections: 1, ftr: true,  agent: "claude-code", outcome: "shipped" },
    { id: "s-2891", title: "Fix refresh token rotation",                       project: "lumen-auth",   when: "2 days ago",  time: "10:42", duration: "38m", corrections: 3, ftr: false, agent: "claude-code", outcome: "shipped" },
    { id: "s-2889", title: "Repo-pattern extraction in users handler",         project: "tabi-sdk",     when: "3 days ago",  time: "13:14", duration: "1h 47m", corrections: 2, ftr: true,  agent: "claude-code", outcome: "shipped" },
    { id: "s-2887", title: "Result<Json<T>, ApiError> across users routes",    project: "lumen-cloud",  when: "3 days ago",  time: "10:00", duration: "52m", corrections: 1, ftr: true,  agent: "claude-code", outcome: "shipped" },
    { id: "s-2884", title: "Tauri IPC boundary refactor",                      project: "lumen-studio", when: "4 days ago",  time: "16:18", duration: "1h 18m", corrections: 0, ftr: true,  agent: "claude-code", outcome: "shipped" },
    { id: "s-2880", title: "Next.js route handlers — server actions",          project: "ginkgo",       when: "4 days ago",  time: "09:55", duration: "26m", corrections: 0, ftr: true,  agent: "cursor",      outcome: "shipped" },
    { id: "s-2877", title: "tonic streaming endpoint — backpressure",          project: "tabi-sdk",     when: "5 days ago",  time: "15:42", duration: "2h 03m", corrections: 4, ftr: false, agent: "claude-code", outcome: "abandoned" },
    { id: "s-2872", title: "axum sub-router for /v2/ namespace",               project: "lumen-cloud",  when: "6 days ago",  time: "11:12", duration: "1h 06m", corrections: 1, ftr: true,  agent: "claude-code", outcome: "shipped" },
    { id: "s-2867", title: "CRDT undo stack reentrancy",                       project: "koto-editor",  when: "1 week ago",  time: "14:30", duration: "3h 02m", corrections: 7, ftr: false, agent: "cursor",      outcome: "shipped" }
  ];

  // Retrospective insights — what's going well / not / cross-cutting
  const retro = {
    going_well: [
      { id: "rw1", kanji: "昇", title: "Adapter pattern is paying off",
        body: "7 sessions in lumen-cloud used the new adapter abstraction — FTR averaged 0.91 vs 0.64 in handlers without it.",
        evidence: ["s-2901","s-2887","s-2872"], delta: "+27% FTR" },
      { id: "rw2", kanji: "技", title: "Response style memory holding strong",
        body: "12 sessions across 3 projects with no trailing-summary corrections. Strength-5 memory is doing its job.",
        evidence: ["s-2905","s-2880","s-2884"], delta: "0 violations · 14d" },
      { id: "rw3", kanji: "速", title: "lumen-cloud sessions getting shorter",
        body: "Median session duration dropped from 1h 18m → 52m over the last 14 days. Codebase memory is compounding.",
        evidence: ["s-2904","s-2901","s-2872"], delta: "−33% duration" }
    ],
    not_going: [
      { id: "ng1", kanji: "破", title: "Svelte 5 reactivity keeps biting koto-editor",
        body: "3 sessions this week needed `let → $state` corrections. Memory exists at strength 2 — sensei may not be surfacing it in context.",
        evidence: ["s-2903","s-2899","s-2867"], delta: "4× corrections", action: "Reinforce m-svelte5-state" },
      { id: "ng2", kanji: "迷", title: "tabi-sdk has the longest sessions and worst FTR",
        body: "Average 1h 55m and FTR 0.41. Session topics drift — backpressure, repo extraction, SQL injection scares — no clear persona yet.",
        evidence: ["s-2877","s-2889"], delta: "FTR 0.41", action: "Draft tabi-sdk persona" },
      { id: "ng3", kanji: "捨", title: "2 abandoned sessions in 7 days",
        body: "OAuth device-flow polling and tonic backpressure — both stalled after 1.5+ hours with no outcome. Worth a post-mortem.",
        evidence: ["s-2902","s-2877"], delta: "2 abandoned", action: "Open post-mortem" }
    ],
    insights: [
      { id: "in1", kanji: "観", title: "Rust + auth is your hardest combo",
        body: "Across lumen-auth and tabi-sdk, the 5 sessions touching auth in Rust averaged FTR 0.42 and 1h 25m. Other Rust work averages 0.79.",
        evidence: ["s-2902","s-2895","s-2891","s-2877"], tone: "neutral" },
      { id: "in2", kanji: "架", title: "Cross-project: adapter pattern transfers",
        body: "lumen-cloud's adapter pattern would apply to tabi-sdk's 5 inline auth call-sites. Anti-pattern detector is already flagging them.",
        evidence: ["s-2901","s-2889"], tone: "positive" },
      { id: "in3", kanji: "曜", title: "Mornings are first-try-right",
        body: "Sessions before 11am have FTR 0.81 vs 0.58 for afternoon work. Tuesdays are your strongest day.",
        evidence: ["s-2905","s-2904","s-2901","s-2895"], tone: "neutral" }
    ]
  };

  // Recommendation checkpoints — moments where a memory/skill/agent was
  // applied. Charts overlay these as vertical markers so before/after is visible.
  const checkpoints = [
    { id: "ck-adapter",  when: "5 days ago", kanji: "技",
      title: "Adopted adapter pattern",
      body: "wrote `lumen-conventions.skill.md` from m-adapter-pattern",
      affects: ["lumen-cloud","lumen-auth"] },
    { id: "ck-style",    when: "3 days ago", kanji: "則",
      title: "Promoted response-style memory to rule",
      body: "no trailing summaries · global response style",
      affects: "all" },
    { id: "ck-svelte",   when: "yesterday",  kanji: "禁",
      title: "Enabled lint check for `let → $state`",
      body: "sensei lint guards against legacy reactivity",
      affects: ["koto-editor"] }
  ];

  return { projects: proj, sessions, retro, checkpoints };
})();

// ═══════════════════════════════════════════════════════════════════════
// Shared chrome
// ═══════════════════════════════════════════════════════════════════════
function SsHero({ totals }) {
  return (
    <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center', gap: 24, background: 'var(--paper)' }}>
      <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>録</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>
          Observatory · Sessions
        </div>
        <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                          color: 'var(--ink)' }}>
          Every session sensei has witnessed.
        </h1>
        <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                     maxWidth: 720, lineHeight: 1.55 }}>
          Across every project. Tagged by language and stack. With a retrospective on what's working, what isn't, and what stands out.
        </p>
      </div>
      <div style={{ display: 'flex', gap: 24, paddingLeft: 24, borderLeft: 'var(--hairline)' }}>
        <SsStat n={totals.count} l="sessions · 7d"/>
        <SsStat n={`${Math.round(totals.ftr*100)}%`} l="ftr" mono accent={totals.ftr < 0.7}/>
        <SsStat n={totals.corrections} l="corrections"/>
        <SsStat n={totals.projects} l="projects"/>
      </div>
    </div>
  );
}
function SsStat({ n, l, mono, accent }) {
  return (
    <div style={{ textAlign: 'center' }}>
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 17, lineHeight: 1, fontWeight: 300,
                     color: accent ? 'var(--accent)' : 'var(--ink)',
                     fontFeatureSettings: '"tnum"' }}>{n}</div>
      <div style={{ fontSize: 11, letterSpacing: '0.12em', color: 'var(--ink-4)',
                     marginTop: 4, textTransform: 'uppercase' }}>{l}</div>
    </div>
  );
}

// language → soft accent (kept to existing tokens)
const LANG_TONE = {
  "Rust":       { color: 'var(--accent)' },
  "TypeScript": { color: 'var(--ai, var(--ink-2))' },
  "Rust+TS":    { color: 'var(--warning)' },
  "Python":     { color: 'var(--success)' }
};

function SessionTags({ project }) {
  const meta = window.SESSIONS.projects[project];
  if (!meta) return null;
  const lang = LANG_TONE[meta.lang] || { color: 'var(--ink-3)' };
  return (
    <div style={{ display: 'inline-flex', gap: 4, alignItems: 'center', flexWrap: 'wrap' }}>
      <span style={{ fontSize: 11, padding: '4px 8px', borderRadius: 10,
                      background: 'var(--paper)', border: 'var(--hairline)',
                      color: 'var(--ink-2)' }}>
        <span className="kanji" style={{ marginRight: 4, color: 'var(--accent)' }}>{meta.kanji}</span>
        {meta.name}
      </span>
      <span className="mono" style={{ fontSize: 11, padding: '4px 8px', borderRadius: 10,
                      background: 'var(--paper)', border: 'var(--hairline)',
                      color: lang.color }}>{meta.lang}</span>
      {meta.stack.slice(0, 2).map(s => (
        <span key={s} className="mono" style={{ fontSize: 11, padding: '4px 8px',
                      borderRadius: 10, background: 'var(--paper)', border: 'var(--hairline)',
                      color: 'var(--ink-3)' }}>{s}</span>
      ))}
    </div>
  );
}

function totalsFrom(sessions) {
  const projects = new Set(sessions.map(s => s.project)).size;
  const corrections = sessions.reduce((sum, s) => sum + s.corrections, 0);
  const ftr = sessions.length === 0 ? 0 :
    sessions.filter(s => s.ftr).length / sessions.length;
  return { count: sessions.length, ftr, corrections, projects };
}

// ═══════════════════════════════════════════════════════════════════════
// OPTION A · DIGEST — retro cards on top + filterable list below
// ═══════════════════════════════════════════════════════════════════════
function SessionsDigest() {
  const D = window.SESSIONS;
  const [project, setProject] = ssS("all");
  const [lang, setLang]       = ssS("all");
  const [outcome, setOutcome] = ssS("all"); // all | shipped | abandoned | corrected

  const filtered = D.sessions.filter(s => {
    if (project !== "all" && s.project !== project) return false;
    const meta = D.projects[s.project];
    if (lang !== "all" && meta?.lang !== lang) return false;
    if (outcome === "shipped"   && s.outcome !== "shipped")   return false;
    if (outcome === "abandoned" && s.outcome !== "abandoned") return false;
    if (outcome === "corrected" && s.corrections === 0)       return false;
    return true;
  });

  const totals = totalsFrom(D.sessions);
  const langs = ["all", ...new Set(Object.values(D.projects).map(p => p.lang))];

  return (
    <div className="sensei" data-screen-label="Observatory · Sessions · Digest"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <SsHero totals={totals}/>

      <div style={{ flex: 1, overflow: 'auto', minHeight: 0,
                     padding: '16px 32px 32px', display: 'flex',
                     flexDirection: 'column', gap: 24 }}>

        {/* Retrospective — three lanes */}
        <section>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 12,
                         paddingBottom: 8, borderBottom: 'var(--hairline)' }}>
            <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>省</span>
            <h3 className="display" style={{ fontSize: 15, fontWeight: 400, margin: 0,
                          color: 'var(--ink)' }}>Retrospective · last 7 days</h3>
            <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              · what sensei sees across your sessions
            </span>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
            <RetroLane title="Going well"   accent="var(--success)"  items={D.retro.going_well} positive/>
            <RetroLane title="Not going well" accent="var(--accent)"   items={D.retro.not_going}/>
            <RetroLane title="Insights"     accent="var(--ink-2)" items={D.retro.insights}/>
          </div>
        </section>

        {/* Filter row */}
        <section>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16, flexWrap: 'wrap',
                         marginBottom: 12 }}>
            <ProjectFilter value={project} onChange={setProject} projects={D.projects}/>
            <FilterChips label="language" value={lang} setValue={setLang} options={langs}/>
            <FilterChips label="outcome"  value={outcome} setValue={setOutcome}
                          options={["all","shipped","corrected","abandoned"]}/>
            <span style={{ flex: 1 }}/>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {filtered.length} of {D.sessions.length}
            </span>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {filtered.map(s => <SessionRow key={s.id} session={s}/>)}
            {filtered.length === 0 &&
              <div style={{ padding: 32, textAlign: 'center', fontSize: 13,
                             color: 'var(--ink-4)' }}>no sessions match.</div>}
          </div>
        </section>
      </div>
    </div>
  );
}

function RetroLane({ title, accent, items, positive }) {
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <span style={{ width: 8, height: 8, borderRadius: '50%', background: accent }}/>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase', fontWeight: 500 }}>{title}</span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {items.map(it => <RetroCard key={it.id} item={it} accent={accent} positive={positive}/>)}
      </div>
    </div>
  );
}

function RetroCard({ item, accent, positive }) {
  return (
    <article style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${accent}`, borderRadius: 6,
                       padding: '12px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 4 }}>
        <span className="kanji" style={{ fontSize: 13, color: accent }}>{item.kanji}</span>
        <div style={{ fontSize: 13, color: 'var(--ink)', fontWeight: 500,
                       lineHeight: 1.4, flex: 1 }}>{item.title}</div>
      </div>
      <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55 }}>
        {item.body}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 8,
                     paddingTop: 8, borderTop: '1px dashed var(--edge)' }}>
        {item.delta && (
          <span className="mono" style={{ fontSize: 11,
                        color: positive ? 'var(--success)' : item.delta.startsWith('−') || item.delta.startsWith('-') ? 'var(--accent)' :
                               item.tone === "positive" ? 'var(--success)' : 'var(--ink-2)' }}>
            {item.delta}
          </span>
        )}
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
          {item.evidence.length} session{item.evidence.length === 1 ? "" : "s"}
        </span>
        <span style={{ flex: 1 }}/>
        {item.action && (
          <button style={{ fontSize: 11, color: 'var(--accent)' }}>{item.action} →</button>
        )}
      </div>
    </article>
  );
}

function FilterChips({ label, value, setValue, options, render }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      <span style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                      textTransform: 'uppercase', marginRight: 4 }}>{label}</span>
      {options.map(o => (
        <button key={o} onClick={() => setValue(o)}
                style={{ padding: '4px 8px', fontSize: 11,
                          background: value === o ? 'var(--ink)' : 'transparent',
                          color: value === o ? 'var(--paper)' : 'var(--ink-2)',
                          border: value === o ? '1px solid var(--ink)' : '1px solid var(--edge)',
                          borderRadius: 20, cursor: 'pointer' }}>
          {render ? render(o) : o}
        </button>
      ))}
    </div>
  );
}

function SessionRow({ session }) {
  const meta = window.SESSIONS.projects[session.project];
  return (
    <article style={{ display: 'grid',
                       gridTemplateColumns: '8px 88px 1fr auto auto',
                       gap: 12, alignItems: 'center',
                       padding: '12px 12px',
                       background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 5 }}>
      <span style={{ width: 7, height: 7, borderRadius: '50%',
                      background: session.ftr ? 'var(--success)' :
                                  session.outcome === "abandoned" ? 'var(--ink-3)' :
                                  'var(--warning)' }}/>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
        {session.id}
      </span>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.4,
                       fontWeight: 500, marginBottom: 4 }}>
          {session.title}
        </div>
        <SessionTags project={session.project}/>
      </div>
      <div style={{ textAlign: 'right' }}>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>
          {session.duration}
        </div>
        <div className="mono" style={{ fontSize: 11, color: session.corrections === 0 ?
                      'var(--success)' : 'var(--ink-3)', marginTop: 4 }}>
          {session.corrections === 0 ? "first-try" : `${session.corrections}× corr.`}
        </div>
      </div>
      <div style={{ textAlign: 'right', minWidth: 80 }}>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {session.when}
        </div>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
          {session.time}
        </div>
      </div>
    </article>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// OPTION B · TIMELINE — heatmap + grouped feed
// ═══════════════════════════════════════════════════════════════════════
function SessionsTimeline() {
  const D = window.SESSIONS;
  const [project, setProject] = ssS("all");

  const filtered = D.sessions.filter(s => project === "all" || s.project === project);
  const totals = totalsFrom(D.sessions);

  // Group by "when" bucket
  const groups = {};
  filtered.forEach(s => {
    if (!groups[s.when]) groups[s.when] = [];
    groups[s.when].push(s);
  });
  const orderedKeys = ["today","yesterday","2 days ago","3 days ago","4 days ago","5 days ago","6 days ago","1 week ago"];
  const dayKeys = orderedKeys.filter(k => groups[k]);

  return (
    <div className="sensei" data-screen-label="Observatory · Sessions · Timeline"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <SsHero totals={totals}/>

      <div style={{ flex: 1, overflow: 'auto', minHeight: 0,
                     padding: '16px 32px 32px', display: 'flex',
                     flexDirection: 'column', gap: 24 }}>

        {/* Activity heatmap by project × day */}
        <ActivityMatrix sessions={D.sessions}/>

        {/* Compact retro strip */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
          <RetroSummary kanji="昇" tone="var(--success)" label="Going well"
                         items={D.retro.going_well}/>
          <RetroSummary kanji="破" tone="var(--accent)"  label="Not going well"
                         items={D.retro.not_going}/>
          <RetroSummary kanji="観" tone="var(--ink-2)" label="Insights"
                         items={D.retro.insights}/>
        </div>

        {/* Filter */}
        <ProjectFilter value={project} onChange={setProject} projects={D.projects}/>

        {/* Timeline */}
        <div style={{ position: 'relative', paddingLeft: 96 }}>
          <div style={{ position: 'absolute', left: 86, top: 4, bottom: 4,
                         width: 1, background: 'var(--edge)' }}/>
          {dayKeys.map(day => (
            <DayGroup key={day} when={day} sessions={groups[day]}/>
          ))}
        </div>
      </div>
    </div>
  );
}

function DayGroup({ when, sessions }) {
  const ftr = sessions.filter(s => s.ftr).length;
  return (
    <div style={{ marginBottom: 16, position: 'relative' }}>
      <div style={{ position: 'absolute', left: -90, top: 0, width: 76,
                     textAlign: 'right' }}>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>
          {when}
        </div>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
          {sessions.length}× · ftr {ftr}/{sessions.length}
        </div>
      </div>
      <span style={{ position: 'absolute', left: -8, top: 5,
                      width: 9, height: 9, borderRadius: '50%',
                      background: 'var(--paper)', border: '1.5px solid var(--accent)' }}/>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {sessions.map(s => <SessionRow key={s.id} session={s}/>)}
      </div>
    </div>
  );
}

function RetroSummary({ kanji, tone, label, items }) {
  return (
    <section style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${tone}`,
                       borderRadius: 6, padding: '12px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 8 }}>
        <span className="kanji" style={{ fontSize: 13, color: tone }}>{kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase' }}>{label}</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{items.length}</span>
      </div>
      <ul style={{ margin: 0, padding: 0, listStyle: 'none',
                    display: 'flex', flexDirection: 'column', gap: 4 }}>
        {items.map(it => (
          <li key={it.id} style={{ fontSize: 11, color: 'var(--ink)', lineHeight: 1.5,
                       paddingLeft: 12, position: 'relative' }}>
            <span style={{ position: 'absolute', left: 0, top: 7, width: 4, height: 4,
                            borderRadius: '50%', background: tone }}/>
            {it.title}
            {it.delta && (
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                            marginLeft: 4 }}>{it.delta}</span>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

// Activity matrix: each cell = one project × one day; size encodes session count, color encodes FTR
function ActivityMatrix({ sessions }) {
  const D = window.SESSIONS;
  const days = ["today","yesterday","2 days ago","3 days ago","4 days ago","5 days ago","6 days ago","1 week ago"];
  const dayShort = { "today":"Today", "yesterday":"Yest", "2 days ago":"−2", "3 days ago":"−3",
                     "4 days ago":"−4", "5 days ago":"−5", "6 days ago":"−6", "1 week ago":"−7" };
  const projects = Object.keys(D.projects);

  // Build cell map
  const cell = (proj, day) => {
    const items = sessions.filter(s => s.project === proj && s.when === day);
    if (items.length === 0) return null;
    const ftr = items.filter(s => s.ftr).length / items.length;
    return { count: items.length, ftr };
  };

  return (
    <section style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 8, padding: '16px 16px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>表</span>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase' }}>activity · 7d</span>
        <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          · projects × days · area = sessions · color = first-try-right rate
        </span>
      </div>

      <div style={{ display: 'grid',
                     gridTemplateColumns: `160px repeat(${days.length}, 1fr)`,
                     rowGap: 4, columnGap: 4, alignItems: 'center' }}>
        {/* Header row */}
        <div/>
        {days.map(d => (
          <div key={d} className="mono"
               style={{ fontSize: 11, color: 'var(--ink-4)', textAlign: 'center' }}>
            {dayShort[d]}
          </div>
        ))}
        {/* Project rows */}
        {projects.map(p => {
          const meta = D.projects[p];
          return (
            <React.Fragment key={p}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                             fontSize: 11, color: 'var(--ink-2)' }}>
                <span className="kanji" style={{ color: 'var(--accent)' }}>{meta.kanji}</span>
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap' }}>{meta.name}</span>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>{meta.lang}</span>
              </div>
              {days.map(d => {
                const c = cell(p, d);
                if (!c) {
                  return <div key={d} style={{ height: 22, opacity: 0.35,
                                  background: 'var(--paper)', borderRadius: 3 }}/>;
                }
                const size = Math.min(22, 8 + c.count * 5);
                const color = c.ftr === 1 ? 'var(--success)' :
                              c.ftr >= 0.5 ? 'var(--warning)' : 'var(--accent)';
                return (
                  <div key={d} style={{ height: 22, display: 'flex',
                                  alignItems: 'center', justifyContent: 'center' }}>
                    <div title={`${c.count} session(s) · FTR ${Math.round(c.ftr*100)}%`}
                         style={{ width: size, height: size, borderRadius: 4,
                                   background: color, opacity: 0.7 + c.ftr * 0.3,
                                   display: 'flex', alignItems: 'center',
                                   justifyContent: 'center', color: 'var(--paper)',
                                   fontSize: 11, fontFamily: 'JetBrains Mono', fontWeight: 500 }}>
                      {c.count}
                    </div>
                  </div>
                );
              })}
            </React.Fragment>
          );
        })}
      </div>

      <div style={{ display: 'flex', gap: 12, marginTop: 12, fontSize: 11,
                     color: 'var(--ink-4)' }}>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
          <span style={{ width: 9, height: 9, borderRadius: 2, background: 'var(--success)' }}/>
          first-try
        </span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
          <span style={{ width: 9, height: 9, borderRadius: 2, background: 'var(--warning)' }}/>
          some corrections
        </span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
          <span style={{ width: 9, height: 9, borderRadius: 2, background: 'var(--accent)' }}/>
          rework
        </span>
      </div>
    </section>
  );
}

Object.assign(window, { SessionsDigest, SessionsTimeline });
