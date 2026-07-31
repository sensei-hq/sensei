// ═══════════════════════════════════════════════════════════════════
//  SENSEI RELAY — the phone companion to the Observatory desktop app
//  Re-skin of the Relay concept into the Zen-Sumi system.
//    terracotta "needs you"  → var(--accent) (shu vermillion)
//    sage "running"          → var(--success) (jade)
//    warm-neutral surfaces   → washi paper tokens
//    SF system + mono        → Fraunces display · Inter ui · JetBrains mono
//  Screens synthesize the three explored directions: a calm 3-state
//  dashboard, a task view that carries BOTH the stage checklist and the
//  activity timeline, first-class approve / respond gates, and the
//  encrypted pairing/security surface.
//  Colours resolve from tokens (content wrapped in .sensei), so light +
//  dark themes come for free.
// ═══════════════════════════════════════════════════════════════════

const R = {
  accent:  'var(--accent)',
  aSoft:   'var(--accent-soft)',
  jade:    'var(--success)',
  jadeSoft:'var(--success-soft)',
  ink:     'var(--ink)',
  ink2:    'var(--ink-2)',
  ink3:    'var(--ink-3)',
  ink4:    'var(--ink-4)',
  paper:   'var(--paper)',
  paper2:  'var(--paper-2)',
  paper3:  'var(--paper-3)',
  edge:    'var(--edge)',
  mono:    'var(--font-mono)',
  ui:      'var(--font-ui)',
  disp:    'var(--font-display)',
};

// ── glyphs ───────────────────────────────────────────────────
const Lock = ({ s = 16, c = R.ink2 }) => (
  <svg width={s} height={s * 17 / 15} viewBox="0 0 15 17" fill="none">
    <rect x="1.5" y="7" width="12" height="9" rx="2.5" stroke={c} strokeWidth="1.6"/>
    <path d="M4 7V5a3.5 3.5 0 0 1 7 0v2" stroke={c} strokeWidth="1.6"/>
  </svg>
);
const Check = ({ s = 12, c = 'var(--paper)', w = 1.9 }) => (
  <svg width={s} height={s} viewBox="0 0 13 13"><path d="M2.5 7l2.8 2.8L10.5 3.5" stroke={c} strokeWidth={w} fill="none" strokeLinecap="round" strokeLinejoin="round"/></svg>
);
const Chevron = ({ c = R.ink3 }) => (
  <svg width="9" height="15" viewBox="0 0 9 15"><path d="M7 1L1.5 7.5 7 14" stroke={c} strokeWidth="2.2" fill="none" strokeLinecap="round" strokeLinejoin="round"/></svg>
);
const ArrowUp = ({ c = 'var(--paper)' }) => (
  <svg width="15" height="15" viewBox="0 0 15 15"><path d="M7.5 13V3M3 7l4.5-4.5L12 7" stroke={c} strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round"/></svg>
);
const Warn = ({ s = 22, c = R.accent }) => (
  <svg width={s} height={s} viewBox="0 0 24 24" fill="none"><path d="M12 3l9 16H3l9-16z" stroke={c} strokeWidth="2" strokeLinejoin="round"/><path d="M12 10v4" stroke={c} strokeWidth="2" strokeLinecap="round"/><circle cx="12" cy="16.6" r="1.1" fill={c}/></svg>
);

const Backbtn = () => (
  <div className="border border-paper-edge flex items-center justify-center shrink-0" style={{ width: 34, height: 34, borderRadius: 17, background: R.paper2 }}><Chevron/></div>
);

// content root that clears the status bar + island
const Screen = ({ children, pad = 56, style = {} }) => (
  <div className="sensei min-h-full flex flex-col" style={{ boxSizing: 'border-box', background: R.paper, paddingTop: pad, fontFamily: R.ui, color: R.ink, ...style }}>{children}</div>
);

const RelayMark = () => (
  <span className="mono font-semibold" style={{ fontSize: 12, letterSpacing: '2.5px', color: R.accent }}>先 · RELAY</span>
);

