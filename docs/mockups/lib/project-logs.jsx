// Project · Logs
// Diagnostic session explorer + issue report flow, aligned to the
// sumi/washi/shu design system (tokens.css). Scoped to a single project.
//
// Layout: outer is owned by the parent (sidebar + chrome). This component
// renders the **content pane** of the Logs section.
//   ┌──────────────┬────────────────────────────────────┐
//   │ Session list │ Trace stream                        │
//   │ (date▾/mod)  │  · header w/ stats                  │
//   │              │  · trace rows (expand for stdout)   │
//   └──────────────┴────────────────────────────────────┘
// Footer button "Report this session ↗" opens IssueReportModal — a
// markdown preview + compose pane styled to match the system.

const { useState: plS, useMemo: plM } = React;

// ── Stub data — same shape the Tauri logs file would export ──────────
const PLOG_SESSIONS = [
  {
    id: "sess-a1b2c3", module: "bootstrap", display_time: "Today · 10:42", project: { id: "lumen-cloud", name: "Lumen Cloud", kanji: "雲" },
    started_at: "2026-05-04T10:42:15.000Z",
    app_version: "0.4.2",
    system_info: { os: "macOS 15.4 Sequoia", arch: "arm64 (Apple M3)", ram_gb: 16, cpu_cores: 10 },
    outcome: "success", duration_ms: 3218,
    traces: [
      { id:"t01", ts:"10:42:15.102", action_type:"check",   step:"homebrew",        desc:"Homebrew package manager",            cmd:"which brew",                                                                    exit:0, out:"/opt/homebrew/bin/brew",                                   err:"",                    ms:12,   ok:true  },
      { id:"t02", ts:"10:42:15.115", action_type:"check",   step:"brew_version",    desc:"Homebrew version",                    cmd:"brew --version",                                                                exit:0, out:"Homebrew 4.4.2",                                         err:"",                    ms:38,   ok:true  },
      { id:"t03", ts:"10:42:15.154", action_type:"check",   step:"postgres_binary", desc:"PostgreSQL binary present",           cmd:"which postgres",                                                                exit:0, out:"/opt/homebrew/bin/postgres",                              err:"",                    ms:8,    ok:true  },
      { id:"t04", ts:"10:42:15.162", action_type:"check",   step:"postgres_port",   desc:"PostgreSQL service running",          cmd:"tcp probe 127.0.0.1:5432",                                                      exit:0, out:"connection accepted",                                   err:"",                    ms:18,   ok:true  },
      { id:"t05", ts:"10:42:15.182", action_type:"check",   step:"ollama_binary",   desc:"Ollama binary present",               cmd:"which ollama",                                                                  exit:0, out:"/opt/homebrew/bin/ollama",                               err:"",                    ms:7,    ok:true  },
      { id:"t06", ts:"10:42:15.190", action_type:"check",   step:"ollama_port",     desc:"Ollama service running",              cmd:"tcp probe 127.0.0.1:11434",                                                     exit:1, out:"",                                                       err:"connection refused",  ms:2002, ok:false, fix_attempted:true,  fix_approach:"brew services start ollama", fix_ok:true  },
      { id:"t07", ts:"10:42:17.194", action_type:"resolve", step:"start_ollama",    desc:"Starting Ollama service",             cmd:"brew services start ollama",                                                    exit:0, out:"Successfully started ollama",                            err:"",                    ms:841,  ok:true  },
      { id:"t08", ts:"10:42:18.036", action_type:"check",   step:"pg_is_ready",     desc:"PostgreSQL accepting connections",    cmd:"pg_isready --quiet",                                                            exit:0, out:"",                                                       err:"",                    ms:48,   ok:true  },
      { id:"t09", ts:"10:42:18.085", action_type:"check",   step:"database_exists", desc:"sensei database exists",              cmd:"psql -lqt",                                                                     exit:0, out:" sensei | jerry | UTF8",                                 err:"",                    ms:31,   ok:true  },
      { id:"t10", ts:"10:42:18.117", action_type:"check",   step:"pgvector",        desc:"pgvector extension installed",        cmd:"psql -d sensei -tAc \"...\"",                                                  exit:0, out:"1",                                                      err:"",                    ms:28,   ok:true  },
      { id:"t11", ts:"10:42:18.146", action_type:"check",   step:"daemon_port",     desc:"Daemon service running",              cmd:"tcp probe 127.0.0.1:7744",                                                      exit:0, out:"connection accepted",                                   err:"",                    ms:16,   ok:true  },
    ]
  },
  {
    id: "sess-x7y8z9", module: "bootstrap", display_time: "Today · 09:15", project: { id: "lumen-studio", name: "Lumen Studio", kanji: "工" },
    app_version: "0.4.2",
    system_info: { os: "macOS 15.4 Sequoia", arch: "arm64 (Apple M3)", ram_gb: 16, cpu_cores: 10 },
    outcome: "partial", duration_ms: 7341,
    traces: [
      { id:"s2t1", ts:"09:15:32.080", action_type:"check",   step:"homebrew",        desc:"Homebrew package manager",       cmd:"which brew",                       exit:0, out:"/opt/homebrew/bin/brew",           err:"",                    ms:11,   ok:true  },
      { id:"s2t2", ts:"09:15:32.092", action_type:"check",   step:"postgres_binary", desc:"PostgreSQL binary present",      cmd:"which postgres",                   exit:1, out:"",                                 err:"postgres: not found", ms:9,    ok:false, fix_attempted:true, fix_approach:"brew install postgresql@16", fix_ok:true  },
      { id:"s2t3", ts:"09:15:32.102", action_type:"resolve", step:"install_postgres",desc:"Installing PostgreSQL",          cmd:"brew install postgresql@16",       exit:0, out:"postgresql@16 17.2 installed",     err:"",                    ms:4218, ok:true  },
      { id:"s2t4", ts:"09:15:36.321", action_type:"check",   step:"postgres_port",   desc:"PostgreSQL service running",     cmd:"tcp probe 127.0.0.1:5432",         exit:1, out:"",                                 err:"connection refused",  ms:2002, ok:false, fix_attempted:true, fix_approach:"brew services start postgresql@16", fix_ok:true  },
      { id:"s2t5", ts:"09:15:38.324", action_type:"resolve", step:"start_postgres",  desc:"Starting PostgreSQL service",    cmd:"brew services start postgresql@16",exit:0, out:"Successfully started postgresql@16",err:"",                    ms:692,  ok:true  },
      { id:"s2t6", ts:"09:15:39.017", action_type:"check",   step:"daemon_port",     desc:"Daemon service running",         cmd:"tcp probe 127.0.0.1:7744",         exit:0, out:"connection accepted",              err:"",                    ms:22,   ok:true  },
    ]
  },
  {
    id: "sess-w1w2w3", module: "session", display_time: "Today · 11:05", project: { id: "lumen-cloud", name: "Lumen Cloud", kanji: "雲" },
    app_version: "0.4.2",
    system_info: { os: "macOS 15.4 Sequoia", arch: "arm64 (Apple M3)", ram_gb: 16, cpu_cores: 10 },
    outcome: "success", duration_ms: 8140,
    traces: [
      { id:"w1t1", ts:"11:05:02.100", action_type:"check",   step:"context_load",    desc:"Load project context",          cmd:"GET /api/projects/p-1/context",  exit:200, out:'{"memories":11,"libs":5}',          err:"",                    ms:42,  ok:true },
      { id:"w1t2", ts:"11:05:02.143", action_type:"resolve", step:"memories_inject", desc:"Inject 11 memories into prompt", cmd:"POST /api/sessions/s-2891/start",exit:200, out:'{"ok":true}',                       err:"",                    ms:58,  ok:true },
      { id:"w1t3", ts:"11:05:02.202", action_type:"check",   step:"assistants_load", desc:"Load configured assistants",     cmd:"GET /api/assistants/families",   exit:200, out:"3 families returned",               err:"",                    ms:39,  ok:true },
      { id:"w1t4", ts:"11:05:02.242", action_type:"resolve", step:"assistant_pick",  desc:"Route to claude (auth task)",    cmd:"POST /api/assistants/route",     exit:200, out:'{"family":"claude"}',               err:"",                    ms:74,  ok:true },
      { id:"w1t5", ts:"11:05:02.317", action_type:"check",   step:"trace_collect",   desc:"Collect ACP transcript",         cmd:"GET /api/sessions/s-2891/trace", exit:200, out:"148 events",                        err:"",                    ms:28,  ok:true },
    ]
  },
  {
    id: "sess-y2y3y4", module: "scan", display_time: "Yesterday · 16:30", project: { id: "brand-kit", name: "Brand Kit", kanji: "紋" },
    app_version: "0.4.2",
    system_info: { os: "macOS 15.4 Sequoia", arch: "arm64 (Apple M3)", ram_gb: 16, cpu_cores: 10 },
    outcome: "success", duration_ms: 1844,
    traces: [
      { id:"s3t1", ts:"16:30:41.022", action_type:"check", step:"scan_start",   desc:"Trigger incremental scan",       cmd:"POST /api/scan?mode=inc",  exit:202, out:'{"scan_id":"scan-xyz"}',    err:"", ms:10, ok:true },
      { id:"s3t2", ts:"16:30:41.033", action_type:"check", step:"scan_walk",    desc:"Walk filesystem (changed)",      cmd:"<internal>",                exit:0,   out:"217 files seen",            err:"", ms:740, ok:true },
      { id:"s3t3", ts:"16:30:41.773", action_type:"check", step:"scan_index",   desc:"Update embeddings",              cmd:"<internal>",                exit:0,   out:"42 embeddings updated",     err:"", ms:1080, ok:true },
      { id:"s3t4", ts:"16:30:42.853", action_type:"check", step:"scan_complete",desc:"Notify clients",                 cmd:"sse broadcast",              exit:0,   out:"3 listeners notified",      err:"", ms:14, ok:true },
    ]
  },
  {
    id: "sess-m3n4o5", module: "session", display_time: "Yesterday · 15:18", project: { id: "lumen-studio", name: "Lumen Studio", kanji: "工" },
    app_version: "0.4.1",
    system_info: { os: "macOS 15.4 Sequoia", arch: "arm64 (Apple M3)", ram_gb: 16, cpu_cores: 10 },
    outcome: "failed", duration_ms: 2318,
    traces: [
      { id:"m1t1", ts:"15:18:12.100", action_type:"check",   step:"context_load",  desc:"Load project context",   cmd:"GET /api/projects/p-1/context", exit:200, out:'{"memories":11}',                  err:"",                    ms:38, ok:true },
      { id:"m1t2", ts:"15:18:12.140", action_type:"resolve", step:"memories_inject", desc:"Inject memories",      cmd:"POST /api/sessions",            exit:500, out:"",                                 err:"OOM: model context > 200k tokens", ms:2120, ok:false, fix_attempted:true, fix_approach:"sensei drop low-relevance memories",  fix_ok:false },
      { id:"m1t3", ts:"15:18:14.260", action_type:"check",   step:"abort",          desc:"Session aborted",       cmd:"<internal>",                    exit:1,   out:"",                                 err:"context too large after dedupe",   ms:60,   ok:false },
    ]
  },
];

