// ═══════════════════════════════════════════════════════════════════
//  SENSEI RELAY · PLANNER — the planning layer on top of Relay.
//  A project has ONE active plan. A plan has phases; each phase holds
//  features / checkpoints / gates. Development runs in non-blocking
//  auto mode while you're away; you watch progress (done · doing ·
//  next), get nudged when a track stalls, and answer decisions —
//  small sets of questions with 3–4 options plus a free reply, the way
//  sensei asks.
//  Reuses R / Screen / Backbtn / Check / Lock / IOSDevice from relay.jsx
//  (shared global scope). Colours resolve from tokens → theme-free.
// ═══════════════════════════════════════════════════════════════════

// ── small shared bits ────────────────────────────────────────
const P = {
  warn:     'var(--warning)',
  warnSoft: 'var(--warning-soft)',
};

function PBar({ pct, c = R.ink }) {
  return (
    <div className="overflow-hidden" style={{ height: 6, borderRadius: 3, background: 'color-mix(in oklch, var(--ink) 9%, transparent)' }}>
      <div className="h-full" style={{ width: pct + '%', background: c, borderRadius: 3 }}/>
    </div>
  );
}

// a compact done / doing / next status pill
function StatePill({ state }) {
  const map = {
    done:   ['done',        R.ink3, R.ink4],
    doing:  ['in progress', R.jade, R.jade],
    next:   ['next',        R.ink3, R.ink4],
    gate:   ['decision',    R.accent, R.accent],
    stall:  ['stalled',     P.warn, P.warn],
  };
  const [label, txt, dot] = map[state];
  return (
    <span className="mono uppercase inline-flex items-center" style={{ fontSize: 10.5, letterSpacing: '.4px', color: txt, gap: 6 }}>
      <span className="rounded-full" style={{ width: 6, height: 6, background: dot, animation: state === 'doing' ? 'relayPulse 2s infinite' : 'none' }}/>{label}
    </span>
  );
}

const AwayPill = () => (
  <div className="inline-flex items-center border border-paper-edge" style={{ gap: 8, background: R.paper2, borderRadius: 999, padding: '8px 16px 8px 12px', fontSize: 12.5, color: R.ink2 }}>
    <span className="rounded-full" style={{ width: 7, height: 7, background: R.jade, animation: 'relayPulse 2s infinite' }}/>Auto mode · working while you’re away
  </div>
);

