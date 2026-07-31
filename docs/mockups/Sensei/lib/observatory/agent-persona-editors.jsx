// Agent editor — autonomy level + tool envelope + replay test against past sessions.
// Persona editor — rules + evidence trail pulled from sensei's memory.

const { useState: agS } = React;

// ─── Agent editor ──────────────────────────────────────────
function AgentEditor() {
  const A = window.EXT_DATA.exampleAgent;
  const [autonomy, setAutonomy] = agS(A.autonomy);
  const [activeFix, setActiveFix] = agS(A.replayFixtures[0].id);

  const fixture = A.replayFixtures.find(f => f.id === activeFix);

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Agent editor"
 >
      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>者</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >Agent editor</div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>{A.name}</h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >{A.description}</p>
        </div>
        <div className="gap-6 pl-6 border-l flex items-start" >
          <AgMini n={`v${A.version}`} l="version" mono/>
          <AgMini n={A.replayFixtures.filter(f => f.result.passed).length + "/" + A.replayFixtures.length}
                  l="replays passing" mono accent/>
          <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-4 bg-ink text-paper border-0 cursor-pointer self-center" >Save</button>
        </div>
      </div>

      <div className="flex-1 min-h-0 grid" style={{
 gridTemplateColumns: '1.3fr 1fr' }}>
        {/* Left column */}
        <div className="py-6 px-8 overflow-auto border-r" >

          <AgSection title="Template">
            <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-2 grid" >
              {A.templates.map(t => (
                <button key={t.id} style={{ borderRadius: 5,
 background: A.template === t.id ? 'var(--paper-3)' : 'transparent',
 border: A.template === t.id ? '1px solid var(--ink)' : 'var(--hairline)', fontFamily: 'var(--font-ui)'
 }} className="py-3 px-3 text-left cursor-pointer" >
                  <div style={{ fontSize: 13 }} className="mb-1 text-ink" >
                    {t.label}
                  </div>
                  <div className="text-ink-3" style={{ fontSize: 11 }}>{t.desc}</div>
                </button>
              ))}
            </div>
          </AgSection>

          <AgSection title="Autonomy ceiling"
                     subtitle="How far the agent runs without a human. Tool access scales with this.">
            <div style={{ gridTemplateColumns: 'repeat(4, 1fr)'
 }} className="gap-2 mb-3 grid" >
              {A.autonomyLevels.map((lvl, i) => (
                <button key={lvl.id} onClick={() => setAutonomy(lvl.id)} style={{ borderRadius: 5,
 background: autonomy === lvl.id ? 'var(--ink)' : 'transparent',
 color: autonomy === lvl.id ? 'var(--paper)' : 'var(--ink)',
 border: autonomy === lvl.id ? '1px solid var(--ink)' : 'var(--hairline)', fontFamily: 'var(--font-ui)', minHeight: 100
 }} className="py-3 px-3 gap-1 text-left cursor-pointer flex flex-col" >
                  <div className="gap-1 flex items-center" >
                    <span className="kanji" style={{ fontSize: 17,
                      color: autonomy === lvl.id ? 'var(--paper)' : 'var(--accent)' }}>{lvl.kanji}</span>
                    <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.14em',
 color: autonomy === lvl.id ? 'var(--paper-3)' : 'var(--ink-4)' }}>level {i + 1}</span>
                  </div>
                  <div className="font-medium" style={{ fontSize: 13 }}>{lvl.label}</div>
                  <div style={{ fontSize: 11, lineHeight: 1.45,
                    color: autonomy === lvl.id ? 'var(--paper-3)' : 'var(--ink-3)' }}>
                    {lvl.rule}
                  </div>
                </button>
              ))}
            </div>
            {/* Powers list for selected */}
            {(() => {
              const cur = A.autonomyLevels.find(l => l.id === autonomy);
              return (
                <div style={{
 borderRadius: 5 }} className="py-3 px-3 bg-paper-2 border border-paper-edge" >
                  <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 text-ink-4 uppercase" >
                    Powers at this level
                  </div>
                  <div className="gap-1 flex flex-wrap" >
                    {cur.powers.map(p => (
                      <span key={p} className="mono py-1 px-2 text-ink-2 bg-paper border border-paper-edge" style={{
 fontSize: 11, borderRadius: 3 }}>{p}</span>
                    ))}
                  </div>
                </div>
              );
            })()}
          </AgSection>

          <AgSection title="Tool envelope" subtitle="What the agent can call. Rationale required for each.">
            <div className="gap-1 flex flex-col" >
              {A.tools.map(t => (
                <div key={t.id} style={{ gridTemplateColumns: '20px 110px 1fr auto',
 borderRadius: 4,
 background: t.allowed ? 'var(--paper-2)' : 'transparent'
 }} className="gap-3 py-2 px-3 grid items-center border border-paper-edge" >
                  <span className="inline-flex items-center justify-center text-paper" style={{
 width: 14, height: 14, borderRadius: 3,
 border: '1px solid ' + (t.allowed ? 'var(--accent)' : 'var(--ink-4)'),
 background: t.allowed ? 'var(--accent)' : 'transparent', fontSize: 11
 }}>{t.allowed ? '✓' : ''}</span>
                  <span className="mono" style={{ fontSize: 13,
                    color: t.allowed ? 'var(--ink)' : 'var(--ink-3)' }}>{t.label}</span>
                  <span className="text-ink-3" style={{ fontSize: 13, lineHeight: 1.45 }}>
                    {t.rationale}
                  </span>
                  <span className="uppercase" style={{ fontSize: 11, color: t.allowed ? 'var(--success)' : 'var(--ink-4)',
 letterSpacing: '0.12em' }}>
                    {t.allowed ? 'on' : 'off'}
                  </span>
                </div>
              ))}
            </div>
          </AgSection>
        </div>

        {/* Right: replay test panel */}
        <div className="py-6 px-6 overflow-auto bg-paper-2" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >Replay test</div>
          <h3 className="display mt-0 mb-3 font-normal text-ink" style={{
 fontSize: 17 }}>
            How would the agent behave on past sessions?
          </h3>

          {/* Fixture list */}
          <div className="gap-1 mb-4 flex flex-col" >
            {A.replayFixtures.map(f => (
              <button key={f.id} onClick={() => setActiveFix(f.id)} style={{ borderRadius: 4,
 background: activeFix === f.id ? 'var(--paper)' : 'transparent',
 border: activeFix === f.id ? '1px solid var(--ink-3)' : 'var(--hairline)', fontFamily: 'var(--font-ui)', gridTemplateColumns: 'auto 1fr auto' }} className="py-2 px-3 gap-2 text-left cursor-pointer grid items-center" >
                <span className="rounded-full" style={{
 width: 8, height: 8,
 background: f.result.passed ? 'var(--success)' : 'var(--warning)'
 }}/>
                <div className="min-w-0" >
                  <div className="text-ink whitespace-nowrap overflow-hidden text-ellipsis" style={{ fontSize: 13 }}>{f.label}</div>
                  <div className="text-ink-3" style={{ fontSize: 11 }}>{f.when}</div>
                </div>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                  {f.result.steps} steps
                </span>
              </button>
            ))}
          </div>

          {/* Fixture detail */}
          <div style={{
 borderRadius: 6
 }} className="py-4 px-4 bg-paper border border-paper-edge" >
            <div className="mb-3 flex items-center justify-between" >
              <span className="text-ink-2" style={{ fontSize: 13 }}>{fixture.label}</span>
              <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.12em',
 color: fixture.result.passed ? 'var(--success)' : 'var(--warning)' }}>
                {fixture.result.passed ? "passed" : "diverged"}
              </span>
            </div>

            <div style={{
 fontSize: 13, lineHeight: 1.55
 }} className="mb-3 text-ink-2" >{fixture.description}</div>

            <div style={{ gridTemplateColumns: 'auto 1fr', gap: '4px 12px',
 fontSize: 11
 }} className="mb-3 grid" >
              <span className="text-ink-4" >Expected outcome</span>
              <span className="text-ink" >{fixture.correctOutcome}</span>
              <span className="text-ink-4" >Steps</span>
              <span className="mono text-ink-2" >{fixture.result.steps}</span>
              <span className="text-ink-4" >Duration</span>
              <span className="mono text-ink-2" >
                {(fixture.result.durationMs/1000).toFixed(1)}s
              </span>
              <span className="text-ink-4" >Tool calls</span>
              <span className="mono text-ink-2" >{fixture.result.toolCalls}</span>
            </div>

            {fixture.result.divergence && (
              <div style={{
 borderRadius: 4,
 borderLeft: '2px solid var(--warning)',
 fontSize: 11, lineHeight: 1.55
 }} className="py-2 px-3 bg-paper-2 text-ink-2" >
                <span style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-warning uppercase block" >
                  Why it diverged
                </span>
                {fixture.result.divergence}
              </div>
            )}

            <div className="gap-2 mt-3 flex" >
              <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-3 bg-ink text-paper border-0 cursor-pointer" >Replay  →</button>
              <button style={{
 fontSize: 13, borderRadius: 5,
 border: '1px solid var(--ink-3)', fontFamily: 'var(--font-ui)'
 }} className="py-2 px-3 bg-transparent text-ink-2 cursor-pointer" >View trace</button>
              <span className="flex-1" />
              <button className="text-ink-3 bg-transparent border-0 cursor-pointer" style={{ fontSize: 11 }}>+ add fixture</button>
            </div>
          </div>

          <div style={{
 borderRadius: 5 }} className="mt-4 py-3 px-3 bg-paper border border-paper-edge" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 text-ink-4 uppercase" >
              Run all replays
            </div>
            <p style={{
 fontSize: 11,
 lineHeight: 1.5
 }} className="mt-0 mb-2 text-ink-3" >
              Sensei reruns every fixture against the current agent definition.
              Use this before publishing a new version.
            </p>
            <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-3 bg-ink text-paper border-0 cursor-pointer w-full" >Run {A.replayFixtures.length} replays  →</button>
          </div>
        </div>
      </div>
    </div>
  );
}

