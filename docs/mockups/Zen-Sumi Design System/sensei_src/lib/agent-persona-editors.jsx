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
      <div style={{ padding: '22px 36px 18px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 22 }}>
        <div className="kanji" style={{ fontSize: 42, color: 'var(--accent)', lineHeight: 1 }}>者</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--ink-mute)',
                         textTransform: 'uppercase', marginBottom: 5 }}>Agent editor</div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--ink)' }}>{A.name}</h1>
          <p style={{ fontSize: 12, color: 'var(--ink-soft)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>{A.description}</p>
        </div>
        <div style={{ paddingLeft: 22, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 22, alignItems: 'flex-start' }}>
          <AgMini n={`v${A.version}`} l="version" mono/>
          <AgMini n={A.replayFixtures.filter(f => f.result.passed).length + "/" + A.replayFixtures.length}
                  l="replays passing" mono accent/>
          <button style={{
            padding: '8px 16px', fontSize: 12, background: 'var(--ink)',
            color: 'var(--paper)', borderRadius: 5, border: 'none',
            cursor: 'pointer', alignSelf: 'center', fontFamily: 'var(--font-ui)'
          }}>Save</button>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.3fr 1fr' }}>
        {/* Left column */}
        <div style={{ overflow: 'auto', padding: '22px 32px', borderRight: 'var(--hairline)' }}>

          <AgSection title="Template">
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
              {A.templates.map(t => (
                <button key={t.id} style={{
                  textAlign: 'left', padding: '12px 14px', borderRadius: 5,
                  background: A.template === t.id ? 'var(--paper-mute)' : 'transparent',
                  border: A.template === t.id ? '1px solid var(--ink)' : 'var(--hairline)',
                  cursor: 'pointer', fontFamily: 'var(--font-ui)'
                }}>
                  <div style={{ fontSize: 13, color: 'var(--ink)', marginBottom: 3 }}>
                    {t.label}
                  </div>
                  <div style={{ fontSize: 11.5, color: 'var(--ink-mute)' }}>{t.desc}</div>
                </button>
              ))}
            </div>
          </AgSection>

          <AgSection title="Autonomy ceiling"
                     subtitle="How far the agent runs without a human. Tool access scales with this.">
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                           gap: 8, marginBottom: 14 }}>
              {A.autonomyLevels.map((lvl, i) => (
                <button key={lvl.id} onClick={() => setAutonomy(lvl.id)} style={{
                  textAlign: 'left', padding: '14px 14px', borderRadius: 5,
                  background: autonomy === lvl.id ? 'var(--ink)' : 'transparent',
                  color: autonomy === lvl.id ? 'var(--paper)' : 'var(--ink)',
                  border: autonomy === lvl.id ? '1px solid var(--ink)' : 'var(--hairline)',
                  cursor: 'pointer', fontFamily: 'var(--font-ui)',
                  display: 'flex', flexDirection: 'column', gap: 6, minHeight: 100
                }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <span className="kanji" style={{ fontSize: 18,
                      color: autonomy === lvl.id ? 'var(--paper)' : 'var(--accent)' }}>{lvl.kanji}</span>
                    <span style={{ fontSize: 9.5, letterSpacing: '0.14em',
                      color: autonomy === lvl.id ? 'var(--paper-mute)' : 'var(--ink-faint)',
                      textTransform: 'uppercase' }}>level {i + 1}</span>
                  </div>
                  <div style={{ fontSize: 13.5, fontWeight: 500 }}>{lvl.label}</div>
                  <div style={{ fontSize: 11.5, lineHeight: 1.45,
                    color: autonomy === lvl.id ? 'var(--paper-mute)' : 'var(--ink-mute)' }}>
                    {lvl.rule}
                  </div>
                </button>
              ))}
            </div>
            {/* Powers list for selected */}
            {(() => {
              const cur = A.autonomyLevels.find(l => l.id === autonomy);
              return (
                <div style={{ padding: '12px 14px', background: 'var(--paper-soft)',
                               borderRadius: 5, border: 'var(--hairline)' }}>
                  <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--ink-faint)',
                                 textTransform: 'uppercase', marginBottom: 7 }}>
                    Powers at this level
                  </div>
                  <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                    {cur.powers.map(p => (
                      <span key={p} className="mono" style={{ fontSize: 11,
                        color: 'var(--ink-soft)', background: 'var(--paper)',
                        padding: '4px 10px', borderRadius: 3,
                        border: 'var(--hairline)' }}>{p}</span>
                    ))}
                  </div>
                </div>
              );
            })()}
          </AgSection>

          <AgSection title="Tool envelope" subtitle="What the agent can call. Rationale required for each.">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {A.tools.map(t => (
                <div key={t.id} style={{
                  display: 'grid', gridTemplateColumns: '20px 110px 1fr auto',
                  gap: 12, alignItems: 'center', padding: '10px 14px',
                  borderRadius: 4, border: 'var(--hairline)',
                  background: t.allowed ? 'var(--paper-soft)' : 'transparent'
                }}>
                  <span style={{
                    width: 14, height: 14, borderRadius: 3,
                    border: '1px solid ' + (t.allowed ? 'var(--accent)' : 'var(--ink-faint)'),
                    background: t.allowed ? 'var(--accent)' : 'transparent',
                    display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                    color: 'var(--paper)', fontSize: 10
                  }}>{t.allowed ? '✓' : ''}</span>
                  <span className="mono" style={{ fontSize: 12,
                    color: t.allowed ? 'var(--ink)' : 'var(--ink-mute)' }}>{t.label}</span>
                  <span style={{ fontSize: 12, color: 'var(--ink-mute)', lineHeight: 1.45 }}>
                    {t.rationale}
                  </span>
                  <span style={{ fontSize: 10, color: t.allowed ? 'var(--success)' : 'var(--ink-faint)',
                                  letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                    {t.allowed ? 'on' : 'off'}
                  </span>
                </div>
              ))}
            </div>
          </AgSection>
        </div>

        {/* Right: replay test panel */}
        <div style={{ overflow: 'auto', padding: '22px 26px',
                       background: 'var(--paper-soft)' }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--ink-mute)',
                         textTransform: 'uppercase', marginBottom: 6 }}>Replay test</div>
          <h3 className="display" style={{ fontSize: 18, fontWeight: 400, margin: '0 0 14px',
                                            color: 'var(--ink)' }}>
            How would the agent behave on past sessions?
          </h3>

          {/* Fixture list */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 18 }}>
            {A.replayFixtures.map(f => (
              <button key={f.id} onClick={() => setActiveFix(f.id)} style={{
                textAlign: 'left', padding: '10px 12px', borderRadius: 4,
                background: activeFix === f.id ? 'var(--paper)' : 'transparent',
                border: activeFix === f.id ? '1px solid var(--ink-mute)' : 'var(--hairline)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)',
                display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                gap: 10, alignItems: 'center'
              }}>
                <span style={{
                  width: 8, height: 8, borderRadius: '50%',
                  background: f.result.passed ? 'var(--success)' : 'var(--warning)'
                }}/>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, color: 'var(--ink)',
                                 whiteSpace: 'nowrap', overflow: 'hidden',
                                 textOverflow: 'ellipsis' }}>{f.label}</div>
                  <div style={{ fontSize: 10.5, color: 'var(--ink-mute)' }}>{f.when}</div>
                </div>
                <span className="mono" style={{ fontSize: 10, color: 'var(--ink-mute)' }}>
                  {f.result.steps} steps
                </span>
              </button>
            ))}
          </div>

          {/* Fixture detail */}
          <div style={{ background: 'var(--paper)', border: 'var(--hairline)',
                         borderRadius: 6, padding: '16px 18px' }}>
            <div style={{ display: 'flex', alignItems: 'center',
                           justifyContent: 'space-between', marginBottom: 12 }}>
              <span style={{ fontSize: 12, color: 'var(--ink-soft)' }}>{fixture.label}</span>
              <span style={{ fontSize: 10.5, letterSpacing: '0.12em',
                              textTransform: 'uppercase',
                              color: fixture.result.passed ? 'var(--success)' : 'var(--warning)' }}>
                {fixture.result.passed ? "passed" : "diverged"}
              </span>
            </div>

            <div style={{ fontSize: 12, color: 'var(--ink-soft)', lineHeight: 1.55,
                           marginBottom: 14 }}>{fixture.description}</div>

            <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '4px 14px',
                           fontSize: 11.5, marginBottom: 14 }}>
              <span style={{ color: 'var(--ink-faint)' }}>Expected outcome</span>
              <span style={{ color: 'var(--ink)' }}>{fixture.correctOutcome}</span>
              <span style={{ color: 'var(--ink-faint)' }}>Steps</span>
              <span className="mono" style={{ color: 'var(--ink-soft)' }}>{fixture.result.steps}</span>
              <span style={{ color: 'var(--ink-faint)' }}>Duration</span>
              <span className="mono" style={{ color: 'var(--ink-soft)' }}>
                {(fixture.result.durationMs/1000).toFixed(1)}s
              </span>
              <span style={{ color: 'var(--ink-faint)' }}>Tool calls</span>
              <span className="mono" style={{ color: 'var(--ink-soft)' }}>{fixture.result.toolCalls}</span>
            </div>

            {fixture.result.divergence && (
              <div style={{ padding: '10px 12px', borderRadius: 4,
                             background: 'var(--paper-soft)',
                             borderLeft: '2px solid var(--warning)',
                             fontSize: 11.5, color: 'var(--ink-soft)', lineHeight: 1.55 }}>
                <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--warning)',
                                textTransform: 'uppercase', display: 'block', marginBottom: 4 }}>
                  Why it diverged
                </span>
                {fixture.result.divergence}
              </div>
            )}

            <div style={{ display: 'flex', gap: 8, marginTop: 14 }}>
              <button style={{
                padding: '7px 14px', fontSize: 12, background: 'var(--ink)',
                color: 'var(--paper)', borderRadius: 5, border: 'none',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
              }}>Replay  →</button>
              <button style={{
                padding: '7px 14px', fontSize: 12, background: 'transparent',
                color: 'var(--ink-soft)', borderRadius: 5,
                border: '1px solid var(--ink-mute)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
              }}>View trace</button>
              <span style={{ flex: 1 }}/>
              <button style={{ fontSize: 11, color: 'var(--ink-mute)',
                                background: 'transparent', border: 'none',
                                cursor: 'pointer' }}>+ add fixture</button>
            </div>
          </div>

          <div style={{ marginTop: 18, padding: '12px 14px', borderRadius: 5,
                         background: 'var(--paper)', border: 'var(--hairline)' }}>
            <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--ink-faint)',
                           textTransform: 'uppercase', marginBottom: 7 }}>
              Run all replays
            </div>
            <p style={{ fontSize: 11.5, color: 'var(--ink-mute)', margin: '0 0 10px',
                         lineHeight: 1.5 }}>
              Sensei reruns every fixture against the current agent definition.
              Use this before publishing a new version.
            </p>
            <button style={{
              padding: '7px 14px', fontSize: 12, background: 'var(--ink)',
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
    <section style={{ marginBottom: 26 }}>
      <div style={{ marginBottom: 12 }}>
        <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0,
                                          color: 'var(--ink)' }}>{title}</h3>
        {subtitle && (
          <div style={{ fontSize: 11.5, color: 'var(--ink-mute)', marginTop: 3 }}>{subtitle}</div>
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
      <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--ink-faint)',
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
      <div style={{ padding: '22px 36px 18px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 22 }}>
        <div className="kanji" style={{ fontSize: 42, color: 'var(--accent)', lineHeight: 1 }}>貌</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--ink-mute)',
                         textTransform: 'uppercase', marginBottom: 5 }}>
            Persona editor  ·  the hat sensei wears
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--ink)' }}>{P.name}</h1>
          <p style={{ fontSize: 12, color: 'var(--ink-soft)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>{P.description}</p>
        </div>
        <div style={{ paddingLeft: 22, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 22 }}>
          <AgMini n={P.rules.length} l="rules"/>
          <AgMini n={P.evidence.length} l="evidence cited" mono/>
          <AgMini n={P.assembled.tokenEstimate.toLocaleString()} l="tokens" mono accent/>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.1fr 1fr 1fr' }}>
        {/* Col 1: Triggers + assembled context */}
        <div style={{ overflow: 'auto', padding: '22px 28px',
                       borderRight: 'var(--hairline)' }}>
          <AgSection title="Triggers"
                     subtitle="When sensei dons this hat. ANDed clauses.">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {P.triggers.map((t, i) => (
                <div key={i} style={{ padding: '10px 12px', borderRadius: 4,
                                       border: 'var(--hairline)', background: 'var(--paper-soft)' }}>
                  <div style={{ fontSize: 12.5, color: 'var(--ink)',
                                 marginBottom: 4 }}>{t.label}</div>
                  <div className="mono" style={{ fontSize: 10.5, color: 'var(--ink-mute)' }}>
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
            <div style={{ padding: '14px 16px', background: 'var(--paper-soft)',
                           border: 'var(--hairline)', borderRadius: 5 }}>
              <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '5px 12px',
                             fontSize: 11.5, marginBottom: 10 }}>
                <span style={{ color: 'var(--ink-faint)' }}>Active rules</span>
                <span className="mono" style={{ color: 'var(--ink-soft)' }}>{P.assembled.activeRules}</span>
                <span style={{ color: 'var(--ink-faint)' }}>Memory refs loaded</span>
                <span className="mono" style={{ color: 'var(--ink-soft)' }}>{P.assembled.memoryRefsLoaded}</span>
                <span style={{ color: 'var(--ink-faint)' }}>Token estimate</span>
                <span className="mono" style={{ color: 'var(--ink-soft)' }}>{P.assembled.tokenEstimate.toLocaleString()}</span>
              </div>
              <pre className="mono" style={{ fontSize: 11, color: 'var(--ink-soft)',
                background: 'transparent', margin: 0, lineHeight: 1.6,
                whiteSpace: 'pre-wrap' }}>{P.assembled.systemSnippet}</pre>
            </div>
          </AgSection>
        </div>

        {/* Col 2: Rules */}
        <div style={{ overflow: 'auto', padding: '22px 26px',
                       borderRight: 'var(--hairline)' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                         marginBottom: 14 }}>
            <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0,
                                              color: 'var(--ink)' }}>Rules</h3>
            <span style={{ fontSize: 11, color: 'var(--ink-mute)' }}>
              short imperatives the persona embodies
            </span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {P.rules.map(r => (
              <button key={r.id} onClick={() => setActiveRule(r.id)} style={{
                textAlign: 'left', padding: '12px 14px', borderRadius: 5,
                background: activeRule === r.id ? 'var(--paper-mute)' : 'transparent',
                border: activeRule === r.id ? '1px solid var(--ink-mute)' : 'var(--hairline)',
                cursor: 'pointer', fontFamily: 'var(--font-ui)'
              }}>
                <div style={{ display: 'flex', alignItems: 'baseline', gap: 8,
                               marginBottom: 5 }}>
                  <span className="mono" style={{ fontSize: 10, color: 'var(--accent)' }}>
                    {r.id.toUpperCase()}
                  </span>
                  <span className="mono" style={{ fontSize: 10, color: 'var(--ink-faint)',
                                                    marginLeft: 'auto' }}>
                    {r.evidenceCount} citations
                  </span>
                </div>
                <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.5 }}>
                  {r.text}
                </div>
                <div style={{ fontSize: 10.5, color: 'var(--ink-faint)', marginTop: 5 }}>
                  last fired {r.lastFired}
                </div>
              </button>
            ))}
            <button style={{ fontSize: 11, color: 'var(--ink-mute)',
              background: 'transparent', border: '1px dashed var(--paper-4)',
              borderRadius: 4, padding: '10px', cursor: 'pointer',
              textAlign: 'center', marginTop: 6 }}>
              + add rule
            </button>
          </div>
        </div>

        {/* Col 3: Evidence trail for selected rule */}
        <div style={{ overflow: 'auto', padding: '22px 28px',
                       background: 'var(--paper-soft)' }}>
          <div style={{ marginBottom: 14 }}>
            <div style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--ink-faint)',
                           textTransform: 'uppercase', marginBottom: 6 }}>
              Evidence trail  ·  {rule.id.toUpperCase()}
            </div>
            <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.5,
                           padding: '10px 12px', background: 'var(--paper)',
                           border: 'var(--hairline)', borderRadius: 4 }}>
              "{rule.text}"
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-mute)', marginTop: 6 }}>
              Pulled live from sensei's memory store. Each row links a session
              where this rule shaped sensei's response.
            </div>
          </div>

          {ruleEvidence.length === 0 ? (
            <div style={{ padding: '20px', textAlign: 'center', fontSize: 12,
                           color: 'var(--ink-mute)' }}>
              No evidence cited for this rule yet.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {ruleEvidence.map(e => (
                <div key={e.memoryId} style={{ background: 'var(--paper)',
                  border: 'var(--hairline)', borderRadius: 5,
                  padding: '12px 14px' }}>
                  <div style={{ display: 'flex', alignItems: 'center',
                                 justifyContent: 'space-between', marginBottom: 7 }}>
                    <span className="mono" style={{ fontSize: 10.5, color: 'var(--accent)' }}>
                      {e.memoryId}
                    </span>
                    <span style={{ fontSize: 10.5, color: 'var(--ink-mute)' }}>{e.when}</span>
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--ink-soft)', lineHeight: 1.6,
                                 marginBottom: 7 }}>
                    {e.snippet}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10,
                                 fontSize: 10.5 }}>
                    <a href="#" style={{ color: 'var(--ink-mute)' }}>{e.sessionId}</a>
                    <span style={{ color: 'var(--ink-faint)' }}>·</span>
                    <a href="#" style={{ color: 'var(--ink-mute)' }}>view memory →</a>
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
