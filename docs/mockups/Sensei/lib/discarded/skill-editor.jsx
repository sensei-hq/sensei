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
    <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
      <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>技</div>
      <div className="flex-1 min-w-0" >
        <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
          Skill editor  ·  {layout === "form" ? "anatomy view" : "document view"}
        </div>
        <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
          {skill.name}
        </h1>
        <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
          {skill.description}
        </p>
      </div>
      <div className="gap-6 pl-6 border-l flex items-start" >
        <div className="text-right" >
          <div className="mono text-ink" style={{ fontSize: 13 }}>v{skill.version}</div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 text-ink-4 uppercase" >version</div>
        </div>
        <div className="text-right" >
          <div className="text-accent" style={{ fontSize: 13 }}>{skill.evidence.required ? "required" : "optional"}</div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 text-ink-4 uppercase" >evidence</div>
        </div>
        <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-4 bg-ink text-paper border-0 cursor-pointer self-center" >Save · v{skill.version.split(/[.-]/).slice(0,2).join('.')}.{Number(skill.version.split('.')[2].split('-')[0])+1}</button>
      </div>
    </div>
  );
}

// ─── Live "assembled context" preview ──────────────────────
function SkAssembledPreview({ skill }) {
  const a = skill.assembled;
  return (
    <div style={{
 borderRadius: 6 }} className="py-4 px-6 bg-paper-2 border border-paper-edge" >
      <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-3 text-ink-4 uppercase" >
        Assembled context  ·  what sensei sees on trigger
      </div>
      <pre className="mono m-0 text-ink-2 bg-transparent" style={{
 fontSize: 11, lineHeight: 1.65, whiteSpace: 'pre-wrap'
 }}>
{`# system\n${a.systemSnippet}\n\n# memory\n${a.memorySnippet}\n\n# tools available\n${a.toolList.map(t => `  · ${t}`).join('\n')}`}
      </pre>
      <div style={{
 fontSize: 11 }} className="mt-3 pt-2 border-t flex justify-between text-ink-3" >
        <span>Token estimate</span>
        <span className="mono text-ink-2" >
          {a.tokenEstimate.toLocaleString()} / {skill.maxTokens.toLocaleString()}
        </span>
      </div>
      <div style={{
 height: 4, background: 'var(--edge)', borderRadius: 2 }} className="mt-1 overflow-hidden" >
        <div className="h-full bg-accent" style={{ width: `${100 * a.tokenEstimate / skill.maxTokens}%` }}/>
      </div>
    </div>
  );
}

// ─── Trigger row ───────────────────────────────────────────
function SkTriggerRow({ t }) {
  return (
    <div style={{ gridTemplateColumns: '110px 70px 1fr auto', borderRadius: 4 }} className="gap-2 py-2 px-3 grid items-center bg-paper-2 border border-paper-edge" >
      <span className="mono text-accent" style={{ fontSize: 11 }}>{t.kind}</span>
      <span className="text-ink-3" style={{ fontSize: 11 }}>{t.op}</span>
      <span className="mono text-ink" style={{ fontSize: 13 }}>{t.value}</span>
      <button className="text-ink-4 bg-transparent border-0 cursor-pointer" style={{ fontSize: 13 }}>×</button>
    </div>
  );
}

// ─── Tool checkbox row ─────────────────────────────────────
function SkToolRow({ tool }) {
  return (
    <label style={{ gridTemplateColumns: '20px 1fr auto',
 borderRadius: 4,
 background: tool.allowed ? 'var(--paper-2)' : 'transparent' }} className="gap-2 py-2 px-3 grid items-center cursor-pointer border border-paper-edge" >
      <span className="inline-flex items-center justify-center text-paper" style={{
 width: 14, height: 14, borderRadius: 3,
 border: '1px solid ' + (tool.allowed ? 'var(--accent)' : 'var(--ink-4)'),
 background: tool.allowed ? 'var(--accent)' : 'transparent', fontSize: 11
 }}>{tool.allowed ? '✓' : ''}</span>
      <span className="mono" style={{ fontSize: 13,
        color: tool.allowed ? 'var(--ink)' : 'var(--ink-3)' }}>{tool.label}</span>
      <span className="uppercase" style={{ fontSize: 11, color: tool.allowed ? 'var(--success)' : 'var(--ink-4)',
 letterSpacing: '0.12em' }}>
        {tool.allowed ? 'allowed' : 'denied'}
      </span>
    </label>
  );
}