// ═══════════════════════════════════════════════════════════════════
//  ① PROJECTS — every active project, its one plan, phase progress,
//     what's running now, and whether a track needs you.
// ═══════════════════════════════════════════════════════════════════
function RelayProjects({ dark = false }) {
  const projects = [
    { name: 'lumen-auth',  machines: '2 machines', plan: 'OAuth & session hardening', phase: 2, of: 4, pct: 47, now: 'refresh-token store', flag: 'gate',  note: '1 decision waiting' },
    { name: 'billing-svc', machines: '1 machine',  plan: 'Invoicing v2',              phase: 3, of: 5, pct: 61, now: 'webhook retry tests', flag: 'doing', note: 'on track' },
    { name: 'telemetry',   machines: '1 machine',  plan: 'Event pipeline rewrite',    phase: 1, of: 3, pct: 18, now: 'ingest schema',        flag: 'stall', note: 'quiet for 22m' },
  ];
  const flagCol = { gate: R.accent, doing: R.jade, stall: P.warn };
  return (
    <IOSDevice dark={dark}>
      <Screen pad={60}>
        <div style={{ padding: '8px 24px 16px' }}>
          <div className="flex items-center justify-between" >
            <RelayMark/>
            <div className="border border-paper-edge flex items-center justify-center" style={{ width: 34, height: 34, borderRadius: 17, background: R.paper2 }}><Lock/></div>
          </div>
          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 32, letterSpacing: '-.02em', marginTop: 14, color: R.ink }}>Projects</div>
          <div style={{ marginTop: 14 }}><AwayPill/></div>
        </div>
        <div className="flex-1 overflow-auto flex flex-col" style={{ padding: '4px 16px 32px', gap: 12 }}>
          {projects.map((p, i) => {
            const c = flagCol[p.flag];
            const attn = p.flag === 'gate' || p.flag === 'stall';
            return (
              <div key={i} style={{
                background: attn ? (p.flag === 'gate' ? R.aSoft : P.warnSoft) : R.paper2,
                border: attn ? `1px solid color-mix(in oklch, ${c} 32%, transparent)` : 'var(--hairline)',
                borderRadius: 20, padding: '16px 16px',
              }}>
                <div className="flex items-center justify-between" >
                  <span className="inline-flex items-center" style={{ gap: 8 }}>
                    <span className="kanji" style={{ fontSize: 15, color: R.ink3 }}>場</span>
                    <span className="mono" style={{ fontSize: 13, color: R.ink }}>{p.name}</span>
                  </span>
                  <span className="mono" style={{ fontSize: 11, color: R.ink3 }}>{p.machines}</span>
                </div>
                <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 19, letterSpacing: '-.01em', color: R.ink, margin: '12px 0 12px' }}>{p.plan}</div>
                <div className="flex items-center justify-between" style={{ marginBottom: 8 }}>
                  <span className="mono" style={{ fontSize: 11.5, color: R.ink2 }}>Phase {p.phase} of {p.of}</span>
                  <span className="mono" style={{ fontSize: 11.5, color: R.ink2 }}>{p.pct}%</span>
                </div>
                <PBar pct={p.pct} c={p.flag === 'stall' ? P.warn : R.ink}/>
                <div className="flex items-center justify-between" style={{ marginTop: 13 }}>
                  <span className="inline-flex items-center" style={{ fontSize: 13, color: p.flag === 'doing' ? R.ink2 : R.ink, gap: 7 }}>
                    <span className="rounded-full shrink-0" style={{ width: 6, height: 6, background: c, animation: p.flag === 'doing' ? 'relayPulse 2s infinite' : 'none' }}/>
                    {p.flag === 'stall' ? 'Paused · ' : 'Now · '}{p.now}
                  </span>
                  <span className="font-semibold whitespace-nowrap" style={{ fontSize: 11.5, color: c }}>{p.note}</span>
                </div>
              </div>
            );
          })}
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  ② PLAN — one project's plan. Phases stacked; each carries its
//     features / checkpoints / gates. Done · doing · next at a glance.
// ═══════════════════════════════════════════════════════════════════
function PlanItem({ label, kind, note, last }) {
  // kind: done | doing | gate | next
  const done = kind === 'done', doing = kind === 'doing', gate = kind === 'gate';
  const ring = done ? R.ink : gate ? R.accent : doing ? R.jade : 'color-mix(in oklch, var(--ink) 16%, transparent)';
  return (
    <div className="flex items-start" style={{ gap: 12, padding: '12px 4px', borderBottom: last ? 'none' : '.5px solid var(--edge)' }}>
      <span className="shrink-0 flex items-center justify-center" style={{ width: 20, height: 20, marginTop: 1, borderRadius: gate ? 5 : '50%', background: done ? R.ink : gate ? R.aSoft : 'transparent', border: done ? 'none' : `2px solid ${ring}` }}>
        {done && <Check s={11}/>}
        {gate && <Lock s={10} c={R.accent}/>}
        {doing && <span className="rounded-full" style={{ width: 7, height: 7, background: R.jade, animation: 'relayPulse 2s infinite' }}/>}
      </span>
      <div className="flex-1" >
        <div style={{ fontSize: 14.5, fontWeight: gate || doing ? 600 : 400, color: done ? R.ink3 : R.ink }}>{label}</div>
        {note && <div className="font-medium" style={{ fontSize: 12, color: gate ? R.accent : doing ? R.jade : R.ink3, marginTop: 2 }}>{note}</div>}
      </div>
    </div>
  );
}

function PlanPhase({ n, name, state, count, items, open }) {
  const stateCol = { done: R.ink4, doing: R.jade, next: R.ink4 };
  return (
    <div className="border border-paper-edge overflow-hidden" style={{ background: R.paper2, borderRadius: 18 }}>
      <div className="flex items-center" style={{ gap: 12, padding: '16px 16px', borderBottom: open ? '.5px solid var(--edge)' : 'none' }}>
        <span className="mono" style={{ fontSize: 12, color: R.ink3, width: 20 }}>{n}</span>
        <div className="flex-1" >
          <div className="font-semibold" style={{ fontSize: 15, color: state === 'next' ? R.ink2 : R.ink }}>{name}</div>
        </div>
        <span className="mono" style={{ fontSize: 11, color: R.ink3 }}>{count}</span>
        <StatePill state={state}/>
      </div>
      {open && <div style={{ padding: '4px 16px 8px' }}>{items.map((it, i) => <PlanItem key={i} {...it} last={i === items.length - 1}/>)}</div>}
    </div>
  );
}

