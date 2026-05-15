// Skill editor — two layouts.
//
// Layout A (form-style): one anatomy field per section. Live "assembled
// context" preview on the right shows what sensei will actually see when
// the skill triggers.
//
// Layout B (code-style): one document with frontmatter on top and a markdown
// body below. Right rail shows the same anatomy fields as inspector chips.
//
// Both layouts cover the same anatomy:
//   name · id · description · scope · triggers · tools · examples
//   evidence requirement · max-token budget · version · author · tags · body

const { useState: skS } = React;

const fieldLabel = {
  fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
  textTransform: 'uppercase', marginBottom: 8, fontFamily: 'var(--font-ui)'
};
const fieldBox = {
  border: 'var(--hairline)', borderRadius: 4, background: 'var(--paper)',
  padding: '8px 12px', fontSize: 13, color: 'var(--ink)',
  fontFamily: 'var(--font-ui)', width: '100%', boxSizing: 'border-box'
};
const monoBox = {
  ...fieldBox, fontFamily: 'var(--font-mono)', fontSize: 13,
  background: 'var(--paper-2)'
};

// ─── Shared bits ───────────────────────────────────────────
function SkHero({ skill, layout }) {
  return (
    <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center', gap: 24 }}>
      <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>技</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>
          Skill editor  ·  {layout === "form" ? "anatomy view" : "document view"}
        </div>
        <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                          color: 'var(--ink)' }}>
          {skill.name}
        </h1>
        <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                     maxWidth: 720, lineHeight: 1.55 }}>
          {skill.description}
        </p>
      </div>
      <div style={{ paddingLeft: 24, borderLeft: 'var(--hairline)',
                     display: 'flex', gap: 24, alignItems: 'flex-start' }}>
        <div style={{ textAlign: 'right' }}>
          <div className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>v{skill.version}</div>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                         textTransform: 'uppercase', marginTop: 4 }}>version</div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div style={{ fontSize: 13, color: 'var(--accent)' }}>{skill.evidence.required ? "required" : "optional"}</div>
          <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                         textTransform: 'uppercase', marginTop: 4 }}>evidence</div>
        </div>
        <button style={{
          padding: '8px 16px', fontSize: 13, background: 'var(--ink)',
          color: 'var(--paper)', borderRadius: 5, border: 'none',
          cursor: 'pointer', alignSelf: 'center', fontFamily: 'var(--font-ui)'
        }}>Save · v{skill.version.split(/[.-]/).slice(0,2).join('.')}.{Number(skill.version.split('.')[2].split('-')[0])+1}</button>
      </div>
    </div>
  );
}

// ─── Live "assembled context" preview ──────────────────────
function SkAssembledPreview({ skill }) {
  const a = skill.assembled;
  return (
    <div style={{ padding: '16px 24px', background: 'var(--paper-2)',
                   borderRadius: 6, border: 'var(--hairline)' }}>
      <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                     textTransform: 'uppercase', marginBottom: 12 }}>
        Assembled context  ·  what sensei sees on trigger
      </div>
      <pre className="mono" style={{ fontSize: 11, color: 'var(--ink-2)',
        background: 'transparent', margin: 0, lineHeight: 1.65, whiteSpace: 'pre-wrap' }}>
{`# system\n${a.systemSnippet}\n\n# memory\n${a.memorySnippet}\n\n# tools available\n${a.toolList.map(t => `  · ${t}`).join('\n')}`}
      </pre>
      <div style={{ borderTop: 'var(--hairline)', marginTop: 12, paddingTop: 8,
                     display: 'flex', justifyContent: 'space-between',
                     fontSize: 11, color: 'var(--ink-3)' }}>
        <span>Token estimate</span>
        <span className="mono" style={{ color: 'var(--ink-2)' }}>
          {a.tokenEstimate.toLocaleString()} / {skill.maxTokens.toLocaleString()}
        </span>
      </div>
      <div style={{ height: 4, background: 'var(--edge)', borderRadius: 2,
                     marginTop: 4, overflow: 'hidden' }}>
        <div style={{ width: `${100 * a.tokenEstimate / skill.maxTokens}%`,
                       height: '100%', background: 'var(--accent)' }}/>
      </div>
    </div>
  );
}

