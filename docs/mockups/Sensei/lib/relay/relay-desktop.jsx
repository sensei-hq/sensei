// ═══════════════════════════════════════════════════════════════════
//  SENSEI RELAY — desktop surfaces (Observatory), architecture, Dōjō
//  These extend the existing Sensei ecosystem rather than sitting apart:
//    · Observatory gains a "Relay" rail item — the coordinator that
//      supervises agents and publishes the filtered status the phone
//      reads. Reuses sessions / projects vocabulary.
//    · The architecture explainer, re-skinned into Zen-Sumi.
//    · A Dōjō angle — relay gates fanned across a shared team.
//  Tokens only; geometry inline. All under .artboard-shell scope.
// ═══════════════════════════════════════════════════════════════════

const rc = (t) => `var(--${t})`;

// small shared glyphs (desktop scale)
const DLock = ({ s = 15, c = 'var(--ink-2)' }) => (
  <svg width={s} height={s * 17 / 15} viewBox="0 0 15 17" fill="none"><rect x="1.5" y="7" width="12" height="9" rx="2.5" stroke={c} strokeWidth="1.5"/><path d="M4 7V5a3.5 3.5 0 0 1 7 0v2" stroke={c} strokeWidth="1.5"/></svg>
);
const Dot = ({ c, pulse }) => <span className="rounded-full shrink-0" style={{ width: 7, height: 7, background: c, animation: pulse ? 'relayPulse 2s infinite' : 'none' }}/>;

// ── Relay-scoped window shell (Observatory chrome + rail) ─────
function RelayWindow({ active = 'relay', children }) {
  const nav = [
    ['家', 'Today', null],
    ['場', 'Projects', null],
    ['信', 'Relay', 'relay'],
    ['今', 'Insights', null],
    ['記', 'Sessions', null],
    ['具', 'Instruments', null],
  ];
  return (
    <div className="artboard-shell w-full h-full flex flex-col overflow-hidden" style={{ background: rc('paper') }}>
      <TauriChrome title="Sensei  先生  ·  observatory"/>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '240px 1fr' }}>
        <aside className="py-6 px-3 gap-4 border-r flex flex-col" style={{ background: rc('paper-2') }}>
          <div className="px-1 font-normal flex items-baseline" style={{ fontFamily: rc('font-display'), fontSize: 22, letterSpacing: '-.02em', gap: 8 }}>Sensei <span className="kanji" style={{ fontSize: 14, color: rc('ink-3') }}>先生</span></div>
          <div>
            <div className="pt-0 pb-2 px-2 uppercase" style={{ fontSize: 11, letterSpacing: '.16em', color: rc('ink-3') }}>Observatory</div>
            <div className="gap-1 flex flex-col" >
              {nav.map(([k, l, id]) => {
                const on = id === active;
                return (
                  <div key={l} className="gap-2 py-2 px-2 grid items-center" style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 6, background: on ? rc('paper') : 'transparent', border: on ? 'var(--hairline)' : '1px solid transparent', color: on ? rc('ink') : rc('ink-2'), fontSize: 13 }}>
                    <span className="kanji" style={{ fontSize: 13, width: 14, color: on ? rc('accent') : rc('ink-3') }}>{k}</span>
                    <span className="inline-flex items-center" style={{ gap: 6 }}>{l}{id === 'relay' && <Dot c={rc('accent')}/>}</span>
                    {id === 'relay' && <span className="mono" style={{ fontSize: 11, color: rc('accent') }}>1</span>}
                  </div>
                );
              })}
            </div>
          </div>
          <div style={{ marginTop: 'auto' }} className="gap-2 p-3">
            <div className="px-1 pb-2 uppercase" style={{ fontSize: 11, letterSpacing: '.16em', color: rc('ink-3') }}>Paired</div>
            {[['macbook-pro', true], ['mac-mini', true], ['iPhone 16 Pro', true]].map(([n]) => (
              <div key={n} className="px-1 flex items-center" style={{ gap: 8, fontSize: 12, color: rc('ink-2'), padding: '4px 4px' }}><Dot c={rc('success')}/><span className="mono" style={{ fontSize: 12 }}>{n}</span></div>
            ))}
          </div>
        </aside>
        <main className="min-h-0 overflow-auto" style={{ background: rc('paper') }}>{children}</main>
      </div>
    </div>
  );
}