function RelayPlan({ dark = false }) {
  return (
    <IOSDevice dark={dark}>
      <Screen pad={56}>
        <div className="flex items-center justify-between" style={{ padding: '8px 24px 0' }}>
          <Backbtn/>
          <span className="mono" style={{ fontSize: 12, color: R.ink3 }}>lumen-auth · auto</span>
        </div>
        <div style={{ padding: '12px 24px 8px' }}>
          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 23, letterSpacing: '-.01em', color: R.ink }}>OAuth & session hardening</div>
          <div className="flex items-center justify-between" style={{ margin: '16px 0 8px' }}>
            <span className="mono" style={{ fontSize: 12, color: R.ink2 }}>Phase 2 of 4</span>
            <span className="mono" style={{ fontSize: 12, color: R.ink2 }}>47%</span>
          </div>
          <PBar pct={47}/>
        </div>
        <div className="flex-1 overflow-auto flex flex-col" style={{ padding: '12px 16px 32px', gap: 12 }}>
          <PlanPhase n="01" name="Foundation" state="done" count="2 / 2" open={false}/>
          <PlanPhase n="02" name="Session hardening" state="doing" count="1 / 3" open items={[
            { label: 'Rotate signing keys', kind: 'done' },
            { label: 'Refresh-token store', kind: 'doing', note: 'writing store.ts · 14:08' },
            { label: 'Session strategy', kind: 'gate', note: 'decision waiting — needs you' },
          ]}/>
          <PlanPhase n="03" name="Rollout" state="next" count="0 / 2" open items={[
            { label: 'Feature flag behind auth_v2', kind: 'next' },
            { label: 'Gradual 5% → 100%', kind: 'next', last: true },
          ]}/>
          <PlanPhase n="04" name="Cleanup" state="next" count="0 / 1" open={false}/>
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  ③ DECISIONS — non-blocking questions sensei surfaces to move a
//     track ahead. 3–4 options plus a free reply. Other tracks keep
//     running meanwhile.
// ═══════════════════════════════════════════════════════════════════
function DecisionQ({ eyebrow, q, options, chosen, freehand }) {
  return (
    <div className="border border-paper-edge" style={{ background: R.paper2, borderRadius: 18, padding: '16px 16px' }}>
      <div className="zs-eyebrow" style={{ marginBottom: 9 }}>{eyebrow}</div>
      <div className="font-semibold" style={{ fontSize: 15.5, lineHeight: 1.35, color: R.ink, marginBottom: 13 }}>{q}</div>
      <div className="flex flex-col" style={{ gap: 8 }}>
        {options.map((o, i) => {
          const sel = i === chosen;
          return (
            <div className="flex items-center" key={i} style={{ gap: 11, background: sel ? R.aSoft : R.paper, border: sel ? '1px solid color-mix(in oklch, var(--accent) 45%, transparent)' : 'var(--hairline)', borderRadius: 12, padding: '12px 16px' }}>
              <span className="rounded-full shrink-0 flex items-center justify-center" style={{ width: 18, height: 18, border: sel ? 'none' : '2px solid color-mix(in oklch, var(--ink) 20%, transparent)', background: sel ? R.accent : 'transparent' }}>{sel && <Check s={10}/>}</span>
              <span style={{ fontSize: 14, color: R.ink }}><span style={{ fontWeight: sel ? 600 : 500 }}>{o[0]}</span>{o[1] && <span className="font-normal" style={{ color: R.ink3 }}> · {o[1]}</span>}</span>
            </div>
          );
        })}
      </div>
      <div className="flex items-center" style={{ gap: 10, margin: '12px 4px' }}>
        <span className="flex-1" style={{ height: 1, background: 'var(--edge)' }}/><span className="mono" style={{ fontSize: 10.5, color: R.ink4, letterSpacing: '.5px' }}>OR</span><span className="flex-1" style={{ height: 1, background: 'var(--edge)' }}/>
      </div>
      <div className="border border-paper-edge" style={{ background: R.paper, borderRadius: 12, padding: '12px 16px', fontSize: 13.5, color: freehand ? R.ink : R.ink3, lineHeight: 1.4 }}>{freehand || 'Type your answer…'}</div>
    </div>
  );
}

function RelayDecisions({ dark = false }) {
  return (
    <IOSDevice dark={dark}>
      <Screen pad={56}>
        <div className="flex items-center justify-between" style={{ padding: '8px 24px 0' }}>
          <Backbtn/>
          <span className="mono" style={{ fontSize: 12, color: R.accent }}>2 waiting</span>
        </div>
        <div style={{ padding: '12px 24px 4px' }}>
          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 24, letterSpacing: '-.01em', color: R.ink }}>Decisions</div>
          <div style={{ fontSize: 13, color: R.ink2, marginTop: 6 }}>Answer when you can — other tracks keep moving.</div>
        </div>
        <div className="flex-1 overflow-auto flex flex-col" style={{ padding: '16px 16px 8px', gap: 12 }}>
          <DecisionQ
            eyebrow="lumen-auth · phase 2"
            q="Which session strategy should sensei use?"
            options={[['Stateless JWT', 'simplest'], ['Server sessions', 'revocable'], ['Hybrid — JWT + refresh store', 'recommended']]}
            chosen={2}/>
          <DecisionQ
            eyebrow="billing-svc · phase 3"
            q="Roll invoicing v2 out to which cohort first?"
            options={[['Internal only', ''], ['5% of users', ''], ['Beta list', '42 accounts']]}
            chosen={-1}
            freehand={null}/>
        </div>
        <div style={{ padding: '12px 16px 32px', background: 'linear-gradient(180deg, transparent, var(--paper) 30%)' }}>
          <div className="text-paper font-semibold flex items-center justify-center" style={{ height: 50, borderRadius: 14, background: R.accent, fontSize: 16, gap: 8 }}>
            Send 1 answer<span className="font-normal" style={{ opacity: .75, fontSize: 13 }}>· 1 left</span>
          </div>
        </div>
      </Screen>
    </IOSDevice>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  ④ NUDGE — a track has gone quiet. Sensei shows what's done / doing
