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
      <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 24 }}>
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>者</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>Agent editor</div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--ink)' }}>{A.name}</h1>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>{A.description}</p>
        </div>
        <div style={{ paddingLeft: 24, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 24, alignItems: 'flex-start' }}>
          <AgMini n={`v${A.version}`} l="version" mono/>
          <AgMini n={A.replayFixtures.filter(f => f.result.passed).length + "/" + A.replayFixtures.length}
                  l="replays passing" mono accent/>
          <button style={{
            padding: '8px 16px', fontSize: 13, background: 'var(--ink)',
            color: 'var(--paper)', borderRadius: 5, border: 'none',
            cursor: 'pointer', alignSelf: 'center', fontFamily: 'var(--font-ui)'
          }}>Save</button>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.3fr 1fr' }}>
        {/* Left column */}
        <div style={{ overflow: 'auto', padding: '24px 32px', borderRight: 'var(--hairline)' }}>

          <AgSection title="Template">
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
              {A.templates.map(t => (
                <button key={t.id} style={{
                  textAlign: 'left', padding: '12px 12px', borderRadius: 5,
                  background: A.template === t.id ? 'var(--paper-3)' : 'transparent',
                  border: A.template === t.id ? '1px solid var(--ink)' : 'var(--hairline)',
                  cursor: 'pointer', fontFamily: 'var(--font-ui)'
                }}>
                  <div style={{ fontSize: 13, color: 'var(--ink)', marginBottom: 4 }}>
                    {t.label}
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>{t.desc}</div>
                </button>
              ))}
            </div>
          </AgSection>

          <AgSection title="Autonomy ceiling"
                     subtitle="How far the agent runs without a human. Tool access scales with this.">
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                           gap: 8, marginBottom: 12 }}>
              {A.autonomyLevels.map((lvl, i) => (
                <button key={lvl.id} onClick={() => setAutonomy(lvl.id)} style={{
                  textAlign: 'left', padding: '12px 12px', borderRadius: 5,
                  background: autonomy === lvl.id ? 'var(--ink)' : 'transparent',
                  color: autonomy === lvl.id ? 'var(--paper)' : 'var(--ink)',
                  border: autonomy === lvl.id ? '1px solid var(--ink)' : 'var(--hairline)',
                  cursor: 'pointer', fontFamily: 'var(--font-ui)',
                  display: 'flex', flexDirection: 'column', gap: 4, minHeight: 100
                }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
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
                <div style={{ padding: '12px 12px', background: 'var(--paper-2)',
                               borderRadius: 5, border: 'var(--hairline)' }}>
                  <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                                 textTransform: 'uppercase', marginBottom: 8 }}>
                    Powers at this level
                  </div>
                  <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                    {cur.powers.map(p => (
                      <span key={p} className="mono" style={{ fontSize: 11,
                        color: 'var(--ink-2)', background: 'var(--paper)',
                        padding: '4px 8px', borderRadius: 3,
                        border: 'var(--hairline)' }}>{p}</span>
                    ))}
                  </div>
                </div>
              );
            })()}
          </AgSection>

          <AgSection title="Tool envelope" subtitle="What the agent can call. Rationale required for each.">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {A.tools.map(t => (
                <div key={t.id} style={{
                  display: 'grid', gridTemplateColumns: '20px 110px 1fr auto',
                  gap: 12, alignItems: 'center', padding: '8px 12px',
                  borderRadius: 4, border: 'var(--hairline)',
                  background: t.allowed ? 'var(--paper-2)' : 'transparent'
                }}>
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
        <div style={{ overflow: 'auto', padding: '24px 24px',
                       background: 'var(--paper-2)' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>Replay test</div>
          <h3 className="display" style={{ fontSize: 17, fontWeight: 400, margin: '0 0 12px',
                                            color: 'var(--ink)' }}>
            How would the agent behave on past sessions?
          </h3>

          {/* Fixture list */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 16 }}>
            {A.replayFixtures.map(f => (
              <button key={f.id} onClick={() => setActiveFix(f.id)} style={{
                textAlign: 'left', padding: '8px 12px', borderRadius: 4,
                background: activeFix === f.id ? 'var(--paper)' : 'transparent',
                border: activeFix === f.id ? '1px solid var(--ink-3)' : 'var(--hairline)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)',
                display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                gap: 8, alignItems: 'center'
              }}>
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
          <div style={{ background: 'var(--paper)', border: 'var(--hairline)',
                         borderRadius: 6, padding: '16px 16px' }}>
            <div style={{ display: 'flex', alignItems: 'center',
                           justifyContent: 'space-between', marginBottom: 12 }}>
              <span style={{ fontSize: 13, color: 'var(--ink-2)' }}>{fixture.label}</span>
              <span style={{ fontSize: 11, letterSpacing: '0.12em',
                              textTransform: 'uppercase',
                              color: fixture.result.passed ? 'var(--success)' : 'var(--warning)' }}>
                {fixture.result.passed ? "passed" : "diverged"}
              </span>
            </div>

            <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55,
                           marginBottom: 12 }}>{fixture.description}</div>

            <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '4px 14px',
                           fontSize: 11, marginBottom: 12 }}>
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
              <div style={{ padding: '8px 12px', borderRadius: 4,
                             background: 'var(--paper-2)',
                             borderLeft: '2px solid var(--warning)',
                             fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55 }}>
                <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--warning)',
                                textTransform: 'uppercase', display: 'block', marginBottom: 4 }}>
                  Why it diverged
                </span>
                {fixture.result.divergence}
              </div>
            )}

            <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
              <button style={{
                padding: '8px 12px', fontSize: 13, background: 'var(--ink)',
                color: 'var(--paper)', borderRadius: 5, border: 'none',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
              }}>Replay  →</button>
              <button style={{
                padding: '8px 12px', fontSize: 13, background: 'transparent',
                color: 'var(--ink-2)', borderRadius: 5,
                border: '1px solid var(--ink-3)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
              }}>View trace</button>
              <span style={{ flex: 1 }}/>
              <button style={{ fontSize: 11, color: 'var(--ink-3)',
                                background: 'transparent', border: 'none',
                                cursor: 'pointer' }}>+ add fixture</button>
            </div>
          </div>

          <div style={{ marginTop: 16, padding: '12px 12px', borderRadius: 5,
                         background: 'var(--paper)', border: 'var(--hairline)' }}>
            <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                           textTransform: 'uppercase', marginBottom: 8 }}>
              Run all replays
            </div>
            <p style={{ fontSize: 11, color: 'var(--ink-3)', margin: '0 0 8px',
                         lineHeight: 1.5 }}>
              Sensei reruns every fixture against the current agent definition.
              Use this before publishing a new version.
            </p>
            <button style={{
              padding: '8px 12px', fontSize: 13, background: 'var(--ink)',
              color: 'var(--paper)', borderRadius: 5, border: 'none',
              cursor: 'pointer', width: '100%', fontFamily: 'var(--font-ui)'
            }}>Run {A.replayFixtures.length} replays  →</button>
          </div>
        </div>
      </div>
    </div>
  );
}