// ─── Trigger row ───────────────────────────────────────────
function SkTriggerRow({ t }) {
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '110px 70px 1fr auto',
                   gap: 8, alignItems: 'center',
                   padding: '8px 12px', borderRadius: 4,
                   background: 'var(--paper-2)', border: 'var(--hairline)' }}>
      <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>{t.kind}</span>
      <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>{t.op}</span>
      <span className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>{t.value}</span>
      <button style={{ fontSize: 13, color: 'var(--ink-4)', background: 'transparent',
        border: 'none', cursor: 'pointer' }}>×</button>
    </div>
  );
}

// ─── Tool checkbox row ─────────────────────────────────────
function SkToolRow({ tool }) {
  return (
    <label style={{ display: 'grid', gridTemplateColumns: '20px 1fr auto',
                     gap: 8, alignItems: 'center', padding: '8px 12px',
                     borderRadius: 4, cursor: 'pointer',
                     background: tool.allowed ? 'var(--paper-2)' : 'transparent',
                     border: 'var(--hairline)' }}>
      <span style={{
        width: 14, height: 14, borderRadius: 3,
        border: '1px solid ' + (tool.allowed ? 'var(--accent)' : 'var(--ink-4)'),
        background: tool.allowed ? 'var(--accent)' : 'transparent',
        display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
        color: 'var(--paper)', fontSize: 11
      }}>{tool.allowed ? '✓' : ''}</span>
      <span className="mono" style={{ fontSize: 13,
        color: tool.allowed ? 'var(--ink)' : 'var(--ink-3)' }}>{tool.label}</span>
      <span style={{ fontSize: 11, color: tool.allowed ? 'var(--success)' : 'var(--ink-4)',
                      letterSpacing: '0.12em', textTransform: 'uppercase' }}>
        {tool.allowed ? 'allowed' : 'denied'}
      </span>
    </label>
  );
}