// Module metadata (kanji + label) — keeps with the visual system
const PLOG_MODULES = {
  bootstrap: { kanji: "健", label: "Bootstrap"  },
  session:   { kanji: "刻", label: "Session"    },
  scan:      { kanji: "観", label: "Scan"       },
  wizard:    { kanji: "導", label: "Setup"      },
};

const fmtMs = n => n >= 1000 ? (n/1000).toFixed(1)+'s' : n+'ms';
const outcomeTone = o =>
  o === 'success' ? 'var(--success)' :
  o === 'partial' ? 'var(--warning)' :
  o === 'failed'  ? 'var(--accent)'   : 'var(--ink-3)';
const anonymize = str => (str || "").replace(/\/Users\/[^/]+\//g, '~/');

const dateKey = s => s.display_time.split(' · ')[0];
const timeKey = s => s.display_time.split(' · ')[1] || '';

// Group sessions by date then by module
function groupSessions(sessions) {
  const dateOrder = []; const dateMap = {};
  sessions.forEach(s => {
    const dk = dateKey(s);
    if (!dateMap[dk]) { dateMap[dk] = { modOrder: [], modMap: {} }; dateOrder.push(dk); }
    const { modOrder, modMap } = dateMap[dk];
    if (!modMap[s.module]) { modMap[s.module] = []; modOrder.push(s.module); }
    modMap[s.module].push(s);
  });
  return dateOrder.map(dk => ({
    date: dk,
    groups: dateMap[dk].modOrder.map(mod => ({ mod, sessions: dateMap[dk].modMap[mod] })),
  }));
}

// ── Action badge — type pill on each trace row ────────────────────────
function PLogActionBadge({ type }) {
  const map = {
    check:    { label: 'CHECK',    color: 'var(--ink-3)', bg: 'var(--paper-3)' },
    resolve:  { label: 'RESOLVE',  color: 'var(--warning)',  bg: 'var(--warning-soft)' },
    instruct: { label: 'INSTRUCT', color: 'var(--accent)',    bg: 'var(--accent-soft)' },
  };
  const m = map[type] || map.check;
  return (
    <span className="mono py-1 px-1" style={{
      fontSize: 11, fontWeight: 600, letterSpacing: '0.12em', borderRadius: 3, color: m.color, background: m.bg,
      whiteSpace: 'nowrap'
}}>{m.label}</span>
  );
}

function PLogStatus({ trace }) {
  if (trace.ok) return <span style={{ color: 'var(--success)', fontSize: 13, lineHeight: 1 }}>✓</span>;
  if (trace.fix_ok) return (
    <span className="mono" style={{ fontSize: 11, color: 'var(--warning)',
                  fontWeight: 600, letterSpacing: '0.06em' }}>FIXED</span>
  );
  return <span style={{ color: 'var(--accent)', fontSize: 13, lineHeight: 1 }}>✗</span>;
}

// ── A single trace row + its expanded detail block ────────────────────
function PLogTraceRow({ trace, expanded, onToggle }) {
  const hasDetail = trace.out || trace.err || trace.fix_attempted;
  return (
    <div style={{ borderBottom: 'var(--hairline)' }}>
      <div onClick={hasDetail ? onToggle : undefined}
           style={{
             display: 'grid', gridTemplateColumns: '74px 168px 1fr 60px 40px', alignItems: 'center',
             cursor: hasDetail ? 'pointer' : 'default'
}} className="gap-3 py-2 px-0" >
        <PLogActionBadge type={trace.action_type}/>
        <span className="mono" style={{ fontSize: 13, color: 'var(--ink-2)',
                      overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>
          {trace.step}
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                      overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>
          {anonymize(trace.cmd)}
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                      textAlign: 'right', fontFeatureSettings: '"tnum"' }}>{fmtMs(trace.ms)}</span>
        <div style={{ textAlign: 'center' }}><PLogStatus trace={trace}/></div>
      </div>

      {expanded && (
        <div style={{
          background: 'var(--paper-2)', borderRadius: 5, border: 'var(--hairline)'
}} className="mb-3 py-3 px-3 ml-9" >
          {trace.out && (
            <div style={{ marginBottom: trace.err ? 10 : 0 }}>
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                              color: 'var(--ink-4)'
}} className="mb-1" >stdout</div>
              <pre className="mono m-0" style={{
 fontSize: 11,
                              color: 'var(--ink-2)', lineHeight: 1.6,
                              whiteSpace: 'pre-wrap', wordBreak: 'break-all'
}}>
                {anonymize(trace.out)}
              </pre>
            </div>
          )}
          {trace.err && (
            <div>
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                              color: 'var(--accent)'
}} className="mb-1" >stderr</div>
              <pre className="mono m-0" style={{
 fontSize: 11,
                              color: 'var(--accent)', lineHeight: 1.6
}}>{trace.err}</pre>
            </div>
          )}
          {trace.fix_attempted && (
            <div style={{
              marginTop: trace.out || trace.err ? 10 : 0,
              paddingTop: trace.out || trace.err ? 10 : 0,
              borderTop: trace.out || trace.err ? 'var(--hairline)' : 'none'
            }}>
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                              color: 'var(--warning)'
}} className="mb-1" >auto-fix attempted</div>
              <pre className="mono m-0" style={{
 fontSize: 11,
                              color: trace.fix_ok ? 'var(--success)' : 'var(--accent)',
                              lineHeight: 1.6
}}>
                $ {trace.fix_approach}{'  '}
                <span style={{ color: 'var(--ink-4)', fontSize: 11 }}>
                  → {trace.fix_ok ? 'succeeded' : 'failed'}
                </span>
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Issue report modal ───────────────────────────────────────────────
function PLogIssueModal({ session, project, onClose }) {
  const [context, setContext] = plS('');
  const [copied,  setCopied]  = plS(false);

  const si = session.system_info;
  const traceMd = session.traces.map(t =>
    `| ${t.step} | ${t.action_type} | \`${anonymize(t.cmd)}\` | ${fmtMs(t.ms)} | ${t.ok ? '✓' : t.fix_ok ? '✓ fixed' : '✗'} |`
  ).join('\n');

  const fixTraces = session.traces.filter(t => t.err || t.fix_attempted);
  const fixDetailsMd = fixTraces.length ? [
    ``, `### Fix details`, ``,
    ...fixTraces.flatMap(t => [
      `**${t.step}** (${t.action_type} · ${fmtMs(t.ms)}) — ${t.ok ? '✓' : t.fix_ok ? '✓ fixed' : '✗ failed'}`,
      ...(t.err ? [`- stderr: \`${t.err}\``] : []),
      ...(t.out && !t.ok ? [`- stdout: \`${anonymize(t.out)}\``] : []),
      ...(t.fix_attempted ? [
        `- fix applied: \`$ ${t.fix_approach}\` → ${t.fix_ok ? 'success' : 'failed'}`
      ] : []),
      ``,
    ]),
  ] : [];

  const moduleLabel = PLOG_MODULES[session.module]?.label ?? session.module;
  const issueTitle = `${moduleLabel} diagnostic — ${si.os} · ${si.arch} · v${session.app_version}`;
  const issueBody = [
    `## System info`,
    `- Project: ${project.name}`,
    `- Module: ${moduleLabel}`,
    `- OS: ${si.os}`,
    `- Architecture: ${si.arch}`,
    `- RAM: ${si.ram_gb} GB · ${si.cpu_cores} cores`,
    `- App version: v${session.app_version}`,
    `- Session: ${session.display_time}`,
    ``,
    `## Trace`,
    ``,
    `| Step | Type | Command | Duration | Result |`,
    `|------|------|---------|----------|--------|`,
    traceMd,
    ...fixDetailsMd,
    ...(context ? [``, `## Additional context`, ``, context] : []),
  ].join('\n');

  const copy = () => {
    navigator.clipboard?.writeText(issueBody);
    setCopied(true);
    setTimeout(() => setCopied(false), 1400);
  };

  return (
    <div onClick={onClose}
         style={{ position: 'absolute', inset: 0, background: 'rgba(20,18,15,0.42)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  zIndex: 30, backdropFilter: 'blur(2px)' }}>
      <div onClick={e => e.stopPropagation()}
           style={{
             width: '88%', maxWidth: 980, maxHeight: '90%',
             background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 12,
             display: 'flex', flexDirection: 'column',
             boxShadow: '0 24px 60px rgba(0,0,0,0.22)'
           }}>
        {/* Header */}
        <div style={{
          display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', borderBottom: 'var(--hairline)', flexShrink: 0
}} className="pt-4 pb-3 px-5" >
          <div>
            <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-1" >
              <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>診</span>
              <h3 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                            letterSpacing: '-0.01em'
}}>
                Report this session
              </h3>
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {session.display_time} · {session.traces.length} traces ·
              {' '}{session.traces.filter(t => t.fix_attempted).length} auto-fixes ·
              {' '}<span className="mono">{session.id}</span>
            </div>
          </div>
          <button onClick={onClose}
                  style={{
 background: 'transparent', border: 'none', cursor: 'pointer',
                           color: 'var(--ink-3)', fontSize: 22,
                           lineHeight: 1
}} className="px-1" >×</button>
        </div>

        {/* Body — two columns */}
        <div style={{
 flex: 1, overflow: 'auto',
                       display: 'grid', gridTemplateColumns: '1fr 280px', minHeight: 0
}} className="gap-5 pt-4 pb-5 px-5" >

          {/* Preview column */}
          <div style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }} className="gap-2" >
            <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                            color: 'var(--ink-3)' }}>Issue preview · anonymized</div>

            {/* Title */}
            <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                            borderRadius: 5,
                            fontSize: 13, color: 'var(--ink)', fontWeight: 500, flexShrink: 0
}} className="py-2 px-3" >
              {issueTitle}
            </div>

            {/* Body */}
            <pre className="mono py-3 px-4 m-0" style={{
 flex: 1, background: 'var(--paper-2)',
                              border: 'var(--hairline)', borderRadius: 5,
                              fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.7,
                              overflow: 'auto', whiteSpace: 'pre-wrap', wordBreak: 'break-word',
                              minHeight: 240
}}>
              {issueBody}
            </pre>
          </div>

          {/* Compose column */}
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-3" >
            {/* Included summary */}
            <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                            borderRadius: 6
}} className="py-3 px-3" >
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                              color: 'var(--ink-3)'
}} className="mb-2" >
                Included in report
              </div>
              {[
                ['Project', project.name],
                ['Module',  moduleLabel],
                ['Session', session.display_time],
                ['OS',      si.os],
                ['Arch',    si.arch],
                ['RAM',     `${si.ram_gb} GB`],
                ['Traces',  session.traces.length],
                ['Fixes',   session.traces.filter(t => t.fix_attempted).length],
                ['App',     `v${session.app_version}`],
              ].map(([k, v]) => (
                <div key={k} style={{
 display: 'flex', justifyContent: 'space-between',
                                       fontSize: 11
}} className="mb-1" >
                  <span style={{ color: 'var(--ink-3)' }}>{k}</span>
                  <span className={['Traces','Fixes','App','RAM','Arch','OS'].includes(k) ? 'mono' : ''}
                         style={{ color: 'var(--ink-2)',
                                   fontSize: ['OS','Arch'].includes(k) ? 10.5 : 11.5,
                                   maxWidth: 160, overflow: 'hidden',
                                   whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>
                    {v}
                  </span>
                </div>
              ))}
            </div>

            {/* Compose */}
            <div>
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                              color: 'var(--ink-3)'
}} className="mb-1" >
                Additional context
              </div>
              <textarea value={context} onChange={e => setContext(e.target.value)}
                          placeholder="What were you doing? Anything else worth knowing?"
                          style={{
 width: '100%', height: 96, resize: 'none',
                                    background: 'var(--paper-2)', border: 'var(--hairline)',
                                    borderRadius: 5, fontSize: 13, color: 'var(--ink)',
                                    fontFamily: 'inherit', lineHeight: 1.5
}} className="py-2 px-2" />
            </div>

            {/* Privacy note */}
            <div style={{ fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.7 }}>
              Paths like{' '}
              <code className="mono py-1 px-1" style={{
 background: 'var(--paper-3)', borderRadius: 3, fontSize: 11
}}>/Users/jerry/</code>
              {' '}are replaced with{' '}
              <code className="mono py-1 px-1" style={{
 background: 'var(--paper-3)', borderRadius: 3, fontSize: 11
}}>~/</code>.
              No personal data is included.
            </div>

            <div style={{ flex: 1 }}/>

            {/* Actions */}
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
              <button onClick={() => alert('Opens github.com/sensei-hq/app/issues/new')}
                      style={{
 width: '100%', borderRadius: 6,
                                fontSize: 13, fontWeight: 500, border: 'none', cursor: 'pointer',
                                background: 'var(--ink)', color: 'var(--paper)'
}} className="py-2 px-4" >
                Submit to GitHub ↗
              </button>
              <button onClick={copy}
                      style={{
 width: '100%', borderRadius: 6,
                                fontSize: 13, fontWeight: 500, cursor: 'pointer',
                                background: 'transparent', border: 'var(--hairline)',
                                color: copied ? 'var(--success)' : 'var(--ink-2)',
                                transition: 'color 0.2s'
}} className="py-2 px-4" >
                {copied ? 'Copied ✓' : 'Copy markdown'}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Main Logs section ────────────────────────────────────────────────