function AgSection({ title, subtitle, children }) {
  return (
    <section style={{ marginBottom: 24 }}>
      <div style={{ marginBottom: 12 }}>
        <h3 className="display" style={{ fontSize: 15, fontWeight: 400, margin: 0,
                                          color: 'var(--ink)' }}>{title}</h3>
        {subtitle && (
          <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>{subtitle}</div>
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
      <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase', marginTop: 4 }}>{l}</div>
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
      <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 24 }}>
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>貌</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            Persona editor  ·  the hat sensei wears
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--ink)' }}>{P.name}</h1>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>{P.description}</p>
        </div>
        <div style={{ paddingLeft: 24, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 24 }}>
          <AgMini n={P.rules.length} l="rules"/>
          <AgMini n={P.evidence.length} l="evidence cited" mono/>
          <AgMini n={P.assembled.tokenEstimate.toLocaleString()} l="tokens" mono accent/>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.1fr 1fr 1fr' }}>
        {/* Col 1: Triggers + assembled context */}
        <div style={{ overflow: 'auto', padding: '24px 24px',
                       borderRight: 'var(--hairline)' }}>
          <AgSection title="Triggers"
                     subtitle="When sensei dons this hat. ANDed clauses.">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {P.triggers.map((t, i) => (
                <div key={i} style={{ padding: '8px 12px', borderRadius: 4,
                                       border: 'var(--hairline)', background: 'var(--paper-2)' }}>
                  <div style={{ fontSize: 13, color: 'var(--ink)',
                                 marginBottom: 4 }}>{t.label}</div>
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
            <div style={{ padding: '12px 16px', background: 'var(--paper-2)',
                           border: 'var(--hairline)', borderRadius: 5 }}>
              <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '5px 12px',
                             fontSize: 11, marginBottom: 8 }}>
                <span style={{ color: 'var(--ink-4)' }}>Active rules</span>
                <span className="mono" style={{ color: 'var(--ink-2)' }}>{P.assembled.activeRules}</span>
                <span style={{ color: 'var(--ink-4)' }}>Memory refs loaded</span>
                <span className="mono" style={{ color: 'var(--ink-2)' }}>{P.assembled.memoryRefsLoaded}</span>
                <span style={{ color: 'var(--ink-4)' }}>Token estimate</span>
                <span className="mono" style={{ color: 'var(--ink-2)' }}>{P.assembled.tokenEstimate.toLocaleString()}</span>
              </div>
              <pre className="mono" style={{ fontSize: 11, color: 'var(--ink-2)',
                background: 'transparent', margin: 0, lineHeight: 1.6,
                whiteSpace: 'pre-wrap' }}>{P.assembled.systemSnippet}</pre>
            </div>
          </AgSection>
        </div>

        {/* Col 2: Rules */}
        <div style={{ overflow: 'auto', padding: '24px 24px',
                       borderRight: 'var(--hairline)' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                         marginBottom: 12 }}>
            <h3 className="display" style={{ fontSize: 15, fontWeight: 400, margin: 0,
                                              color: 'var(--ink)' }}>Rules</h3>
            <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              short imperatives the persona embodies
            </span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {P.rules.map(r => (
              <button key={r.id} onClick={() => setActiveRule(r.id)} style={{
                textAlign: 'left', padding: '12px 12px', borderRadius: 5,
                background: activeRule === r.id ? 'var(--paper-3)' : 'transparent',
                border: activeRule === r.id ? '1px solid var(--ink-3)' : 'var(--hairline)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
              }}>
                <div style={{ display: 'flex', alignItems: 'baseline', gap: 8,
                               marginBottom: 4 }}>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
                    {r.id.toUpperCase()}
                  </span>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)',
                                                    marginLeft: 'auto' }}>
                    {r.evidenceCount} citations
                  </span>
                </div>
                <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.5 }}>
                  {r.text}
                </div>
                <div style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
                  last fired {r.lastFired}
                </div>
              </button>
            ))}
            <button style={{ fontSize: 11, color: 'var(--ink-3)',
              background: 'transparent', border: '1px dashed var(--edge)',
              borderRadius: 4, padding: '8px', cursor: 'pointer',
              textAlign: 'center', marginTop: 4 }}>
              + add rule
            </button>
          </div>
        </div>

        {/* Col 3: Evidence trail for selected rule */}
        <div style={{ overflow: 'auto', padding: '24px 24px',
                       background: 'var(--paper-2)' }}>
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                           textTransform: 'uppercase', marginBottom: 4 }}>
              Evidence trail  ·  {rule.id.toUpperCase()}
            </div>
            <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.5,
                           padding: '8px 12px', background: 'var(--paper)',
                           border: 'var(--hairline)', borderRadius: 4 }}>
              "{rule.text}"
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
              Pulled live from sensei's memory store. Each row links a session
              where this rule shaped sensei's response.
            </div>
          </div>

          {ruleEvidence.length === 0 ? (
            <div style={{ padding: '16px', textAlign: 'center', fontSize: 13,
                           color: 'var(--ink-3)' }}>
              No evidence cited for this rule yet.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {ruleEvidence.map(e => (
                <div key={e.memoryId} style={{ background: 'var(--paper)',
                  border: 'var(--hairline)', borderRadius: 5,
                  padding: '12px 12px' }}>
                  <div style={{ display: 'flex', alignItems: 'center',
                                 justifyContent: 'space-between', marginBottom: 8 }}>
                    <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
                      {e.memoryId}
                    </span>
                    <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>{e.when}</span>
                  </div>
                  <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6,
                                 marginBottom: 8 }}>
                    {e.snippet}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                                 fontSize: 11 }}>
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