// ─── Layout A: form-style ──────────────────────────────────
function SkillEditorFormStyle() {
  const skill = window.EXT_DATA.exampleSkill;

  return (
    <div className="sensei" data-screen-label="Skill editor · Form layout"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <SkHero skill={skill} layout="form"/>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.4fr 1fr', gap: 0 }}>
        {/* Left: anatomy form */}
        <div style={{ overflow: 'auto', padding: '24px 32px', borderRight: 'var(--hairline)' }}>
          <SkSection title="Identity">
            <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr', gap: 12 }}>
              <SkField label="Name"><input style={fieldBox} defaultValue={skill.name}/></SkField>
              <SkField label="ID"><input style={monoBox} defaultValue={skill.id} readOnly/></SkField>
            </div>
            <SkField label="Description">
              <textarea style={{ ...fieldBox, minHeight: 64, resize: 'vertical',
                lineHeight: 1.5 }} defaultValue={skill.description}/>
            </SkField>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
              <SkField label="Author"><input style={fieldBox} defaultValue={skill.author}/></SkField>
              <SkField label="Version"><input style={monoBox} defaultValue={skill.version}/></SkField>
              <SkField label="Scope">
                <select style={fieldBox} defaultValue={skill.scope}>
                  <option value="global">Global only</option>
                  <option value="either">Pinnable per-project</option>
                  <option value="project">Project only</option>
                </select>
              </SkField>
            </div>
            <SkField label="Tags">
              <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', alignItems: 'center' }}>
                {skill.tags.map(t => (
                  <span key={t} style={{ fontSize: 11, color: 'var(--ink-2)',
                    background: 'var(--paper-3)', padding: '4px 8px', borderRadius: 3,
                    fontFamily: 'var(--font-mono)' }}>{t} <span style={{ color: 'var(--ink-4)', marginLeft: 4 }}>×</span></span>
                ))}
                <button style={{ fontSize: 11, color: 'var(--ink-3)', background: 'transparent',
                  border: 'var(--hairline)', borderRadius: 3, padding: '4px 8px',
                  cursor: 'pointer' }}>+ tag</button>
              </div>
            </SkField>
          </SkSection>

          <SkSection title="Triggers" subtitle="When sensei should reach for this skill — all clauses ANDed">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {skill.triggers.map((t, i) => <SkTriggerRow key={i} t={t}/>)}
              <button style={{ fontSize: 11, color: 'var(--ink-3)',
                background: 'transparent', border: '1px dashed var(--edge)',
                borderRadius: 4, padding: '8px', cursor: 'pointer', textAlign: 'center' }}>
                + add clause
              </button>
            </div>
          </SkSection>

          <SkSection title="Tool access" subtitle="Which MCPs and tools the skill can call">
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 4 }}>
              {skill.tools.map(t => <SkToolRow key={t.id} tool={t}/>)}
            </div>
          </SkSection>

          <SkSection title="Examples" subtitle="Input → output pairs · drive evals + behavior">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              {skill.examples.map((ex, i) => (
                <div key={i} style={{ border: 'var(--hairline)', borderRadius: 6,
                  background: 'var(--paper-2)', padding: '12px 12px' }}>
                  <div style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                                 textTransform: 'uppercase', marginBottom: 4 }}>Input</div>
                  <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.55,
                                 marginBottom: 8 }}>{ex.in}</div>
                  <div style={{ fontSize: 11, color: 'var(--accent)', letterSpacing: '0.14em',
                                 textTransform: 'uppercase', marginBottom: 4 }}>Sensei's response</div>
                  <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.55 }}>{ex.out}</div>
                </div>
              ))}
              <button style={{ fontSize: 11, color: 'var(--ink-3)',
                background: 'transparent', border: '1px dashed var(--edge)',
                borderRadius: 4, padding: '8px', cursor: 'pointer', textAlign: 'center' }}>
                + example pair
              </button>
            </div>
          </SkSection>

          <SkSection title="Evidence requirement"
                     subtitle="Session signals that justify use — keeps the skill honest">
            <SkField label="Required signal">
              <input style={fieldBox} defaultValue={skill.evidence.signal}/>
            </SkField>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
              <SkField label="Sources">
                <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                  {skill.evidence.sources.map(s => (
                    <span key={s} className="mono" style={{ fontSize: 11,
                      color: 'var(--ink-2)', background: 'var(--paper-3)',
                      padding: '4px 8px', borderRadius: 3 }}>{s}</span>
                  ))}
                </div>
              </SkField>
              <SkField label="Memory refs">
                <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                  {skill.evidence.memoryRefs.map(m => (
                    <span key={m} className="mono" style={{ fontSize: 11,
                      color: 'var(--accent)', background: 'var(--paper-3)',
                      padding: '4px 8px', borderRadius: 3 }}>{m}</span>
                  ))}
                </div>
              </SkField>
            </div>
          </SkSection>

          <SkSection title="Token budget">
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <input style={{ ...monoBox, width: 120 }} defaultValue={skill.maxTokens}/>
              <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                ceiling for assembled context · current estimate {skill.assembled.tokenEstimate.toLocaleString()}
              </span>
            </div>
          </SkSection>

          <SkSection title="Skill body" subtitle="The prompt sensei brings to the agent">
            <textarea style={{ ...monoBox, minHeight: 220, resize: 'vertical',
              lineHeight: 1.5, fontFamily: 'var(--font-mono)' }} defaultValue={skill.body}/>
          </SkSection>
        </div>

        {/* Right: live preview */}
        <div style={{ overflow: 'auto', padding: '24px 24px',
                       background: 'var(--paper)' }}>
          <div style={{ position: 'sticky', top: 0 }}>
            <SkAssembledPreview skill={skill}/>

            {/* Validation panel */}
            <div style={{ marginTop: 16, padding: '16px 16px', borderRadius: 6,
                           border: 'var(--hairline)' }}>
              <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                             textTransform: 'uppercase', marginBottom: 12 }}>
                Health check
              </div>
              {[
                { ok: true,  label: "All triggers parse." },
                { ok: true,  label: "Tool whitelist is non-empty." },
                { ok: true,  label: "≥2 examples covering distinct cases." },
                { ok: false, label: "Body references a tool not in whitelist (`fs-write`)." },
                { ok: true,  label: "Evidence requirement is testable." },
              ].map((c, i) => (
                <div key={i} style={{ display: 'grid', gridTemplateColumns: '14px 1fr',
                                       gap: 8, alignItems: 'center', padding: '4px 0',
                                       fontSize: 13, color: c.ok ? 'var(--ink-2)' : 'var(--warning)' }}>
                  <span style={{ width: 14, textAlign: 'center', fontSize: 13 }}>
                    {c.ok ? "✓" : "!"}
                  </span>
                  <span>{c.label}</span>
                </div>
              ))}
            </div>

            {/* Test panel */}
            <div style={{ marginTop: 16, padding: '16px 16px', borderRadius: 6,
                           border: 'var(--hairline)', background: 'var(--paper-2)' }}>
              <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                             textTransform: 'uppercase', marginBottom: 8 }}>
                Try it · against past session
              </div>
              <select style={{ ...fieldBox, marginBottom: 8 }}>
                <option>lumen-app · 2025-10-04 boundary-thrash</option>
                <option>lumen-canvas · 2025-09-30 trait-leak</option>
              </select>
              <button style={{
                padding: '8px 12px', fontSize: 13, background: 'var(--ink)',
                color: 'var(--paper)', borderRadius: 5, border: 'none',
                cursor: 'pointer', width: '100%', fontFamily: 'var(--font-ui)'
              }}>Replay  →</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── Layout B: code-style ──────────────────────────────────
