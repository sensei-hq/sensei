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
    <div className="sensei" data-screen-label="Agent editor"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      {/* Hero */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>者</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >Agent editor</div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>{A.name}</h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >{A.description}</p>
        </div>
        <div style={{
 borderLeft: 'var(--hairline)',
                       display: 'flex', alignItems: 'flex-start'
}} className="gap-5 pl-5" >
          <AgMini n={`v${A.version}`} l="version" mono/>
          <AgMini n={A.replayFixtures.filter(f => f.result.passed).length + "/" + A.replayFixtures.length}
                  l="replays passing" mono accent/>
          <button style={{
 fontSize: 13, background: 'var(--ink)',
            color: 'var(--paper)', borderRadius: 5, border: 'none',
            cursor: 'pointer', alignSelf: 'center', fontFamily: 'var(--font-ui)'
}} className="py-2 px-4" >Save</button>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.3fr 1fr' }}>
        {/* Left column */}
        <div style={{ overflow: 'auto', borderRight: 'var(--hairline)' }} className="py-5 px-6" >

          <AgSection title="Template">
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-2" >
              {A.templates.map(t => (
                <button key={t.id} style={{
                  textAlign: 'left', borderRadius: 5,
                  background: A.template === t.id ? 'var(--paper-3)' : 'transparent',
                  border: A.template === t.id ? '1px solid var(--ink)' : 'var(--hairline)',
                  cursor: 'pointer', fontFamily: 'var(--font-ui)'
}} className="py-3 px-3" >
                  <div style={{ fontSize: 13, color: 'var(--ink)' }} className="mb-1" >
                    {t.label}
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>{t.desc}</div>
                </button>
              ))}
            </div>
          </AgSection>

          <AgSection title="Autonomy ceiling"
                     subtitle="How far the agent runs without a human. Tool access scales with this.">
            <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)'
}} className="gap-2 mb-3" >
              {A.autonomyLevels.map((lvl, i) => (
                <button key={lvl.id} onClick={() => setAutonomy(lvl.id)} style={{
                  textAlign: 'left', borderRadius: 5,
                  background: autonomy === lvl.id ? 'var(--ink)' : 'transparent',
                  color: autonomy === lvl.id ? 'var(--paper)' : 'var(--ink)',
                  border: autonomy === lvl.id ? '1px solid var(--ink)' : 'var(--hairline)',
                  cursor: 'pointer', fontFamily: 'var(--font-ui)',
                  display: 'flex', flexDirection: 'column', minHeight: 100
}} className="py-3 px-3 gap-1" >
                  <div style={{ display: 'flex', alignItems: 'center' }} className="gap-1" >
                    <span className="kanji" style={{ fontSize: 17,
                      color: autonomy === lvl.id ? 'var(--paper)' : 'var(--accent)' }}>{lvl.kanji}</span>
                    <span style={{ fontSize: 11, letterSpacing: '0.14em',
                      color: autonomy === lvl.id ? 'var(--paper-3)' : 'var(--ink-4)',
                      textTransform: 'uppercase' }}>level {i + 1}</span>
                  </div>
                  <div style={{ fontSize: 13, fontWeight: 500 }}>{lvl.label}</div>
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
 background: 'var(--paper-2)',
                               borderRadius: 5, border: 'var(--hairline)'
}} className="py-3 px-3" >
                  <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                                 textTransform: 'uppercase'
}} className="mb-2" >
                    Powers at this level
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1" >
                    {cur.powers.map(p => (
                      <span key={p} className="mono py-1 px-2" style={{
 fontSize: 11,
                        color: 'var(--ink-2)', background: 'var(--paper)', borderRadius: 3,
                        border: 'var(--hairline)'
}}>{p}</span>
                    ))}
                  </div>
                </div>
              );
            })()}
          </AgSection>

          <AgSection title="Tool envelope" subtitle="What the agent can call. Rationale required for each.">
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
              {A.tools.map(t => (
                <div key={t.id} style={{
                  display: 'grid', gridTemplateColumns: '20px 110px 1fr auto', alignItems: 'center',
                  borderRadius: 4, border: 'var(--hairline)',
                  background: t.allowed ? 'var(--paper-2)' : 'transparent'
}} className="gap-3 py-2 px-3" >
                  <span style={{
                    width: 14, height: 14, borderRadius: 3,
                    border: '1px solid ' + (t.allowed ? 'var(--accent)' : 'var(--ink-4)'),
                    background: t.allowed ? 'var(--accent)' : 'transparent',
                    display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                    color: 'var(--paper)', fontSize: 11
                  }}>{t.allowed ? '✓' : ''}</span>
                  <span className="mono" style={{ fontSize: 13,
                    color: t.allowed ? 'var(--ink)' : 'var(--ink-3)' }}>{t.label}</span>
                  <span style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.45 }}>
                    {t.rationale}
                  </span>
                  <span style={{ fontSize: 11, color: t.allowed ? 'var(--success)' : 'var(--ink-4)',
                                  letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                    {t.allowed ? 'on' : 'off'}
                  </span>
                </div>
              ))}
            </div>
          </AgSection>
        </div>

        {/* Right: replay test panel */}
        <div style={{
 overflow: 'auto',
                       background: 'var(--paper-2)'
}} className="py-5 px-5" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >Replay test</div>
          <h3 className="display mt-0 mb-3" style={{
 fontSize: 17, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            How would the agent behave on past sessions?
          </h3>

          {/* Fixture list */}
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1 mb-4" >
            {A.replayFixtures.map(f => (
              <button key={f.id} onClick={() => setActiveFix(f.id)} style={{
                textAlign: 'left', borderRadius: 4,
                background: activeFix === f.id ? 'var(--paper)' : 'transparent',
                border: activeFix === f.id ? '1px solid var(--ink-3)' : 'var(--hairline)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)',
                display: 'grid', gridTemplateColumns: 'auto 1fr auto', alignItems: 'center'
}} className="py-2 px-3 gap-2" >
                <span style={{
                  width: 8, height: 8, borderRadius: '50%',
                  background: f.result.passed ? 'var(--success)' : 'var(--warning)'
                }}/>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: 'var(--ink)',
                                 whiteSpace: 'nowrap', overflow: 'hidden',
                                 textOverflow: 'ellipsis' }}>{f.label}</div>
                  <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>{f.when}</div>
                </div>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {f.result.steps} steps
                </span>
              </button>
            ))}
          </div>

          {/* Fixture detail */}
          <div style={{
 background: 'var(--paper)', border: 'var(--hairline)',
                         borderRadius: 6
}} className="py-4 px-4" >
            <div style={{
 display: 'flex', alignItems: 'center',
                           justifyContent: 'space-between'
}} className="mb-3" >
              <span style={{ fontSize: 13, color: 'var(--ink-2)' }}>{fixture.label}</span>
              <span style={{ fontSize: 11, letterSpacing: '0.12em',
                              textTransform: 'uppercase',
                              color: fixture.result.passed ? 'var(--success)' : 'var(--warning)' }}>
                {fixture.result.passed ? "passed" : "diverged"}
              </span>
            </div>

            <div style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55
}} className="mb-3" >{fixture.description}</div>

            <div style={{
 display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '4px 12px',
                           fontSize: 11
}} className="mb-3" >
              <span style={{ color: 'var(--ink-4)' }}>Expected outcome</span>
              <span style={{ color: 'var(--ink)' }}>{fixture.correctOutcome}</span>
              <span style={{ color: 'var(--ink-4)' }}>Steps</span>
              <span className="mono" style={{ color: 'var(--ink-2)' }}>{fixture.result.steps}</span>
              <span style={{ color: 'var(--ink-4)' }}>Duration</span>
              <span className="mono" style={{ color: 'var(--ink-2)' }}>
                {(fixture.result.durationMs/1000).toFixed(1)}s
              </span>
              <span style={{ color: 'var(--ink-4)' }}>Tool calls</span>
              <span className="mono" style={{ color: 'var(--ink-2)' }}>{fixture.result.toolCalls}</span>
            </div>

            {fixture.result.divergence && (
              <div style={{
 borderRadius: 4,
                             background: 'var(--paper-2)',
                             borderLeft: '2px solid var(--warning)',
                             fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55
}} className="py-2 px-3" >
                <span style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--warning)',
                                textTransform: 'uppercase', display: 'block'
}} className="mb-1" >
                  Why it diverged
                </span>
                {fixture.result.divergence}
              </div>
            )}

            <div style={{ display: 'flex' }} className="gap-2 mt-3" >
              <button style={{
 fontSize: 13, background: 'var(--ink)',
                color: 'var(--paper)', borderRadius: 5, border: 'none',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
}} className="py-2 px-3" >Replay  →</button>
              <button style={{
 fontSize: 13, background: 'transparent',
                color: 'var(--ink-2)', borderRadius: 5,
                border: '1px solid var(--ink-3)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
}} className="py-2 px-3" >View trace</button>
              <span style={{ flex: 1 }}/>
              <button style={{ fontSize: 11, color: 'var(--ink-3)',
                                background: 'transparent', border: 'none',
                                cursor: 'pointer' }}>+ add fixture</button>
            </div>
          </div>

          <div style={{
 borderRadius: 5,
                         background: 'var(--paper)', border: 'var(--hairline)'
}} className="mt-4 py-3 px-3" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                           textTransform: 'uppercase'
}} className="mb-2" >
              Run all replays
            </div>
            <p style={{
 fontSize: 11, color: 'var(--ink-3)',
                         lineHeight: 1.5
}} className="mt-0 mb-2" >
              Sensei reruns every fixture against the current agent definition.
              Use this before publishing a new version.
            </p>
            <button style={{
 fontSize: 13, background: 'var(--ink)',
              color: 'var(--paper)', borderRadius: 5, border: 'none',
              cursor: 'pointer', width: '100%', fontFamily: 'var(--font-ui)'
}} className="py-2 px-3" >Run {A.replayFixtures.length} replays  →</button>
          </div>
        </div>
      </div>
    </div>
  );
}