// scope: "project" (default) — sessions for one project, scoped subtitle
// scope: "collective"        — every session sensei has run, project tags shown
function ProjLogs({ project, scope = "project" }) {
  const collective = scope === "collective";
  const [selectedId, setSelectedId] = plS(PLOG_SESSIONS[0].id);
  const [expandedTraceId, setExpandedTraceId] = plS(null);
  const [openDates, setOpenDates] = plS(new Set(['Today']));
  const [showReport, setShowReport] = plS(false);
  const [moduleFilter, setModuleFilter] = plS("all"); // all | bootstrap | session | scan

  const visibleSessions = plM(() => {
    if (moduleFilter === "all") return PLOG_SESSIONS;
    return PLOG_SESSIONS.filter(s => s.module === moduleFilter);
  }, [moduleFilter]);

  const session = visibleSessions.find(s => s.id === selectedId)
                || PLOG_SESSIONS.find(s => s.id === selectedId)
                || visibleSessions[0]
                || PLOG_SESSIONS[0];
  const dateGroups = plM(() => groupSessions(visibleSessions), [visibleSessions]);

  const toggleDate = dk => setOpenDates(prev => {
    const next = new Set(prev);
    next.has(dk) ? next.delete(dk) : next.add(dk);
    return next;
  });
  const toggleTrace = id => setExpandedTraceId(p => p === id ? null : id);

  const fixCount = session.traces.filter(t => t.fix_attempted).length;
  const stats = [
    { label: 'Total time', value: fmtMs(session.duration_ms) },
    { label: 'Traces',     value: session.traces.length },
    { label: 'Auto-fixes', value: fixCount },
    { label: 'Outcome',    value: session.outcome, color: outcomeTone(session.outcome) },
  ];

  return (
    <div style={{ height: '100%', position: 'relative',
                   display: 'grid', gridTemplateColumns: '264px 1fr', minHeight: 0 }}>

      {/* ── Session list ── */}
      <aside style={{ borderRight: 'var(--hairline)', background: 'var(--paper-2)',
                       display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div style={{ borderBottom: 'var(--hairline)' }} className="pt-5 pb-3 px-4" >
          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
            <span className="kanji" style={{ fontSize: 17, color: 'var(--accent)' }}>診</span>
            <span style={{ fontSize: 11, letterSpacing: '0.16em', textTransform: 'uppercase',
                             color: 'var(--ink-3)' }}>diagnostic logs</span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--ink-4)' }} className="mt-1" >
            {collective ? (
              <>every session sensei has run · <span className="mono" style={{ color: 'var(--ink-3)' }}>{PLOG_SESSIONS.length} total</span></>
            ) : (
              <>scoped to <span className="mono" style={{ color: 'var(--ink-3)' }}>{project?.name}</span></>
            )}
          </div>

          {collective && (
            <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1 mt-3" >
              {[
                { id: 'all',       label: 'All' },
                { id: 'bootstrap', label: 'Bootstrap' },
                { id: 'session',   label: 'Session' },
                { id: 'scan',      label: 'Scan' },
              ].map(f => {
                const on = moduleFilter === f.id;
                return (
                  <button key={f.id} onClick={() => setModuleFilter(f.id)}
                          style={{
 borderRadius: 4, cursor: 'pointer',
                                    border: 'var(--hairline)', fontSize: 11,
                                    background: on ? 'var(--ink)' : 'transparent',
                                    color: on ? 'var(--paper)' : 'var(--ink-2)',
                                    borderColor: on ? 'var(--ink)' : undefined
}} className="py-1 px-2" >
                    {f.label}
                  </button>
                );
              })}
            </div>
          )}
        </div>

        <div style={{ flex: 1, overflowY: 'auto' }} className="py-2 px-0" >
          {dateGroups.map(({ date, groups }) => {
            const open = openDates.has(date);
            return (
              <div key={date}>
                <div onClick={() => toggleDate(date)}
                     style={{
 display: 'flex', alignItems: 'center', cursor: 'pointer', userSelect: 'none'
}} className="gap-1 pt-2 pb-1 px-4" >
                  <span style={{ fontSize: 11, color: 'var(--ink-4)', lineHeight: 1,
                                  transform: open ? 'none' : 'rotate(-90deg)',
                                  display: 'inline-block', transition: 'transform 0.15s' }}>▾</span>
                  <span style={{ fontSize: 11, fontWeight: 600, letterSpacing: '0.06em',
                                  textTransform: 'uppercase', color: 'var(--ink-2)' }}>
                    {date}
                  </span>
                </div>

                {open && groups.map(({ mod, sessions }) => (
                  <div key={mod}>
                    <div style={{
 display: 'flex', alignItems: 'center'
}} className="gap-1 py-1 pl-6 pr-4" >
                      <span className="kanji" style={{ fontSize: 11, color: 'var(--ink-4)',
                                    lineHeight: 1 }}>
                        {PLOG_MODULES[mod]?.kanji ?? '◆'}
                      </span>
                      <span style={{ fontSize: 11, letterSpacing: '0.14em',
                                      textTransform: 'uppercase', color: 'var(--ink-4)' }}>
                        {PLOG_MODULES[mod]?.label ?? mod}
                      </span>
                    </div>

                    {sessions.map(s => {
                      const sel = s.id === selectedId;
                      const isCurrent = s.id === PLOG_SESSIONS[0].id;
                      return (
                        <div key={s.id}
                             onClick={() => { setSelectedId(s.id); setExpandedTraceId(null); }}
                             style={{
 cursor: 'pointer',
                                       background: sel ? 'var(--paper)' : 'transparent',
                                       borderLeft: sel ? '2px solid var(--accent)' : '2px solid transparent',
                                       transition: 'background 0.12s'
}} className="py-2 pl-6 pr-4" >
                          {isCurrent && (
                            <div style={{
 fontSize: 11, letterSpacing: '0.16em',
                                            textTransform: 'uppercase',
                                            color: 'var(--accent)'
}} className="mb-1" >current</div>
                          )}
                          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-1 mb-1" >
                            <span style={{ width: 6, height: 6, borderRadius: '50%', flexShrink: 0,
                                            background: outcomeTone(s.outcome) }}/>
                            <span style={{ fontSize: 13, fontWeight: 500,
                                            color: sel ? 'var(--ink)' : 'var(--ink-2)' }}>
                              {timeKey(s)}
                            </span>
                            {collective && s.project && (
                              <span style={{
 display: 'inline-flex',
                                              alignItems: 'center', fontSize: 11,
                                              color: 'var(--ink-3)'
}} className="gap-1 ml-auto" >
                                <span className="kanji" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
                                  {s.project.kanji}
                                </span>
                                <span style={{ maxWidth: 80, whiteSpace: 'nowrap',
                                                overflow: 'hidden', textOverflow: 'ellipsis' }}>
                                  {s.project.name}
                                </span>
                              </span>
                            )}
                          </div>
                          <div className="mono ml-3" style={{
 fontSize: 11,
                                          color: 'var(--ink-3)'
}}>
                            {fmtMs(s.duration_ms)} · {s.traces.length} steps
                            {s.traces.filter(t => t.fix_attempted).length
                              ? ` · ${s.traces.filter(t => t.fix_attempted).length} fix`
                              : ''}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                ))}
              </div>
            );
          })}
        </div>

        <div style={{
 borderTop: 'var(--hairline)',
                       fontSize: 11, color: 'var(--ink-4)', lineHeight: 1.6
}} className="py-2 px-4" >
          <span className="mono">retention · 30 days</span>
        </div>
      </aside>

      {/* ── Trace stream ── */}
      <div style={{ display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
        {/* Session header */}
        <div style={{ borderBottom: 'var(--hairline)' }} className="pt-5 pb-4 px-6" >
          <div style={{
 display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between'
}} className="gap-4 mb-4" >
            <div style={{ minWidth: 0, flex: 1 }}>
              <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-1" >
                <span className="kanji" style={{ fontSize: 13, color: 'var(--ink-3)' }}>
                  {PLOG_MODULES[session.module]?.kanji ?? '◆'}
                </span>
                <span style={{ fontSize: 11, letterSpacing: '0.16em', textTransform: 'uppercase',
                                color: 'var(--ink-3)' }}>
                  {PLOG_MODULES[session.module]?.label ?? session.module} · session
                </span>
                {collective && session.project && (
                  <>
                    <span style={{ color: 'var(--ink-4)', opacity: 0.5 }}>·</span>
                    <span style={{
 display: 'inline-flex', alignItems: 'center',
                                    fontSize: 11, color: 'var(--ink-2)'
}} className="gap-1" >
                      <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>
                        {session.project.kanji}
                      </span>
                      {session.project.name}
                    </span>
                  </>
                )}
              </div>
              <h2 className="display mt-0 mb-2" style={{
 fontSize: 22, fontWeight: 400, letterSpacing: '-0.01em'
}}>
                {session.display_time}
              </h2>
              <div style={{
 display: 'flex', flexWrap: 'wrap',
                             fontSize: 11, color: 'var(--ink-3)'
}} className="gap-2" >
                {[session.system_info.os, session.system_info.arch,
                  `${session.system_info.ram_gb} GB`, `${session.system_info.cpu_cores} cores`,
                  `v${session.app_version}`].map((v, i, a) => (
                  <React.Fragment key={i}>
                    <span className={i >= 1 && i <= 3 ? 'mono' : ''}>{v}</span>
                    {i < a.length - 1 && <span style={{ opacity: 0.4 }}>·</span>}
                  </React.Fragment>
                ))}
                <span style={{ opacity: 0.4 }}>·</span>
                <span className="mono">{session.id}</span>
              </div>
            </div>
            <button onClick={() => setShowReport(true)}
                    style={{
 flexShrink: 0, borderRadius: 6,
                              border: 'none', cursor: 'pointer',
                              background: 'var(--ink)', color: 'var(--paper)',
                              fontSize: 13, fontWeight: 500
}} className="py-2 px-4" >
              Report this session ↗
            </button>
          </div>

          <div style={{ display: 'flex' }} className="gap-5" >
            {stats.map(st => (
              <div key={st.label} style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
                <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                                color: 'var(--ink-3)' }}>{st.label}</div>
                <div className="display" style={{ fontSize: 15, fontWeight: 400,
                                color: st.color || 'var(--ink)' }}>
                  {st.value}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Column headers */}
        <div style={{
 display: 'grid', gridTemplateColumns: '74px 168px 1fr 60px 40px', background: 'var(--paper-2)',
                       borderBottom: 'var(--hairline)', flexShrink: 0
}} className="gap-3 py-2 px-6" >
          {['action', 'step', 'command', 'duration', ''].map((h, i) => (
            <div key={i} style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                                    color: 'var(--ink-4)',
                                    textAlign: i === 3 ? 'right' : 'left' }}>{h}</div>
          ))}
        </div>

        {/* Trace rows */}
        <div style={{ flex: 1, overflowY: 'auto' }} className="px-6" >
          {session.traces.map(t => (
            <PLogTraceRow key={t.id} trace={t}
                           expanded={expandedTraceId === t.id}
                           onToggle={() => toggleTrace(t.id)}/>
          ))}
          <div style={{ height: 28 }}/>
        </div>
      </div>

      {showReport && (
        <PLogIssueModal session={session} project={project}
                          onClose={() => setShowReport(false)}/>
      )}
    </div>
  );
}

// Collective-scoped wrapper — used by the observatory's "Logs" sidebar
// route. No project chrome — assumes it's mounted inside the observatory
// shell. Shows every session sensei has run, tagged by project.
function ObsLogs() {
  return (
    <div className="sensei" data-screen-label="Collective · Logs"
         style={{ width: '100%', height: '100%', overflow: 'hidden' }}>
      <ProjLogs scope="collective"/>
    </div>
  );
}

// Standalone canvas wrapper — collective-scope Logs section with its own
// Tauri chrome and a faux observatory sidebar so the section is legible
// out-of-context.
function ProjectLogsPage() {
  return (
    <div className="sensei" data-screen-label="Collective · Logs"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                   background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <PerspectiveChrome
        title="先生  ·  Sensei"
        subtitle="logs · all projects"
        accent="var(--success)"/>
      <div style={{ flex: 1, minHeight: 0 }}>
        <ProjLogs scope="collective"/>
      </div>
    </div>
  );
}

// Standalone "report modal" canvas — show the issue modal in isolation
// over the collective logs view.
function ProjectLogsReportPage() {
  const [open, setOpen] = plS(true);
  const session = PLOG_SESSIONS.find(s => s.outcome === 'partial') || PLOG_SESSIONS[1];
  return (
    <div className="sensei" data-screen-label="Collective · Logs · report"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                   background: 'var(--paper)', overflow: 'hidden', position: 'relative' }}>
      <PerspectiveChrome
        title="先生  ·  Sensei"
        subtitle="logs · report"
        accent="var(--success)"/>
      <div style={{ flex: 1, minHeight: 0, position: 'relative' }}>
        <ProjLogs scope="collective"/>
        {open && (
          <PLogIssueModal session={session} project={session.project}
                            onClose={() => setOpen(false)}/>
        )}
      </div>
    </div>
  );
}

Object.assign(window, { ProjLogs, ObsLogs, ProjectLogsPage, ProjectLogsReportPage, PLOG_SESSIONS, PLOG_MODULES });