function SkillEditorCodeStyle() {
  const skill = window.EXT_DATA.exampleSkill;
  const frontmatter = `---
name: ${skill.name}
id: ${skill.id}
version: ${skill.version}
author: ${skill.author}
scope: ${skill.scope}
tags: [${skill.tags.join(', ')}]

triggers:
${skill.triggers.map(t => `  - ${t.kind} ${t.op} "${t.value}"`).join('\n')}

tools:
${skill.tools.filter(t=>t.allowed).map(t => `  - ${t.label}`).join('\n')}

evidence:
  required: ${skill.evidence.required}
  signal: "${skill.evidence.signal}"
  sources: [${skill.evidence.sources.join(', ')}]

max_tokens: ${skill.maxTokens}
---

`;

  return (
    <div className="sensei" data-screen-label="Skill editor · Code layout"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <SkHero skill={skill} layout="code"/>

      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1.6fr 1fr' }}>
        {/* Left: code document */}
        <div style={{ borderRight: 'var(--hairline)', display: 'flex',
                       flexDirection: 'column', minWidth: 0 }}>
          {/* tab strip */}
          <div style={{ display: 'flex', borderBottom: 'var(--hairline)',
                         background: 'var(--paper-2)' }}>
            {["skill.md", "examples.json", "evals.log"].map((t, i) => (
              <button key={t} style={{
                padding: '8px 16px', fontSize: 13,
                background: i === 0 ? 'var(--paper)' : 'transparent',
                borderRight: 'var(--hairline)',
                borderBottom: i === 0 ? 'none' : 'var(--hairline)',
                marginBottom: i === 0 ? -1 : 0,
                color: i === 0 ? 'var(--ink)' : 'var(--ink-3)',
                fontFamily: 'var(--font-mono)', cursor: 'pointer', border: 'none'
              }}>{t}</button>
            ))}
            <span style={{ flex: 1 }}/>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)',
                                              padding: '12px 16px' }}>
              utf-8 · markdown · ~{(frontmatter.length + skill.body.length) | 0} chars
            </span>
          </div>

          {/* code body */}
          <div style={{ flex: 1, overflow: 'auto', display: 'grid',
                         gridTemplateColumns: '40px 1fr' }}>
            <div style={{ background: 'var(--paper-2)', borderRight: 'var(--hairline)',
                           padding: '12px 0', textAlign: 'right',
                           fontFamily: 'var(--font-mono)', fontSize: 11,
                           color: 'var(--ink-4)', lineHeight: 1.65,
                           userSelect: 'none' }}>
              {Array.from({ length: (frontmatter + skill.body).split('\n').length }, (_, i) => (
                <div key={i} style={{ paddingRight: 8 }}>{i + 1}</div>
              ))}
            </div>
            <pre style={{ margin: 0, padding: '12px 16px',
                           fontFamily: 'var(--font-mono)', fontSize: 13,
                           color: 'var(--ink)', lineHeight: 1.65,
                           whiteSpace: 'pre-wrap' }}>
              <span style={{ color: 'var(--accent)' }}>{frontmatter}</span>
              <span>{skill.body}</span>
            </pre>
          </div>

          {/* status bar */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 16,
                         padding: '4px 16px', borderTop: 'var(--hairline)',
                         background: 'var(--paper-2)', fontSize: 11,
                         color: 'var(--ink-3)', fontFamily: 'var(--font-mono)' }}>
            <span>Ln 24, Col 1</span>
            <span>·</span>
            <span style={{ color: 'var(--success)' }}>● parsed</span>
            <span>·</span>
            <span>{skill.evidence.required ? "evidence required" : "evidence optional"}</span>
            <span style={{ flex: 1 }}/>
            <span>scope: {skill.scope}</span>
          </div>
        </div>

        {/* Right: inspector */}
        <div style={{ overflow: 'auto', padding: '16px 24px',
                       display: 'flex', flexDirection: 'column', gap: 12 }}>
          <InspectorChip k="Identity" rows={[
            ['name', skill.name],
            ['id', skill.id],
            ['version', skill.version],
            ['scope', skill.scope],
          ]}/>
          <InspectorChip k="Triggers" rows={skill.triggers.map(t =>
            [t.kind, `${t.op} "${t.value}"`]
          )}/>
          <InspectorChip k="Tools allowed" rows={skill.tools.filter(t=>t.allowed).map(t => [t.label, '✓'])}/>
          <InspectorChip k="Evidence" rows={[
            ['required', skill.evidence.required ? 'yes' : 'no'],
            ['signal', skill.evidence.signal],
            ['memory refs', skill.evidence.memoryRefs.length + ' linked'],
          ]}/>
          <InspectorChip k="Budget" rows={[
            ['max tokens', skill.maxTokens.toLocaleString()],
            ['estimate', skill.assembled.tokenEstimate.toLocaleString()],
            ['headroom', `${skill.maxTokens - skill.assembled.tokenEstimate} tokens`],
          ]}/>

          <div style={{ marginTop: 'auto' }}>
            <SkAssembledPreview skill={skill}/>
          </div>
        </div>
      </div>
    </div>
  );
}