function AgSection({ title, subtitle, children }) {
  return (
    <section className="mb-5" >
      <div className="mb-3" >
        <h3 className="display m-0" style={{
 fontSize: 15, fontWeight: 400,
                                          color: 'var(--ink)'
}}>{title}</h3>
        {subtitle && (
          <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >{subtitle}</div>
        )}
      </div>
      {children}
    </section>
  );
}
function AgMini({ n, l, mono, accent }) {
  return (
    <div style={{ textAlign: 'right' }}>
      <div className={mono ? "mono" : "display"} style={{
        fontSize: mono ? 13 : 22, color: accent ? 'var(--accent)' : 'var(--ink)',
        fontWeight: 400, lineHeight: 1
      }}>{n}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase'
}} className="mt-1" >{l}</div>
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
    <div className="sensei" data-screen-label="Persona editor"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      {/* Hero */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="gap-5 pt-5 pb-4 px-6" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>貌</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            Persona editor  ·  the hat sensei wears
          </div>
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>{P.name}</h1>
          <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                       maxWidth: 720, lineHeight: 1.55
}} className="mt-1 mb-0" >{P.description}</p>
        </div>
        <div style={{
 borderLeft: 'var(--hairline)',
                       display: 'flex'
}} className="gap-5 pl-5" >
          <AgMini n={P.rules.length} l="rules"/>
          <AgMini n={P.evidence.length} l="evidence cited" mono/>
          <AgMini n={P.assembled.tokenEstimate.toLocaleString()} l="tokens" mono accent/>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.1fr 1fr 1fr' }}>
        {/* Col 1: Triggers + assembled context */}
        <div style={{
 overflow: 'auto',
                       borderRight: 'var(--hairline)'
}} className="py-5 px-5" >
          <AgSection title="Triggers"
                     subtitle="When sensei dons this hat. ANDed clauses.">
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
              {P.triggers.map((t, i) => (
                <div key={i} style={{
 borderRadius: 4,
                                       border: 'var(--hairline)', background: 'var(--paper-2)'
}} className="py-2 px-3" >
                  <div style={{
 fontSize: 13, color: 'var(--ink)'
}} className="mb-1" >{t.label}</div>
                  <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
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
            <div style={{
 background: 'var(--paper-2)',
                           border: 'var(--hairline)', borderRadius: 5
}} className="py-3 px-4" >
              <div style={{
 display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '4px 12px',
                             fontSize: 11
}} className="mb-2" >
                <span style={{ color: 'var(--ink-4)' }}>Active rules</span>
                <span className="mono" style={{ color: 'var(--ink-2)' }}>{P.assembled.activeRules}</span>
                <span style={{ color: 'var(--ink-4)' }}>Memory refs loaded</span>
                <span className="mono" style={{ color: 'var(--ink-2)' }}>{P.assembled.memoryRefsLoaded}</span>
                <span style={{ color: 'var(--ink-4)' }}>Token estimate</span>
                <span className="mono" style={{ color: 'var(--ink-2)' }}>{P.assembled.tokenEstimate.toLocaleString()}</span>
              </div>
              <pre className="mono m-0" style={{
 fontSize: 11, color: 'var(--ink-2)',
                background: 'transparent', lineHeight: 1.6,
                whiteSpace: 'pre-wrap'
}}>{P.assembled.systemSnippet}</pre>
            </div>
          </AgSection>
        </div>

        {/* Col 2: Rules */}
        <div style={{
 overflow: 'auto',
                       borderRight: 'var(--hairline)'
}} className="py-5 px-5" >
          <div style={{
 display: 'flex', alignItems: 'center', justifyContent: 'space-between'
}} className="mb-3" >
            <h3 className="display m-0" style={{
 fontSize: 15, fontWeight: 400,
                                              color: 'var(--ink)'
}}>Rules</h3>
            <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              short imperatives the persona embodies
            </span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            {P.rules.map(r => (
              <button key={r.id} onClick={() => setActiveRule(r.id)} style={{
                textAlign: 'left', borderRadius: 5,
                background: activeRule === r.id ? 'var(--paper-3)' : 'transparent',
                border: activeRule === r.id ? '1px solid var(--ink-3)' : 'var(--hairline)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
}} className="py-3 px-3" >
                <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2 mb-1" >
                  <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
                    {r.id.toUpperCase()}
                  </span>
                  <span className="mono ml-auto" style={{
 fontSize: 11, color: 'var(--ink-4)'
}}>
                    {r.evidenceCount} citations
                  </span>
                </div>
                <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.5 }}>
                  {r.text}
                </div>
                <div style={{ fontSize: 11, color: 'var(--ink-4)' }} className="mt-1" >
                  last fired {r.lastFired}
                </div>
              </button>
            ))}
            <button style={{
 fontSize: 11, color: 'var(--ink-3)',
              background: 'transparent', border: '1px dashed var(--edge)',
              borderRadius: 4, cursor: 'pointer',
              textAlign: 'center'
}} className="p-2 mt-1" >
              + add rule
            </button>
          </div>
        </div>

        {/* Col 3: Evidence trail for selected rule */}
        <div style={{
 overflow: 'auto',
                       background: 'var(--paper-2)'
}} className="py-5 px-5" >
          <div className="mb-3" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                           textTransform: 'uppercase'
}} className="mb-1" >
              Evidence trail  ·  {rule.id.toUpperCase()}
            </div>
            <div style={{
 fontSize: 13, color: 'var(--ink)', lineHeight: 1.5, background: 'var(--paper)',
                           border: 'var(--hairline)', borderRadius: 4
}} className="py-2 px-3" >
              "{rule.text}"
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >
              Pulled live from sensei's memory store. Each row links a session
              where this rule shaped sensei's response.
            </div>
          </div>

          {ruleEvidence.length === 0 ? (
            <div style={{
 textAlign: 'center', fontSize: 13,
                           color: 'var(--ink-3)'
}} className="p-4" >
              No evidence cited for this rule yet.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
              {ruleEvidence.map(e => (
                <div key={e.memoryId} style={{
 background: 'var(--paper)',
                  border: 'var(--hairline)', borderRadius: 5
}} className="py-3 px-3" >
                  <div style={{
 display: 'flex', alignItems: 'center',
                                 justifyContent: 'space-between'
}} className="mb-2" >
                    <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
                      {e.memoryId}
                    </span>
                    <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>{e.when}</span>
                  </div>
                  <div style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6
}} className="mb-2" >
                    {e.snippet}
                  </div>
                  <div style={{
 display: 'flex', alignItems: 'center',
                                 fontSize: 11
}} className="gap-2" >
                    <a href="#" style={{ color: 'var(--ink-3)' }}>{e.sessionId}</a>
                    <span style={{ color: 'var(--ink-4)' }}>·</span>
                    <a href="#" style={{ color: 'var(--ink-3)' }}>view memory →</a>
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