// ═══════════════════════════════════════════════════════════════════
//  DASHBOARD — calm 3-state header + tasks grouped by machine, each
//  row carrying a stage bar. The single "needs you" leads.
// ═══════════════════════════════════════════════════════════════════
function StageBar({ stages }) {
  // stages: array of 'done'|'live'|'block'|'todo'
  const col = { done: R.ink, live: R.jade, block: R.accent, todo: 'color-mix(in oklch, var(--ink) 12%, transparent)' };
  return (
    <div className="flex" style={{ gap: 4, marginTop: 12 }}>
      {stages.map((s, i) => <span className="flex-1" key={i} style={{ height: 4, borderRadius: 2, background: col[s] }}/>)}
    </div>
  );
}

function TaskCard({ task, onOpen }) {
  const blocked = task.state === 'block';
  const running = task.state === 'live';
  const done = task.state === 'done';
  const dotCol = blocked ? R.accent : running ? R.jade : R.ink4;
  const statusCol = blocked ? R.accent : running ? R.jade : R.ink3;
  return (
    <button className="text-left w-full block cursor-pointer" onClick={onOpen} style={{
 background: blocked ? R.aSoft : R.paper2,
 border: blocked ? '1px solid color-mix(in oklch, var(--accent) 30%, transparent)' : 'var(--hairline)',
 borderRadius: 20, padding: '16px 16px', opacity: done ? 0.66 : 1 }}>
      <div className="flex items-center justify-between" >
        <span className="mono flex items-center" style={{ fontSize: 11, letterSpacing: '.3px', color: statusCol, gap: 7 }}>
          <span className="rounded-full" style={{ width: 7, height: 7, background: dotCol, animation: running ? 'relayPulse 2s infinite' : 'none' }}/>{task.machine}
        </span>
        <span className="mono" style={{ fontSize: 12, color: R.ink3 }}>{task.ago}</span>
      </div>
      <div className="font-semibold" style={{ fontSize: 17, margin: '12px 0 4px', color: R.ink }}>{task.title}</div>
      <div className="font-medium flex items-center" style={{ fontSize: 13, color: statusCol, gap: 6, fontFamily: R.ui }}>
        {done && <Check s={13} c={R.ink3} w={1.8}/>}{task.status}
      </div>
      {!done && <StageBar stages={task.stages}/>}
    </button>
  );
}

function RelayDashboard({ dark = false }) {
  const tasks = [
    { title: 'Refactor auth module', machine: 'macbook-pro', ago: '4m', state: 'block', status: 'Needs you — approve DB migration', stages: ['done','done','done','done','block','todo'] },
    { title: 'Write integration tests', machine: 'mac-mini', ago: 'now', state: 'live', status: 'Running — writing test_checkout.py', stages: ['done','done','live','todo','todo'] },
    { title: 'Nightly data backfill', machine: 'mac-mini', ago: 'now', state: 'live', status: 'Processing batch 42 / 120', stages: ['done','live','todo','todo'] },
    { title: 'Update dependencies', machine: 'macbook-pro', ago: '12m', state: 'done', status: 'Completed · 24 files changed', stages: [] },
  ];
  return (
    <IOSDevice dark={dark}>
      <Screen pad={60}>
        <div style={{ padding: '8px 24px 16px' }}>
          <div className="flex items-center justify-between" >
            <RelayMark/>
            <div className="border border-paper-edge flex items-center justify-center" style={{ width: 34, height: 34, borderRadius: 17, background: R.paper2 }}><Lock/></div>
          </div>
          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 32, letterSpacing: '-.02em', marginTop: 14, color: R.ink }}>Agents</div>
          <div className="flex" style={{ gap: 26, marginTop: 16 }}>
            {[['1','needs you',R.accent],['2','running',R.jade],['5','done',R.ink4]].map(([n,l,c]) => (
              <div key={l}><div className="font-normal" style={{ fontFamily: R.disp, fontSize: 30, letterSpacing: '-.03em', color: c }}>{n}</div><div style={{ fontSize: 12, color: R.ink3, marginTop: 2 }}>{l}</div></div>
            ))}
          </div>
        </div>
        <div className="flex-1 overflow-auto flex flex-col" style={{ padding: '4px 16px 32px', gap: 12 }}>
          {tasks.map((t, i) => <TaskCard key={i} task={t}/>)}
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  TASK DETAIL — stage checklist up top, live activity timeline below,
//  the pending approval docked as a sheet. One screen, both models.
// ═══════════════════════════════════════════════════════════════════
function TimelineRow({ time, live, block, children }) {
  const c = block ? R.accent : live ? R.jade : R.ink4;
  return (
    <div className="relative" >
      <span className="absolute rounded-full" style={{ left: block ? -28 : -27, top: 2, width: block ? 12 : 10, height: block ? 12 : 10, background: c, boxShadow: '0 0 0 3px var(--paper)' }}/>
      <div className="mono" style={{ fontSize: 11, color: block ? R.accent : R.ink3 }}>{time}</div>
      <div style={{ fontSize: 14, marginTop: 3, color: R.ink }}>{children}</div>
    </div>
  );
}