function InspectorChip({ k, rows }) {
  return (
    <div style={{ border: 'var(--hairline)', borderRadius: 5,
                   padding: '12px 12px', background: 'var(--paper)' }}>
      <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                     textTransform: 'uppercase', marginBottom: 8 }}>{k}</div>
      <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '5px 12px',
                     fontSize: 11 }}>
        {rows.map(([l, v], i) => (
          <React.Fragment key={i}>
            <span className="mono" style={{ color: 'var(--ink-3)' }}>{l}</span>
            <span style={{ color: 'var(--ink)', wordBreak: 'break-word' }}>{v}</span>
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

// ─── Generic field/section primitives ─────────────────────
function SkField({ label, children }) {
  return (
    <div style={{ marginBottom: 12 }}>
      <div style={fieldLabel}>{label}</div>
      {children}
    </div>
  );
}
function SkSection({ title, subtitle, children }) {
  return (
    <section style={{ marginBottom: 24 }}>
      <div style={{ marginBottom: 12 }}>
        <h3 className="display" style={{ fontSize: 15, fontWeight: 400, margin: 0,
                                          color: 'var(--ink)' }}>{title}</h3>
        {subtitle && (
          <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
            {subtitle}
          </div>
        )}
      </div>
      {children}
    </section>
  );
}

window.SkillEditorFormStyle = SkillEditorFormStyle;
window.SkillEditorCodeStyle = SkillEditorCodeStyle;