function AgSection({ title, subtitle, children }) {
  return (
    <section className="mb-6" >
      <div className="mb-3" >
        <h3 className="display m-0 font-normal text-ink" style={{
 fontSize: 15 }}>{title}</h3>
        {subtitle && (
          <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >{subtitle}</div>
        )}
      </div>
      {children}
    </section>
  );
}
function AgMini({ n, l, mono, accent }) {
  return (
    <div className="text-right" >
      <div className={mono ? "mono" : "display"} style={{
        fontSize: mono ? 13 : 22, color: accent ? 'var(--accent)' : 'var(--ink)',
        fontWeight: 400, lineHeight: 1
      }}>{n}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 text-ink-4 uppercase" >{l}</div>
    </div>
  );
}

// ─── Persona editor ────────────────────────────────────────
function PersonaEditor() {
  const P = window.EXT_DATA.examplePersona;
  const [activeRule, setActiveRule] = agS(P.rules[0].id);

  const rule = P.rules.find(r => r.id === activeRule);
  const ruleEvidence = P.evidence.filter(e => e.ruleId === activeRule);

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Persona editor"
 >
      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>貌</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            Persona editor  ·  the hat sensei wears
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>{P.name}</h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >{P.description}</p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
          <AgMini n={P.rules.length} l="rules"/>
          <AgMini n={P.evidence.length} l="evidence cited" mono/>
          <AgMini n={P.assembled.tokenEstimate.toLocaleString()} l="tokens" mono accent/>
        </div>
      </div>

      <div className="flex-1 min-h-0 grid" style={{
 gridTemplateColumns: '1.1fr 1fr 1fr' }}>
        {/* Col 1: Triggers + assembled context */}
        <div className="py-6 px-6 overflow-auto border-r" >
          <AgSection title="Triggers"
                     subtitle="When sensei dons this hat. ANDed clauses.">
            <div className="gap-1 flex flex-col" >
              {P.triggers.map((t, i) => (
                <div key={i} style={{
 borderRadius: 4 }} className="py-2 px-3 border border-paper-edge bg-paper-2" >
                  <div style={{
 fontSize: 13 }} className="mb-1 text-ink" >{t.label}</div>
                  <div className="mono text-ink-3" style={{ fontSize: 11 }}>
                    {t.kind} {t.op} "{t.value}"
                  </div>
                </div>
              ))}
            </div>
          </AgSection>

          <AgSection title="What & why" subtitle="Persona description (covers stance, not method)">
            <textarea style={{ ...fieldBox, minHeight: 100, resize: 'vertical',
              lineHeight: 1.55 }} defaultValue={P.description}/>
          </AgSection>

          <AgSection title="Assembled context">
            <div style={{ borderRadius: 5
 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
              <div style={{ gridTemplateColumns: 'auto 1fr', gap: '4px 12px',
 fontSize: 11
 }} className="mb-2 grid" >
                <span className="text-ink-4" >Active rules</span>
                <span className="mono text-ink-2" >{P.assembled.activeRules}</span>
                <span className="text-ink-4" >Memory refs loaded</span>
                <span className="mono text-ink-2" >{P.assembled.memoryRefsLoaded}</span>
                <span className="text-ink-4" >Token estimate</span>
                <span className="mono text-ink-2" >{P.assembled.tokenEstimate.toLocaleString()}</span>
              </div>
              <pre className="mono m-0 text-ink-2 bg-transparent" style={{
 fontSize: 11, lineHeight: 1.6,
 whiteSpace: 'pre-wrap'
 }}>{P.assembled.systemSnippet}</pre>
            </div>
          </AgSection>
        </div>

        {/* Col 2: Rules */}
        <div className="py-6 px-6 overflow-auto border-r" >
          <div className="mb-3 flex items-center justify-between" >
            <h3 className="display m-0 font-normal text-ink" style={{
 fontSize: 15 }}>Rules</h3>
            <span className="text-ink-3" style={{ fontSize: 11 }}>
              short imperatives the persona embodies
            </span>
          </div>
          <div className="gap-1 flex flex-col" >
            {P.rules.map(r => (
              <button key={r.id} onClick={() => setActiveRule(r.id)} style={{ borderRadius: 5,
 background: activeRule === r.id ? 'var(--paper-3)' : 'transparent',
 border: activeRule === r.id ? '1px solid var(--ink-3)' : 'var(--hairline)', fontFamily: 'var(--font-ui)'
 }} className="py-3 px-3 text-left cursor-pointer" >
                <div className="gap-2 mb-1 flex items-baseline" >
                  <span className="mono text-accent" style={{ fontSize: 11 }}>
                    {r.id.toUpperCase()}
                  </span>
                  <span className="mono ml-auto text-ink-4" style={{
 fontSize: 11 }}>
                    {r.evidenceCount} citations
                  </span>
                </div>
                <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.5 }}>
                  {r.text}
                </div>
                <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
                  last fired {r.lastFired}
                </div>
              </button>
            ))}
            <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 4 }} className="p-2 mt-1 text-ink-3 bg-transparent cursor-pointer text-center" >
              + add rule
            </button>
          </div>
        </div>

        {/* Col 3: Evidence trail for selected rule */}
        <div className="py-6 px-6 overflow-auto bg-paper-2" >
          <div className="mb-3" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-1 text-ink-4 uppercase" >
              Evidence trail  ·  {rule.id.toUpperCase()}
            </div>
            <div style={{
 fontSize: 13, lineHeight: 1.5, borderRadius: 4
 }} className="py-2 px-3 text-ink bg-paper border border-paper-edge" >
              "{rule.text}"
            </div>
            <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
              Pulled live from sensei's memory store. Each row links a session
              where this rule shaped sensei's response.
            </div>
          </div>

          {ruleEvidence.length === 0 ? (
            <div style={{ fontSize: 13 }} className="p-4 text-center text-ink-3" >
              No evidence cited for this rule yet.
            </div>
          ) : (
            <div className="gap-2 flex flex-col" >
              {ruleEvidence.map(e => (
                <div key={e.memoryId} style={{ borderRadius: 5
 }} className="py-3 px-3 bg-paper border border-paper-edge" >
                  <div className="mb-2 flex items-center justify-between" >
                    <span className="mono text-accent" style={{ fontSize: 11 }}>
                      {e.memoryId}
                    </span>
                    <span className="text-ink-3" style={{ fontSize: 11 }}>{e.when}</span>
                  </div>
                  <div style={{
 fontSize: 13, lineHeight: 1.6
 }} className="mb-2 text-ink-2" >
                    {e.snippet}
                  </div>
                  <div style={{
 fontSize: 11
 }} className="gap-2 flex items-center" >
                    <a className="text-ink-3" href="#" >{e.sessionId}</a>
                    <span className="text-ink-4" >·</span>
                    <a className="text-ink-3" href="#" >view memory →</a>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

window.AgentEditor = AgentEditor;
window.PersonaEditor = PersonaEditor;