function RelayTaskDetail({ dark = false }) {
  const chip = { fontFamily: R.mono, fontSize: 12, background: R.paper3, padding: '4px 8px', borderRadius: 6 };
  return (
    <IOSDevice dark={dark}>
      <Screen pad={56}>
        <div className="flex items-center justify-between" style={{ padding: '8px 24px 0' }}>
          <Backbtn/>
          <span className="mono" style={{ fontSize: 12, color: R.ink3 }}>macbook-pro · Claude · 4m</span>
        </div>
        <div style={{ padding: '12px 24px 8px' }}>
          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 24, letterSpacing: '-.01em', color: R.ink }}>Refactor auth module</div>
          <div className="font-semibold flex items-center" style={{ fontSize: 13, color: R.accent, marginTop: 8, gap: 7 }}><span className="rounded-full" style={{ width: 8, height: 8, background: R.accent }}/>Blocked — waiting for your approval</div>
        </div>

        <div className="flex-1 overflow-auto" style={{ padding: '12px 24px 24px' }}>
          {/* stages */}
          <div className="zs-eyebrow" style={{ marginBottom: 10 }}>Plan · 4 of 6</div>
          <div className="border border-paper-edge" style={{ background: R.paper2, borderRadius: 16, padding: '4px 4px', marginBottom: 22 }}>
            {[['Analyze current auth flow','done'],['Draft OAuth schema','done'],['Migrate user model','done'],['Apply DB migration','block'],['Update tests & ship','todo']].map(([label, st], i, a) => {
              const done = st === 'done', block = st === 'block';
              return (
                <div className="flex items-center" key={i} style={{ gap: 13, padding: '16px 16px', borderBottom: i < a.length - 1 ? '.5px solid var(--edge)' : 'none', background: block ? R.aSoft : 'transparent', borderRadius: block ? 12 : 0, margin: block ? '2px 4px' : 0 }}>
                  <span className="rounded-full shrink-0 flex items-center justify-center" style={{ width: 22, height: 22, background: done ? R.ink : 'transparent', border: done ? 'none' : block ? '2.5px solid var(--accent)' : '2px solid color-mix(in oklch, var(--ink) 16%, transparent)' }}>{done && <Check/>}</span>
                  <div className="flex-1" >
                    <div style={{ fontSize: 15, fontWeight: block ? 600 : 400, color: done ? R.ink3 : R.ink }}>{label}</div>
                    {block && <div className="font-medium" style={{ fontSize: 12, color: R.accent }}>needs approval</div>}
                  </div>
                </div>
              );
            })}
          </div>

          {/* timeline */}
          <div className="zs-eyebrow" style={{ marginBottom: 12 }}>Activity</div>
          <div className="flex flex-col" style={{ borderLeft: '2px solid var(--edge)', paddingLeft: 20, gap: 18, marginLeft: 4 }}>
            <TimelineRow time="14:02">Started task · read 12 files</TimelineRow>
            <TimelineRow time="14:05"><span className="flex items-center flex-wrap" style={{ gap: 8 }}>Edited <span style={chip}>auth.ts</span><span className="mono" style={{ fontSize: 12, color: R.jade }}>+42</span><span className="mono" style={{ fontSize: 12, color: R.accent }}>−8</span></span></TimelineRow>
            <TimelineRow time="14:08" live>Ran test suite · <span className="font-semibold" style={{ color: R.jade }}>18 passed</span></TimelineRow>
            <TimelineRow time="14:10 · WAITING" block>Wants to apply the migration.</TimelineRow>
          </div>
        </div>

        {/* approval sheet */}
        <div style={{ padding: '12px 16px 32px', background: 'linear-gradient(180deg, transparent, var(--paper) 34%)' }}>
          <div className="border border-paper-edge shadow" style={{ background: R.paper2, borderRadius: 20, padding: '16px 16px' }}>
            <div className="mono" style={{ fontSize: 11, letterSpacing: '.5px', color: R.ink3, marginBottom: 8 }}>RUN ON PRODUCTION DB</div>
            <div className="mono" style={{ fontSize: 13, background: R.paper3, borderRadius: 10, padding: '12px 12px', color: R.ink }}>psql &lt; 003_add_oauth.sql</div>
            <div className="flex" style={{ gap: 9, marginTop: 12 }}>
              <div className="flex-1 text-paper font-semibold flex items-center justify-center" style={{ height: 44, borderRadius: 12, background: R.accent, fontSize: 15 }}>Approve</div>
              <div className="flex-1 font-semibold flex items-center justify-center" style={{ height: 44, borderRadius: 12, background: R.paper3, color: R.ink2, fontSize: 15 }}>Deny</div>
            </div>
          </div>
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  APPROVE ACTION — full-screen gate for a destructive command.
// ═══════════════════════════════════════════════════════════════════
function Toggle({ on = true }) {
  return (
    <span className="relative shrink-0" style={{ width: 44, height: 26, borderRadius: 13, background: on ? R.jade : R.paper3, border: on ? 'none' : 'var(--hairline)' }}>
      <span className="absolute rounded-full bg-paper shadow-sm" style={{ top: 3, [on ? 'right' : 'left']: 3, width: 20, height: 20 }}/>
    </span>
  );
}