const REB = ({ children }) => <div className="zs-eyebrow" style={{ marginBottom: 8 }}>{children}</div>;

// ═══════════════════════════════════════════════════════════════════
//  COORDINATOR — the desktop hub: paired devices, the live published
//  event stream, and pending gates the phone can answer.
// ═══════════════════════════════════════════════════════════════════
function RelayCoordinator() {
  const events = [
    ['14:10', 'macbook-pro', 'auth · gate opened', 'Approve DB migration on production', 'gate'],
    ['14:08', 'macbook-pro', 'auth · test', 'Ran suite — 18 passed', 'ok'],
    ['14:05', 'macbook-pro', 'auth · edit', 'auth.ts +42 −8', 'edit'],
    ['14:03', 'mac-mini', 'tests · run', 'Writing test_checkout.py', 'live'],
    ['14:01', 'mac-mini', 'backfill · progress', 'Batch 42 / 120', 'live'],
  ];
  const tone = { gate: rc('accent'), ok: rc('success'), live: rc('ink-3'), edit: rc('ink-2') };
  return (
    <RelayWindow>
      <div className="p-8 flex flex-col" style={{ gap: 24 }}>
        {/* header */}
        <div className="flex items-end justify-between" >
          <div>
            <REB>信 · relay coordinator</REB>
            <div className="font-normal" style={{ fontFamily: rc('font-display'), fontSize: 28, letterSpacing: '-.02em' }}>What your agents are relaying</div>
            <div className="zs-body-sm" style={{ marginTop: 6, maxWidth: 620 }}>The coordinator daemon watches every agent CLI on this machine, filters raw activity into safe events, and publishes only those — plus the moments a human is needed — to your paired phones.</div>
          </div>
          <div className="flex items-center" style={{ gap: 8 }}>
            <span className="zs-badge zs-badge-success"><Dot c={rc('success')} pulse/> relay live</span>
            <button className="zs-btn zs-btn-secondary zs-btn-sm">Pause publishing</button>
          </div>
        </div>

        {/* stat strip */}
        <div className="grid" style={{ gridTemplateColumns: 'repeat(4,1fr)', gap: 16 }}>
          {[['1', 'pending gate', rc('accent')], ['3', 'agents running', rc('success')], ['2', 'machines', rc('ink')], ['2', 'phones paired', rc('ink')]].map(([n, l, c]) => (
            <div key={l} className="p-4 rounded-lg border border-paper-edge bg-paper-soft">
              <div className="font-normal" style={{ fontFamily: rc('font-display'), fontSize: 30, letterSpacing: '-.03em', color: c }}>{n}</div>
              <div className="text-sm text-ink-mute" style={{ marginTop: 2 }}>{l}</div>
            </div>
          ))}
        </div>

        <div className="grid" style={{ gridTemplateColumns: '1.4fr 1fr', gap: 20 }}>
          {/* event stream */}
          <div>
            <REB>Published stream</REB>
            <div className="rounded-lg border border-paper-edge bg-paper-soft overflow-hidden" >
              {events.map(([t, m, k, d, s], i) => (
                <div className="grid items-center" key={i} style={{ gridTemplateColumns: 'auto auto 1fr auto', gap: 12, padding: '16px 16px', borderBottom: i < events.length - 1 ? 'var(--hairline)' : 'none', background: s === 'gate' ? rc('accent-soft') : 'transparent' }}>
                  <span className="mono text-xs text-ink-mute">{t}</span>
                  <Dot c={tone[s]} pulse={s === 'live'}/>
                  <div className="min-w-0" >
                    <div className="mono" style={{ fontSize: 12, color: rc('ink-3') }}>{m} · {k}</div>
                    <div className="text-sm" style={{ marginTop: 2, color: s === 'gate' ? rc('accent') : rc('ink'), fontWeight: s === 'gate' ? 600 : 400 }}>{d}</div>
                  </div>
                  {s === 'gate' && <span className="zs-badge zs-badge-accent">needs you</span>}
                </div>
              ))}
            </div>
            <div className="text-xs text-ink-faint mono" style={{ marginTop: 10 }}>Redacted before publish: file contents · secrets · full diffs · shell output</div>
          </div>

          {/* pending gate detail */}
          <div>
            <REB>Pending gate</REB>
            <div className="rounded-lg border border-paper-edge p-4" style={{ background: rc('accent-soft'), borderColor: 'color-mix(in oklch, var(--accent) 30%, transparent)' }}>
              <div className="flex items-center" style={{ gap: 8 }}><DLock c={rc('accent')}/><span className="mono text-xs" style={{ color: rc('accent') }}>macbook-pro · auth</span></div>
              <div className="text-lg font-semibold" style={{ marginTop: 10 }}>Approve database migration</div>
              <div className="mono text-sm" style={{ background: rc('ink'), color: rc('paper'), borderRadius: 8, padding: '12px 12px', marginTop: 12 }}>psql &lt; 003_add_oauth.sql</div>
              <div className="text-sm text-ink-soft" style={{ marginTop: 12, lineHeight: 1.5 }}>Relayed to <b>iPhone 16 Pro</b> 40s ago · awaiting your decision there, or answer here.</div>
              <div className="flex" style={{ gap: 8, marginTop: 14 }}>
                <button className="zs-btn zs-btn-primary flex-1 justify-center" style={{ background: rc('accent') }}>Approve</button>
                <button className="zs-btn zs-btn-secondary flex-1 justify-center" >Deny</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </RelayWindow>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  PLAN AUTHORING — define a task's plan and mark which steps must
//  gate to a human. This is what drives the phone's stage checklist.
// ═══════════════════════════════════════════════════════════════════
function RelayPlanAuthor() {
  const steps = [
    ['Analyze current auth flow', 'auto', null],
    ['Draft OAuth schema', 'auto', null],
    ['Migrate user model', 'auto', null],
    ['Apply DB migration', 'gate', 'command touches production'],
    ['Update tests & ship', 'gate', 'opens a pull request'],
  ];
  return (
    <RelayWindow>
      <div className="p-8 flex flex-col" style={{ gap: 22, maxWidth: 900 }}>
        <div>
          <REB>信 · relay · plan</REB>
          <div className="font-normal" style={{ fontFamily: rc('font-display'), fontSize: 28, letterSpacing: '-.02em' }}>Refactor auth module</div>
          <div className="zs-body-sm" style={{ marginTop: 6, maxWidth: 640 }}>Sensei drafts the plan with the agent; you decide which steps run freely and which must stop for you. Gated steps are exactly what your phone will surface while you're away.</div>
        </div>

        <div className="rounded-lg border border-paper-edge bg-paper-soft overflow-hidden" >
          {steps.map(([label, mode, why], i) => {
            const gate = mode === 'gate';
            return (
              <div className="grid items-center" key={i} style={{ gridTemplateColumns: 'auto 1fr auto', gap: 14, padding: '16px 16px', borderBottom: i < steps.length - 1 ? 'var(--hairline)' : 'none' }}>
                <span className="mono text-xs text-ink-faint" style={{ width: 18 }}>{String(i + 1).padStart(2, '0')}</span>
                <div>
                  <div className="text-base" style={{ fontWeight: gate ? 600 : 400 }}>{label}</div>
                  {gate && <div className="text-sm text-accent" style={{ marginTop: 2 }}>gate · {why}</div>}
                </div>
                <div className="flex" style={{ background: rc('paper-3'), borderRadius: 6, padding: 2 }}>
                  {[['auto', 'Auto'], ['gate', 'Gate']].map(([v, l]) => {
                    const on = v === mode;
                    return <div key={v} style={{ fontSize: 12, padding: '4px 12px', borderRadius: 4, background: on ? rc('paper') : 'transparent', color: on ? (v === 'gate' ? rc('accent') : rc('ink')) : rc('ink-3'), fontWeight: on ? 600 : 400 }}>{l}</div>;
                  })}
                </div>
              </div>
            );
          })}
        </div>

        <div className="flex items-center" style={{ gap: 12 }}>
          <button className="zs-btn zs-btn-primary">Start with these gates</button>
          <span className="text-sm text-ink-mute">2 of 5 steps will ask for you · pushed to iPhone 16 Pro</span>
        </div>
      </div>
    </RelayWindow>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  ARCHITECTURE — three planes + the human-in-the-loop round-trip.
// ═══════════════════════════════════════════════════════════════════
function Plane({ kanji, title, sub, tone, children }) {
  return (
    <div className="rounded-lg border border-paper-edge p-6 flex-1 min-w-0 flex flex-col" style={{ background: rc('paper-soft'), gap: 14, borderColor: tone ? `color-mix(in oklch, ${tone} 24%, var(--edge))` : rc('edge') }}>
      <div className="flex items-center" style={{ gap: 10 }}>
        <span className="kanji" style={{ fontSize: 22, color: tone || rc('ink-2'), width: 26 }}>{kanji}</span>
        <div><div className="text-base font-semibold">{title}</div><div className="text-sm text-ink-mute">{sub}</div></div>
      </div>
      {children}
    </div>
  );
}
function MiniCard({ title, meta, tone }) {
  return (
    <div className="rounded border border-paper-edge p-3 flex items-center min-w-0" style={{ background: rc('paper'), gap: 10 }}>
      <Dot c={tone || rc('ink-3')} pulse={!!tone}/>
      <div className="min-w-0" ><div className="text-sm font-medium whitespace-nowrap overflow-hidden text-ellipsis" >{title}</div><div className="mono text-xs text-ink-mute">{meta}</div></div>
    </div>
  );
}

function RelayArchitecture() {
  const round = [
    ['1', 'Agent hits a gate', 'Claude Code pauses on an approval, question, or account limit.'],
    ['2', 'Coordinator filters', 'Redacts code + secrets; keeps only the decision and its context.'],
    ['3', 'Relay forwards', 'A zero-knowledge relay passes the encrypted blob — it can read none of it.'],
    ['4', 'Phone notifies you', 'Sensei Relay surfaces the gate; you approve, answer, or reject.'],
    ['5', 'Answer travels back', 'The signed decision returns the same path; the agent resumes.'],
  ];
  return (
    <div className="artboard-shell w-full h-full overflow-visible" style={{ background: rc('paper') }}>
      <div className="p-8 flex flex-col" style={{ gap: 24 }}>
        <div>
          <REB>信 · relay architecture</REB>
          <div className="font-light" style={{ fontFamily: rc('font-display'), fontSize: 30, letterSpacing: '-.02em', maxWidth: 720, lineHeight: 1.1 }}>Your agents stay on your machine. Only what needs you leaves it.</div>
          <div className="zs-body-sm" style={{ marginTop: 10, maxWidth: 680 }}>Three planes, one quiet contract. The desktop coordinator is the only thing that touches your code; the relay is deliberately blind; the phone holds just enough to make a decision.</div>
        </div>

        {/* three planes */}
        <div className="flex items-stretch" style={{ gap: 20 }}>
          <Plane kanji="機" title="Your machines" sub="agents + coordinator" tone={rc('success')}>
            <MiniCard title="Claude Code · auth" meta="macbook-pro · gated" tone={rc('accent')}/>
            <MiniCard title="Codex · tests" meta="mac-mini · running" tone={rc('success')}/>
            <MiniCard title="coordinator daemon" meta="filters → publishes"/>
          </Plane>
          <div className="flex flex-col justify-center" style={{ color: rc('ink-4'), fontSize: 20 }}>→</div>
          <Plane kanji="鍵" title="Zero-knowledge relay" sub="end-to-end encrypted" tone={rc('ink-2')}>
            <div className="rounded border border-paper-edge p-3 mono text-xs overflow-x-hidden" style={{ background: rc('paper'), color: rc('ink-3'), lineHeight: 1.7 }}>
              <div>{'{'} event: "gate",</div>
              <div style={{ paddingLeft: 12 }}>machine: "macbook-pro",</div>
              <div style={{ paddingLeft: 12 }}>kind: "run_command",</div>
              <div style={{ paddingLeft: 12 }}>ciphertext: "▓▓▓▓▓▓" {'}'}</div>
            </div>
            <div className="text-sm text-ink-mute" style={{ lineHeight: 1.5 }}>The relay routes encrypted blobs by device key. It never holds a key that can open one.</div>
          </Plane>
          <div className="flex flex-col justify-center" style={{ color: rc('ink-4'), fontSize: 20 }}>→</div>
          <Plane kanji="携" title="Your phone" sub="Sensei Relay" tone={rc('accent')}>
            <MiniCard title="Approve DB migration" meta="tap to decide" tone={rc('accent')}/>
            <MiniCard title="18 tests passed" meta="mac-mini · 2m ago"/>
            <div className="text-sm text-ink-mute" style={{ lineHeight: 1.5 }}>Decrypts locally. Sends back a signed yes / no / reply.</div>
          </Plane>
        </div>

        {/* round trip */}
        <div>
          <REB>The round-trip</REB>
          <div className="grid" style={{ gridTemplateColumns: 'repeat(5,1fr)', gap: 14 }}>
            {round.map(([n, t, d], i) => (
              <div key={i} className="rounded-lg border border-paper-edge p-4 bg-paper-soft flex flex-col relative" style={{ gap: 8 }}>
                <span className="mono font-semibold" style={{ fontSize: 12, color: rc('accent') }}>{n}</span>
                <div className="text-sm font-semibold">{t}</div>
                <div className="text-sm text-ink-mute" style={{ lineHeight: 1.45 }}>{d}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════
//  DŌJŌ ANGLE — relay gates fanned across a shared team. Gates can be
//  routed to whoever is on call; decisions carry attribution.
// ═══════════════════════════════════════════════════════════════════
function DojoRelayGates() {
  const gates = [
    ['Approve prod migration', 'lumen-auth', 'macbook-pro · Aiko', 'Aiko N.', 'oncall', 'accent'],
    ['Merge to main', 'billing-svc', 'ci-runner · Codex', 'Rai T.', 'routed', 'accent'],
    ['Re-auth Anthropic limit', 'lumen-auth', 'mac-mini · Claude', null, 'open', 'accent'],
    ['Rotate staging secret', 'infra', 'bastion · Aider', 'Mei L.', 'approved', 'success'],
  ];
  return (
    <div className="artboard-shell w-full h-full overflow-auto" style={{ background: rc('paper') }}>
      <div className="p-12 flex flex-col" style={{ gap: 24 }}>
        <div className="flex items-end justify-between" >
          <div>
            <REB>群 · dōjō · shared relay</REB>
            <div className="font-normal" style={{ fontFamily: rc('font-display'), fontSize: 30, letterSpacing: '-.02em' }}>Gates across the team</div>
            <div className="zs-body-sm" style={{ marginTop: 6, maxWidth: 660 }}>When agents run for the whole org, their human-in-the-loop moments fan into one shared queue. Route a gate to whoever's on call; every decision carries attribution and lands back in the Dōjō record.</div>
          </div>
          <span className="zs-badge zs-badge-accent"><Dot c={rc('accent')}/> 3 open</span>
        </div>

        <div className="grid" style={{ gridTemplateColumns: 'repeat(3,1fr)', gap: 16 }}>
          {[['3', 'awaiting', rc('accent')], ['12', 'resolved today', rc('ink')], ['1.4m', 'median to answer', rc('success')]].map(([n, l, c]) => (
            <div key={l} className="p-4 rounded-lg border border-paper-edge bg-paper-soft"><div className="font-normal" style={{ fontFamily: rc('font-display'), fontSize: 28, letterSpacing: '-.03em', color: c }}>{n}</div><div className="text-sm text-ink-mute" style={{ marginTop: 2 }}>{l}</div></div>
          ))}
        </div>

        <div className="rounded-lg border border-paper-edge bg-paper-soft overflow-hidden" >
          <div style={{ gridTemplateColumns: '1.5fr 1fr 1.2fr 1fr 96px', gap: 12, padding: '12px 16px' }} className="zs-eyebrow grid border-b">
            <span>Gate</span><span>Project</span><span>Source</span><span>Handled by</span><span>Status</span>
          </div>
          {gates.map(([g, proj, src, who, st, tone], i) => (
            <div className="grid items-center" key={i} style={{ gridTemplateColumns: '1.5fr 1fr 1.2fr 1fr 96px', gap: 12, padding: '16px 16px', borderBottom: i < gates.length - 1 ? 'var(--hairline)' : 'none' }}>
              <div className="flex items-center" style={{ gap: 9 }}><DLock c={tone === 'success' ? rc('success') : rc('accent')}/><span className="text-sm font-medium" >{g}</span></div>
              <span className="mono text-xs text-ink-mute">{proj}</span>
              <span className="mono text-xs text-ink-mute">{src}</span>
              <span className="text-sm text-ink-soft">{who || <span className="text-ink-faint">— unclaimed</span>}</span>
              <span className={'zs-badge ' + (tone === 'success' ? 'zs-badge-success' : 'zs-badge-accent')} style={{ textTransform: 'capitalize' }}>{st}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  RelayCoordinator, RelayPlanAuthor, RelayArchitecture, DojoRelayGates,
});