// ─── Layout A: form-style ──────────────────────────────────
function SkillEditorFormStyle() {
  const skill = window.EXT_DATA.exampleSkill;

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Skill editor · Form layout"
 >
      <SkHero skill={skill} layout="form"/>

      <div style={{
 gridTemplateColumns: '1.4fr 1fr'
 }} className="gap-0 flex-1 min-h-0 grid" >
        {/* Left: anatomy form */}
        <div className="py-6 px-8 overflow-auto border-r" >
          <SkSection title="Identity">
            <div style={{ gridTemplateColumns: '1.4fr 1fr' }} className="gap-3 grid" >
              <SkField label="Name"><input style={fieldBox} defaultValue={skill.name}/></SkField>
              <SkField label="ID"><input style={monoBox} defaultValue={skill.id} readOnly/></SkField>
            </div>
            <SkField label="Description">
              <textarea style={{ ...fieldBox, minHeight: 64, resize: 'vertical',
                lineHeight: 1.5 }} defaultValue={skill.description}/>
            </SkField>
            <div style={{ gridTemplateColumns: '1fr 1fr 1fr' }} className="gap-3 grid" >
              <SkField label="Author"><input style={fieldBox} defaultValue={skill.author} className="gap-1" /></SkField>
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
              <div className="flex flex-wrap items-center" >
                {skill.tags.map(t => (
                  <span key={t} style={{
 fontSize: 11, borderRadius: 3,
 fontFamily: 'var(--font-mono)'
 }} className="py-1 px-2 text-ink-2 bg-paper-3" >{t} <span className="ml-1 text-ink-4" >×</span></span>
                ))}
                <button style={{
 fontSize: 11, borderRadius: 3 }} className="py-1 px-2 text-ink-3 bg-transparent border border-paper-edge cursor-pointer" >+ tag</button>
              </div>
            </SkField>
          </SkSection>

          <SkSection title="Triggers" subtitle="When sensei should reach for this skill — all clauses ANDed">
            <div className="gap-1 flex flex-col" >
              {skill.triggers.map((t, i) => <SkTriggerRow key={i} t={t}/>)}
              <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 4 }} className="p-2 text-ink-3 bg-transparent cursor-pointer text-center" >
                + add clause
              </button>
            </div>
          </SkSection>

          <SkSection title="Tool access" subtitle="Which MCPs and tools the skill can call">
            <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-1 grid" >
              {skill.tools.map(t => <SkToolRow key={t.id} tool={t}/>)}
            </div>
          </SkSection>

          <SkSection title="Examples" subtitle="Input → output pairs · drive evals + behavior">
            <div className="gap-3 flex flex-col" >
              {skill.examples.map((ex, i) => (
                <div key={i} style={{ borderRadius: 6 }} className="py-3 px-3 border border-paper-edge bg-paper-2" >
                  <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-4 uppercase" >Input</div>
                  <div style={{
 fontSize: 13, lineHeight: 1.55
 }} className="mb-2 text-ink" >{ex.in}</div>
                  <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-accent uppercase" >Sensei's response</div>
                  <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.55 }}>{ex.out}</div>
                </div>
              ))}
              <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 4 }} className="p-2 text-ink-3 bg-transparent cursor-pointer text-center" >
                + example pair
              </button>
            </div>
          </SkSection>

          <SkSection title="Evidence requirement"
                     subtitle="Session signals that justify use — keeps the skill honest">
            <SkField label="Required signal">
              <input style={fieldBox} defaultValue={skill.evidence.signal} className="gap-3" />
            </SkField>
            <div className="grid" style={{ gridTemplateColumns: '1fr 1fr' }}>
              <SkField label="Sources">
                <div className="gap-1 flex flex-wrap" >
                  {skill.evidence.sources.map(s => (
                    <span key={s} className="mono py-1 px-2 text-ink-2 bg-paper-3" style={{
 fontSize: 11, borderRadius: 3
 }}>{s}</span>
                  ))}
                </div>
              </SkField>
              <SkField label="Memory refs">
                <div className="gap-1 flex flex-wrap" >
                  {skill.evidence.memoryRefs.map(m => (
                    <span key={m} className="mono py-1 px-2 text-accent bg-paper-3" style={{
 fontSize: 11, borderRadius: 3
 }}>{m}</span>
                  ))}
                </div>
              </SkField>
            </div>
          </SkSection>

          <SkSection title="Token budget">
            <div className="gap-3 flex items-center" >
              <input style={{ ...monoBox, width: 120 }} defaultValue={skill.maxTokens}/>
              <span className="text-ink-3" style={{ fontSize: 11 }}>
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
        <div className="py-6 px-6 overflow-auto bg-paper" >
          <div className="sticky" style={{ top: 0 }}>
            <SkAssembledPreview skill={skill}/>

            {/* Validation panel */}
            <div style={{
 borderRadius: 6 }} className="mt-4 py-4 px-4 border border-paper-edge" >
              <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-3 text-ink-4 uppercase" >
                Health check
              </div>
              {[
                { ok: true,  label: "All triggers parse." },
                { ok: true,  label: "Tool whitelist is non-empty." },
                { ok: true,  label: "≥2 examples covering distinct cases." },
                { ok: false, label: "Body references a tool not in whitelist (`fs-write`)." },
                { ok: true,  label: "Evidence requirement is testable." },
              ].map((c, i) => (
                <div key={i} style={{ gridTemplateColumns: '14px 1fr',
 fontSize: 13, color: c.ok ? 'var(--ink-2)' : 'var(--warning)'
 }} className="gap-2 py-1 px-0 grid items-center" >
                  <span className="text-center" style={{ width: 14, fontSize: 13 }}>
                    {c.ok ? "✓" : "!"}
                  </span>
                  <span>{c.label}</span>
                </div>
              ))}
            </div>

            {/* Test panel */}
            <div style={{
 borderRadius: 6 }} className="mt-4 py-4 px-4 border border-paper-edge bg-paper-2" >
              <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 text-ink-4 uppercase" >
                Try it · against past session
              </div>
              <select style={{ ...fieldBox }} className="mb-2" >
                <option>lumen-app · 2025-10-04 boundary-thrash</option>
                <option>lumen-canvas · 2025-09-30 trait-leak</option>
              </select>
              <button style={{
 fontSize: 13, borderRadius: 5, fontFamily: 'var(--font-ui)'
 }} className="py-2 px-3 bg-ink text-paper border-0 cursor-pointer w-full" >Replay  →</button>
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Skill editor · Code layout"
 >
      <SkHero skill={skill} layout="code"/>

      <div className="flex-1 min-h-0 grid" style={{
 gridTemplateColumns: '1.6fr 1fr' }}>
        {/* Left: code document */}
        <div className="border-r flex flex-col min-w-0" >
          {/* tab strip */}
          <div className="flex border-b bg-paper-2" >
            {["skill.md", "examples.json", "evals.log"].map((t, i) => (
              <button key={t} style={{
 fontSize: 13,
 background: i === 0 ? 'var(--paper)' : 'transparent',
 borderBottom: i === 0 ? 'none' : 'var(--hairline)',
 marginBottom: i === 0 ? -1 : 0,
 color: i === 0 ? 'var(--ink)' : 'var(--ink-3)',
 fontFamily: 'var(--font-mono)' }} className="py-2 px-4 border-r cursor-pointer border-0" >{t}</button>
            ))}
            <span className="flex-1" />
            <span className="mono py-3 px-4 text-ink-4" style={{
 fontSize: 11 }}>
              utf-8 · markdown · ~{(frontmatter.length + skill.body.length) | 0} chars
            </span>
          </div>

          {/* code body */}
          <div className="flex-1 overflow-auto grid" style={{
 gridTemplateColumns: '40px 1fr' }}>
            <div style={{
 fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.65,
 userSelect: 'none'
 }} className="py-3 px-0 bg-paper-2 border-r text-right text-ink-4" >
              {Array.from({ length: (frontmatter + skill.body).split('\n').length }, (_, i) => (
                <div key={i} className="pr-2" >{i + 1}</div>
              ))}
            </div>
            <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 13, lineHeight: 1.65,
 whiteSpace: 'pre-wrap'
 }} className="py-3 px-4 m-0 text-ink" >
              <span className="text-accent" >{frontmatter}</span>
              <span>{skill.body}</span>
            </pre>
          </div>

          {/* status bar */}
          <div style={{ fontSize: 11, fontFamily: 'var(--font-mono)'
 }} className="gap-4 py-1 px-4 flex items-center border-t bg-paper-2 text-ink-3" >
            <span>Ln 24, Col 1</span>
            <span>·</span>
            <span className="text-success" >● parsed</span>
            <span>·</span>
            <span>{skill.evidence.required ? "evidence required" : "evidence optional"}</span>
            <span className="flex-1" />
            <span>scope: {skill.scope}</span>
          </div>
        </div>

        {/* Right: inspector */}
        <div className="py-4 px-6 gap-3 overflow-auto flex flex-col" >
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
    <div style={{ borderRadius: 5 }} className="py-3 px-3 border border-paper-edge bg-paper" >
      <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 text-ink-4 uppercase" >{k}</div>
      <div className="grid" style={{ gridTemplateColumns: 'auto 1fr', gap: '4px 12px',
 fontSize: 11 }}>
        {rows.map(([l, v], i) => (
          <React.Fragment key={i}>
            <span className="mono text-ink-3" >{l}</span>
            <span className="text-ink" style={{ wordBreak: 'break-word' }}>{v}</span>
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

// ─── Generic field/section primitives ─────────────────────
function SkField({ label, children }) {
  return (
    <div className="mb-3">
      <div className="zs-eyebrow mb-1">{label}</div>
      {children}
    </div>
  );
}

function SkSection({ title, subtitle, children }) {
  return (
    <section className="mb-6">
      <div className="mb-3">
        <h3 className="display m-0 font-normal text-ink" style={{
 fontSize: 15 }}>{title}</h3>
        {subtitle && (
          <div className="mt-1 text-ink-3" style={{ fontSize: 11 }}>
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