function RelayApprove({ dark = false }) {
  return (
    <IOSDevice dark={dark}>
      <Screen pad={56} style={{ paddingBottom: 30 }}>
        <div className="flex justify-end" style={{ padding: '8px 24px 4px' }}>
          <div className="border border-paper-edge flex items-center justify-center" style={{ width: 34, height: 34, borderRadius: 17, background: R.paper2 }}>
            <svg width="15" height="15" viewBox="0 0 15 15"><path d="M4 4l7 7M11 4l-7 7" stroke={R.ink3} strokeWidth="2" strokeLinecap="round"/></svg>
          </div>
        </div>
        <div className="flex-1 flex flex-col justify-center" style={{ padding: '0 24px' }}>
          <div className="flex items-center justify-center" style={{ width: 48, height: 48, borderRadius: 14, background: R.aSoft, marginBottom: 18 }}><Warn/></div>
          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 26, letterSpacing: '-.01em', lineHeight: 1.2, color: R.ink }}>The agent wants to run a command</div>
          <div style={{ fontSize: 14, color: R.ink2, marginTop: 8 }}>On <span className="mono" style={{ fontSize: 13 }}>mac-mini</span> · destructive · not reversible</div>
          <div className="mono" style={{ fontSize: 13.5, lineHeight: 1.6, background: R.ink, color: R.paper, borderRadius: 14, padding: 16, marginTop: 18 }}>rm -rf ./dist<br/>npm run build --prod</div>
          <div className="flex items-center justify-between border border-paper-edge" style={{ background: R.paper2, borderRadius: 14, padding: '16px 16px', marginTop: 12 }}>
            <span className="font-medium" style={{ fontSize: 14, color: R.ink }}>Dry-run in a sandbox first</span><Toggle on/>
          </div>
        </div>
        <div className="flex flex-col" style={{ padding: '16px 24px 0', gap: 10 }}>
          <div className="text-paper font-semibold flex items-center justify-center" style={{ height: 50, borderRadius: 14, background: R.accent, fontSize: 16 }}>Approve &amp; run</div>
          <div className="font-semibold flex items-center justify-center" style={{ height: 50, borderRadius: 14, background: R.paper3, color: R.ink2, fontSize: 16 }}>Reject</div>
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  RESPOND — quick-pick options + free reply (keyboard up).
// ═══════════════════════════════════════════════════════════════════
function RelayRespond({ dark = false }) {
  return (
    <IOSDevice dark={dark} keyboard>
      <Screen pad={56} style={{ height: '100%' }}>
        <div className="flex items-center" style={{ padding: '8px 24px 12px', gap: 11, borderBottom: '.5px solid var(--edge)' }}>
          <Backbtn/>
          <div><div className="font-semibold" style={{ fontSize: 15, color: R.ink }}>Refactor auth module</div><div className="mono" style={{ fontSize: 11, color: R.ink3 }}>macbook-pro · Claude</div></div>
        </div>
        <div className="flex-1 overflow-auto flex flex-col" style={{ padding: '16px 16px 8px', gap: 12 }}>
          <div className="self-start border border-paper-edge" style={{ maxWidth: '82%', background: R.paper2, borderRadius: '18px 18px 18px 5px', padding: '12px 16px', fontSize: 14, lineHeight: 1.45, color: R.ink }}>I need to pick a migration strategy. Which do you want?</div>
          <div className="flex flex-col" style={{ gap: 8 }}>
            <div className="border border-paper-edge font-semibold" style={{ background: R.paper2, borderRadius: 13, padding: '12px 16px', fontSize: 14, color: R.ink }}>Additive &amp; reversible <span className="font-normal" style={{ color: R.ink3 }}>· safest</span></div>
            <div className="border border-paper-edge font-semibold" style={{ background: R.paper2, borderRadius: 13, padding: '12px 16px', fontSize: 14, color: R.ink }}>In-place rewrite <span className="font-normal" style={{ color: R.ink3 }}>· faster</span></div>
          </div>
          <div className="self-end text-paper" style={{ maxWidth: '82%', background: R.accent, borderRadius: '18px 18px 5px 18px', padding: '12px 16px', fontSize: 14, lineHeight: 1.45 }}>Go additive. Keep the old columns for a week.</div>
          <div className="self-start border border-paper-edge" style={{ maxWidth: '82%', background: R.paper2, borderRadius: '18px 18px 18px 5px', padding: '12px 16px', fontSize: 14, lineHeight: 1.45, color: R.ink }}>Got it — resuming with an additive migration.</div>
        </div>
        <div style={{ padding: '8px 16px 8px' }}>
          <div className="flex items-center border border-paper-edge" style={{ gap: 10, background: R.paper2, borderRadius: 22, padding: '8px 8px 8px 16px' }}>
            <span className="flex-1" style={{ fontSize: 15, color: R.ink3 }}>Message the agent…</span>
            <div className="flex items-center justify-center" style={{ width: 32, height: 32, borderRadius: 16, background: R.accent }}><ArrowUp/></div>
          </div>
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  SECURITY — encrypted pairing + scoped permissions.
// ═══════════════════════════════════════════════════════════════════
function PermRow({ label, on, last }) {
  return (
    <div className="flex items-center justify-between" style={{ padding: '16px 16px', borderBottom: last ? 'none' : '.5px solid var(--edge)' }}>
      <span style={{ fontSize: 15, color: R.ink }}>{label}</span><Toggle on={on}/>
    </div>
  );
}

function RelaySecurity({ dark = false }) {
  return (
    <IOSDevice dark={dark}>
      <Screen pad={56}>
        <div className="flex items-center" style={{ padding: '8px 24px 0' }}><Backbtn/></div>
        <div style={{ padding: '12px 24px 8px' }}><div className="font-normal" style={{ fontFamily: R.disp, fontSize: 26, letterSpacing: '-.02em', color: R.ink }}>Security</div></div>
        <div className="flex-1 overflow-auto flex flex-col" style={{ padding: '12px 16px 32px', gap: 18 }}>

          <div style={{ background: R.ink, borderRadius: 20, padding: 18, color: R.paper }}>
            <div className="flex items-center" style={{ gap: 11 }}>
              <span className="flex items-center justify-center" style={{ width: 36, height: 36, borderRadius: 10, background: R.jadeSoft }}><Lock c={R.jade}/></span>
              <div><div className="mono" style={{ fontSize: 13, color: R.paper }}>macbook-pro</div><div style={{ fontSize: 12, color: R.jade, marginTop: 4 }}>Paired · end-to-end encrypted</div></div>
            </div>
            <div className="flex items-center flex-wrap" style={{ fontSize: 12.5, color: 'color-mix(in oklch, var(--paper) 70%, transparent)', marginTop: 14, gap: 8 }}>Your Mac<span style={{ opacity: .5 }}>→</span><Lock s={12} c={R.jade}/>relay<span style={{ opacity: .5 }}>→</span>iPhone</div>
            <div style={{ fontSize: 12, lineHeight: 1.5, color: 'color-mix(in oklch, var(--paper) 55%, transparent)', marginTop: 8 }}>Relay only sees the status you publish — never your code, keys, or files.</div>
          </div>

          <div>
            <div className="zs-eyebrow" style={{ padding: '0 8px 8px' }}>This phone can</div>
            <div className="border border-paper-edge overflow-hidden" style={{ background: R.paper2, borderRadius: 18 }}>
              <PermRow label="Approve commands" on/>
              <PermRow label="Send replies to the agent" on/>
              <PermRow label="Receive file contents" on={false} last/>
            </div>
          </div>

          <div>
            <div className="zs-eyebrow" style={{ padding: '0 8px 8px' }}>Notifications</div>
            <div className="border border-paper-edge" style={{ background: R.paper2, borderRadius: 18 }}><PermRow label="Push when an agent needs me" on last/></div>
          </div>

          <div className="font-semibold flex items-center justify-center" style={{ height: 48, borderRadius: 14, border: '1.5px dashed color-mix(in oklch, var(--accent) 50%, transparent)', color: R.accent, fontSize: 15, gap: 9 }}>
            <svg width="16" height="16" viewBox="0 0 16 16">{[[1,1],[9,1],[1,9]].map(([x,y],i)=><rect key={i} x={x} y={y} width="6" height="6" rx="1.5" stroke={R.accent} strokeWidth="1.6" fill="none"/>)}</svg>Pair another machine
          </div>
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  PAIRING — the onboarding round-trip: phone shows a code the daemon
//  reads back. Mirrors the "sensei relay pair" CLI moment.
// ═══════════════════════════════════════════════════════════════════
function RelayPairing({ dark = false }) {
  return (
    <IOSDevice dark={dark}>
      <Screen pad={64} style={{ paddingBottom: 30 }}>
        <div style={{ padding: '8px 24px 0' }}><RelayMark/></div>
        <div className="flex-1 flex flex-col justify-center text-center items-center" style={{ padding: '0 24px' }}>
          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 26, letterSpacing: '-.02em', lineHeight: 1.25, color: R.ink }}>Pair your Mac</div>
          <div style={{ fontSize: 14, color: R.ink2, marginTop: 8, lineHeight: 1.5 }}>On your machine, run the command below.<br/>Nothing leaves your Mac until it matches this code.</div>

          <div className="grid w-full" style={{ margin: '24px 0', gridTemplateColumns: 'repeat(3,1fr)', gap: 10 }}>
            {['7','2','9','4','1','6'].map((d, i) => (
              <div className="border border-paper-edge flex items-center justify-center" key={i} style={{ aspectRatio: '1', borderRadius: 16, background: R.paper2, fontFamily: R.disp, fontSize: 34, color: R.ink }}>{d}</div>
            ))}
          </div>

          <div className="mono w-full" style={{ fontSize: 13, background: R.ink, color: R.paper, borderRadius: 12, padding: '12px 16px', boxSizing: 'border-box' }}>$ sensei relay pair</div>
          <div className="flex items-center" style={{ fontSize: 12, color: R.ink3, marginTop: 14, gap: 8 }}><span className="rounded-full" style={{ width: 7, height: 7, background: R.jade, animation: 'relayPulse 2s infinite' }}/>Waiting for macbook-pro…</div>
        </div>
      </Screen>
    </IOSDevice>
  );
}

Object.assign(window, {
  RelayDashboard, RelayTaskDetail, RelayApprove, RelayRespond, RelaySecurity, RelayPairing,
});