//     / next and offers a nudge to continue or a check.
// ═══════════════════════════════════════════════════════════════════
function MiniStep({ kind, label, note, last }) {
  const done = kind === 'done', doing = kind === 'doing';
  const c = done ? R.ink4 : doing ? P.warn : 'color-mix(in oklch, var(--ink) 16%, transparent)';
  return (
    <div className="flex items-start" style={{ gap: 12, padding: '12px 0', borderBottom: last ? 'none' : '.5px solid var(--edge)' }}>
      <span className="rounded-full shrink-0 flex items-center justify-center" style={{ width: 18, height: 18, marginTop: 1, background: done ? R.ink : 'transparent', border: done ? 'none' : `2px solid ${c}` }}>{done && <Check s={10}/>}</span>
      <div className="flex-1" >
        <div style={{ fontSize: 14.5, color: done ? R.ink3 : R.ink, fontWeight: doing ? 600 : 400 }}>{label}</div>
        {note && <div style={{ fontSize: 12, color: doing ? P.warn : R.ink3, marginTop: 2 }}>{note}</div>}
      </div>
    </div>
  );
}

function RelayNudge({ dark = false }) {
  return (
    <IOSDevice dark={dark}>
      <Screen pad={56}>
        <div className="flex items-center justify-between" style={{ padding: '8px 24px 0' }}>
          <Backbtn/>
          <span className="mono" style={{ fontSize: 12, color: R.ink3 }}>telemetry · auto</span>
        </div>
        <div className="flex-1 overflow-auto" style={{ padding: '16px 24px 24px' }}>
          {/* stalled banner */}
          <div style={{ background: P.warnSoft, border: `1px solid color-mix(in oklch, ${P.warn} 32%, transparent)`, borderRadius: 18, padding: '16px 16px' }}>
            <div className="flex items-center" style={{ gap: 10 }}>
              <span className="flex items-center justify-center shrink-0" style={{ width: 34, height: 34, borderRadius: 10, background: 'color-mix(in oklch, var(--warning) 18%, transparent)' }}>
                <span className="kanji" style={{ fontSize: 17, color: P.warn }}>静</span>
              </span>
              <div>
                <div className="font-semibold" style={{ fontSize: 15.5, color: R.ink }}>This track has gone quiet</div>
                <div style={{ fontSize: 12.5, color: R.ink2, marginTop: 2 }}>No activity for 22m · waiting on API rate limit</div>
              </div>
            </div>
            <div style={{ fontSize: 12.5, color: R.ink3, marginTop: 12 }}>sensei will retry on its own in ~8m. You can nudge it now.</div>
          </div>

          <div className="font-normal" style={{ fontFamily: R.disp, fontSize: 20, letterSpacing: '-.01em', color: R.ink, margin: '24px 0 4px' }}>Event pipeline rewrite</div>
          <div className="mono" style={{ fontSize: 11.5, color: R.ink3, marginBottom: 12 }}>Phase 1 of 3 · 18%</div>

          <div className="zs-eyebrow" style={{ marginBottom: 4 }}>Where it stands</div>
          <MiniStep kind="done" label="Map current event schema"/>
          <MiniStep kind="doing" label="Draft ingest schema" note="paused mid-write · migrations/ingest.sql"/>
          <MiniStep kind="next" label="Backfill last 30 days" last/>
        </div>

        <div className="flex flex-col" style={{ padding: '12px 24px 32px', gap: 10 }}>
          <div className="text-paper font-semibold flex items-center justify-center" style={{ height: 50, borderRadius: 14, background: R.accent, fontSize: 16 }}>Nudge to continue</div>
          <div className="font-semibold flex items-center justify-center" style={{ height: 50, borderRadius: 14, background: R.paper3, color: R.ink2, fontSize: 16 }}>View logs</div>
        </div>
      </Screen>
    </IOSDevice>
  );
}

Object.assign(window, {
  RelayProjects, RelayPlan, RelayDecisions, RelayNudge,
});
