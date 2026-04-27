// Learnings — three simplified alternatives
//
// The original page packs a lot in: hero stats + recommendations inbox +
// 6 tabs + scope filter + project filter + sort + 4 different feed types.
// These three options each commit to one organising principle, so the page
// reads top-to-bottom without forcing the user to triangulate.
//
//   A · TRIAGE     — by immediacy: Now / Soon / Settled. No tabs.
//   B · ANATOMY    — one memory at a time. What / Why / How / Where laid out.
//   C · BRIEF      — single scrollable brief. Chart on top, then two grouped lists.
//
// All three share the existing window.LEARNINGS data so the user can compare
// like-for-like.

const { useState: l2S, useMemo: l2M } = React;

// ═══════════════════════════════════════════════════════════════════════
// Shared little bits
// ═══════════════════════════════════════════════════════════════════════
function L2Hero({ title, sub, kanji, right }) {
  return (
    <div style={{ padding: '22px 36px 16px', borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center', gap: 22, background: 'var(--paper)' }}>
      <div className="kanji" style={{ fontSize: 42, color: 'var(--shu)', lineHeight: 1 }}>{kanji}</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', marginBottom: 5 }}>
          Observatory · Learnings
        </div>
        <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                          color: 'var(--sumi)' }}>{title}</h1>
        <p style={{ fontSize: 12, color: 'var(--sumi-2)', margin: '4px 0 0',
                     maxWidth: 720, lineHeight: 1.55 }}>{sub}</p>
      </div>
      {right}
    </div>
  );
}

// Categorise a memory for the "what / why / how / where" anatomy.
// what  = memory.what
// why   = memory.because
// how   = inferred surface — skill / agent / command / inline rule
// where = scope (level + project + modules + stack)
function inferHow(memory) {
  // Heuristic that's good enough for this prototype.
  if (memory.category === "preference")  return { kind: "rule",    glyph: "則", label: "inline response rule",        target: "global response style" };
  if (memory.category === "convention")  return { kind: "skill",   glyph: "技", label: "skill",                       target: memory.scope.project ? `${memory.scope.project}-conventions.skill.md` : "global-style.skill.md" };
  if (memory.category === "anti_pattern")return { kind: "lint",    glyph: "禁", label: "lint check",                  target: "sensei lint" };
  if (memory.category === "pattern")     return { kind: "skill",   glyph: "技", label: "skill",                       target: `${memory.references.pattern || "pattern"}.skill.md` };
  // correctness — depends on scope
  if (memory.scope.modules && memory.scope.modules.length === 1) {
    return { kind: "agent", glyph: "作", label: "agent", target: `${memory.scope.modules[0].split("/")[0]}.agent.md` };
  }
  return { kind: "command", glyph: "令", label: "command",
           target: memory.scope.project ? `/${memory.scope.project}-check` : "/check" };
}

function L2WhereLine({ scope }) {
  const L = window.LEARNINGS;
  const bits = [scope.level];
  if (scope.project) bits.push(L.projects[scope.project]?.name || scope.project);
  if (scope.modules) bits.push(...scope.modules);
  if (scope.stack)   bits.push(...scope.stack);
  return (
    <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
      {bits.join(" · ")}
    </span>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// OPTION A · TRIAGE — by immediacy
//
// Three columns, no tabs:
//   ▸ Now      — violations + high-impact recommendations  (act this week)
//   ▸ Soon     — emerging patterns + medium recs           (worth a look)
//   ▸ Settled  — battle-tested memories                    (browsable, low-noise)
//
// Single project filter at the top. Strength is shown as a bar, not dots.
// ═══════════════════════════════════════════════════════════════════════
function LearningsTriage() {
  const L = window.LEARNINGS;
  const [project, setProject] = l2S("all");

  const inProj = (m) =>
    project === "all" ||
    m.scope?.project === project ||
    m.projects?.includes(project) ||
    m.projects?.includes("all");

  // NOW: violations · high-impact recs · top 2 corrections
  const now = {
    violations: L.memories.filter(m => m.violated > 0 && m.state !== "archived" && inProj(m)),
    recs:       L.recommendations.filter(r => r.impact === "high"),
    corrections:L.corrections.filter(c => inProj(c)).slice(0, 3)
  };

  // SOON: medium recs · emerging patterns · challenged memories
  const soon = {
    recs:       L.recommendations.filter(r => r.impact === "medium"),
    patterns:   L.patterns.filter(p => p.kind === "emerging" && (project === "all" || p.projects.includes(project))),
    challenged: L.memories.filter(m => m.state === "challenged" && inProj(m))
  };

  // SETTLED: battle-tested + reinforced memories, sorted by strength
  const settled = L.memories
    .filter(m => (m.state === "battle-tested" || m.state === "reinforced") && inProj(m) && m.violated === 0)
    .sort((a, b) => b.strength - a.strength);

  return (
    <div className="sensei" data-screen-label="Observatory · Learnings · Triage"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <L2Hero kanji="学" title="What needs you, what's working, what's quiet."
              sub="Memories sorted by immediacy. Most days you only need the first column."
              right={<TriageStats counts={L.counts}/>}/>

      {/* shared project filter */}
      <div style={{ padding: '12px 36px', borderBottom: 'var(--hairline)' }}>
        <ProjectFilter value={project} onChange={setProject} projects={L.projects}/>
      </div>

      <div style={{ flex: 1, overflow: 'auto', minHeight: 0,
                     display: 'grid', gridTemplateColumns: '1fr 1fr 1fr',
                     gap: 0 }}>
        <TriageColumn
          accent="var(--shu)"
          kanji="今"
          title="Now"
          sub="violations · high-impact recs · this week"
          count={now.violations.length + now.recs.length + now.corrections.length}>
          {now.violations.map(m => <ViolationCard key={m.id} memory={m}/>)}
          {now.recs.map(r => <RecCardSlim key={r.id} rec={r}/>)}
          {now.corrections.map(c => <CorrectionMini key={c.id} c={c}/>)}
          {(now.violations.length + now.recs.length + now.corrections.length === 0) &&
            <Quiet text="nothing urgent."/>}
        </TriageColumn>

        <TriageColumn
          accent="var(--amber)"
          kanji="近"
          title="Soon"
          sub="emerging patterns · worth a look"
          count={soon.recs.length + soon.patterns.length + soon.challenged.length}>
          {soon.recs.map(r => <RecCardSlim key={r.id} rec={r}/>)}
          {soon.patterns.map(p => <PatternMini key={p.id} pattern={p}/>)}
          {soon.challenged.map(m => <ChallengedMini key={m.id} memory={m}/>)}
          {(soon.recs.length + soon.patterns.length + soon.challenged.length === 0) &&
            <Quiet text="nothing brewing."/>}
        </TriageColumn>

        <TriageColumn
          accent="var(--jade)"
          kanji="定"
          title="Settled"
          sub="battle-tested · low noise · reference">
          {settled.map(m => <SettledRow key={m.id} memory={m}/>)}
        </TriageColumn>
      </div>
    </div>
  );
}

function TriageStats({ counts }) {
  return (
    <div style={{ display: 'flex', gap: 22, paddingLeft: 22, borderLeft: 'var(--hairline)' }}>
      <Mini n={counts.recs} l="to act" accent/>
      <Mini n={counts.memories} l="memories"/>
      <Mini n={`+${Math.round(counts.ftrFromMemory*100)}%`} l="ftr lift" mono/>
    </div>
  );
}
function Mini({ n, l, accent, mono }) {
  return (
    <div style={{ textAlign: 'center' }}>
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 18, lineHeight: 1, fontWeight: 300,
                     color: accent ? 'var(--shu)' : 'var(--sumi)',
                     fontFeatureSettings: '"tnum"' }}>{n}</div>
      <div style={{ fontSize: 9, letterSpacing: '0.12em', color: 'var(--sumi-4)',
                     marginTop: 3, textTransform: 'uppercase' }}>{l}</div>
    </div>
  );
}

function TriageColumn({ accent, kanji, title, sub, count, children }) {
  return (
    <div style={{ borderRight: 'var(--hairline)', display: 'flex',
                   flexDirection: 'column', minHeight: 0 }}>
      <div style={{ padding: '16px 22px 12px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'baseline', gap: 10 }}>
        <span className="kanji" style={{ fontSize: 22, color: accent, lineHeight: 1 }}>{kanji}</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 13, color: 'var(--sumi)', fontWeight: 500 }}>{title}</div>
          <div style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 2 }}>{sub}</div>
        </div>
        {count != null && (
          <span className="mono" style={{ fontSize: 11, color: accent,
                        fontFeatureSettings: '"tnum"' }}>{count}</span>
        )}
      </div>
      <div style={{ flex: 1, overflow: 'auto', padding: '12px 18px',
                     display: 'flex', flexDirection: 'column', gap: 8 }}>
        {children}
      </div>
    </div>
  );
}

function Quiet({ text }) {
  return (
    <div style={{ padding: '40px 8px', textAlign: 'center', fontSize: 11.5,
                   color: 'var(--sumi-4)', fontStyle: 'italic' }}>{text}</div>
  );
}

function ViolationCard({ memory }) {
  const how = inferHow(memory);
  return (
    <div style={{ background: 'var(--shu-soft)', border: '1px solid transparent',
                   borderLeft: '2px solid var(--shu)', borderRadius: 5,
                   padding: '10px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 5 }}>
        <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--shu)',
                        textTransform: 'uppercase', fontWeight: 500 }}>
          violated · {memory.violated}×
        </span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>{memory.lastRelevant}</span>
      </div>
      <div style={{ fontSize: 12.5, color: 'var(--sumi)', lineHeight: 1.4, fontWeight: 500 }}>
        {memory.what}
      </div>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 6 }}>
        <span className="kanji" style={{ color: 'var(--shu)' }}>{how.glyph}</span>{" "}
        reinforce in <span className="mono">{how.target}</span>
      </div>
    </div>
  );
}

function RecCardSlim({ rec }) {
  const kindGlyph = {
    "promote-pattern": "昇", "create-agent": "作", "write-skill": "技",
    "archive-memory": "納", "enrich-memory": "育", "cross-project": "渡"
  }[rec.kind] || "?";
  return (
    <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 5, padding: '10px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 5 }}>
        <span className="kanji" style={{ fontSize: 12, color: 'var(--shu)' }}>{kindGlyph}</span>
        <span style={{ fontSize: 9.5, color: 'var(--sumi-3)', letterSpacing: '0.14em',
                        textTransform: 'uppercase' }}>{rec.action}</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>{rec.targetKind}</span>
      </div>
      <div style={{ fontSize: 12, color: 'var(--sumi)', lineHeight: 1.4 }}>{rec.title}</div>
      <div style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 5,
                     lineHeight: 1.5 }}>{rec.reasoning}</div>
    </div>
  );
}

function CorrectionMini({ c }) {
  return (
    <div style={{ display: 'flex', alignItems: 'baseline', gap: 8,
                   padding: '7px 12px', background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 5 }}>
      <span className="kanji" style={{ fontSize: 11, color: 'var(--amber)' }}>直</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11.5, color: 'var(--sumi)', lineHeight: 1.4,
                       overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {c.text}
        </div>
      </div>
      <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-2)' }}>{c.count}×</span>
    </div>
  );
}

function PatternMini({ pattern }) {
  return (
    <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderLeft: '2px solid var(--amber)', borderRadius: 5,
                   padding: '9px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 7, marginBottom: 4 }}>
        <span className="mono" style={{ fontSize: 12, color: 'var(--sumi)' }}>{pattern.name}</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 10, color: 'var(--jade)' }}>
          FTR +{Math.round(pattern.ftrDelta*100)}%
        </span>
      </div>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', lineHeight: 1.45 }}>
        {pattern.occurrences} places · candidate to promote
      </div>
    </div>
  );
}

function ChallengedMini({ memory }) {
  return (
    <div style={{ background: 'var(--amber-soft)', border: '1px solid transparent',
                   borderRadius: 5, padding: '9px 12px' }}>
      <div style={{ fontSize: 9.5, color: 'var(--amber)', letterSpacing: '0.14em',
                     textTransform: 'uppercase', marginBottom: 4 }}>challenged</div>
      <div style={{ fontSize: 12, color: 'var(--sumi)', lineHeight: 1.4 }}>{memory.what}</div>
    </div>
  );
}

function SettledRow({ memory }) {
  return (
    <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, padding: '6px 0' }}>
      <StrengthBar value={memory.strength} compact/>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 12, color: 'var(--sumi)', lineHeight: 1.4 }}>{memory.what}</div>
        <L2WhereLine scope={memory.scope}/>
      </div>
    </div>
  );
}

function StrengthBar({ value, compact }) {
  return (
    <div title={`strength ${value}/5`}
         style={{ width: compact ? 24 : 60, height: 3, borderRadius: 2,
                   background: 'var(--paper-edge)', position: 'relative',
                   flexShrink: 0, marginTop: 6 }}>
      <div style={{ position: 'absolute', inset: 0,
                     width: `${(value/5)*100}%`,
                     background: value === 5 ? 'var(--jade)' : 'var(--sumi-2)',
                     borderRadius: 2 }}/>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// OPTION B · ANATOMY — memory laid out as What / Why / How / Where
//
// Center stage. Left rail: list of memories grouped by category, with the
// active item highlighted. Right of stage: a small chart of strength
// distribution and recent lifecycle. The four boxes — What / Why / How /
// Where — make the memory's anatomy literal.
// ═══════════════════════════════════════════════════════════════════════
function LearningsAnatomy() {
  const L = window.LEARNINGS;
  const active = L.memories.filter(m => m.state !== "archived");
  const groups = [
    { id: "correctness", label: "Correctness", glyph: "正" },
    { id: "convention",  label: "Conventions", glyph: "流" },
    { id: "preference",  label: "Preferences", glyph: "好" },
    { id: "pattern",     label: "Patterns",    glyph: "紋" }
  ];
  const [openId, setOpen] = l2S(active[0].id);
  const memory = active.find(m => m.id === openId) || active[0];

  return (
    <div className="sensei" data-screen-label="Observatory · Learnings · Anatomy"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <L2Hero kanji="覚" title="Every memory has the same anatomy."
              sub="What it is · why it matters · how it's surfaced · where it applies."
              right={<HealthChart memories={active}/>}/>

      <div style={{ flex: 1, display: 'grid',
                     gridTemplateColumns: '260px 1fr',
                     minHeight: 0 }}>
        {/* List rail */}
        <aside style={{ borderRight: 'var(--hairline)', overflow: 'auto',
                         padding: '14px 0' }}>
          {groups.map(g => {
            const items = active.filter(m => m.category === g.id ||
              (g.id === "pattern" && m.category === "anti_pattern"));
            if (items.length === 0) return null;
            return (
              <div key={g.id} style={{ marginBottom: 14 }}>
                <div style={{ padding: '4px 18px 6px', display: 'flex',
                               alignItems: 'baseline', gap: 8 }}>
                  <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>{g.glyph}</span>
                  <span style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                                  textTransform: 'uppercase' }}>{g.label}</span>
                  <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>{items.length}</span>
                </div>
                {items.map(m => (
                  <button key={m.id} onClick={() => setOpen(m.id)}
                          style={{ width: '100%', textAlign: 'left',
                                    padding: '7px 18px 7px 22px',
                                    background: openId === m.id ? 'var(--paper-2)' : 'transparent',
                                    borderLeft: openId === m.id ? '2px solid var(--shu)' : '2px solid transparent',
                                    cursor: 'pointer' }}>
                    <div style={{ fontSize: 12, color: 'var(--sumi)', lineHeight: 1.4,
                                   display: '-webkit-box', WebkitLineClamp: 2,
                                   WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
                      {m.what}
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 4 }}>
                      <StrengthBar value={m.strength} compact/>
                      {m.violated > 0 && (
                        <span className="mono" style={{ fontSize: 10, color: 'var(--shu)' }}>
                          {m.violated}×broken
                        </span>
                      )}
                    </div>
                  </button>
                ))}
              </div>
            );
          })}
        </aside>

        {/* Anatomy stage */}
        <main style={{ overflow: 'auto', padding: '24px 36px 36px' }}>
          <AnatomyStage memory={memory}/>
        </main>
      </div>
    </div>
  );
}

function AnatomyStage({ memory }) {
  const L = window.LEARNINGS;
  const how = inferHow(memory);
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 18 }}>
      {/* Title + meta */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 6 }}>
          <span style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                          textTransform: 'uppercase' }}>{memory.category.replace("_", "-")}</span>
          <span style={{ fontSize: 9.5, color: 'var(--sumi-4)' }}>·</span>
          <span style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>{memory.state}</span>
          <span style={{ flex: 1 }}/>
          <StrengthBar value={memory.strength}/>
          <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-2)' }}>
            {memory.strength}/5
          </span>
        </div>
        <h2 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                          color: 'var(--sumi)', lineHeight: 1.3 }}>
          {memory.what}
        </h2>
      </div>

      {/* 2×2 anatomy grid */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr',
                     gridTemplateRows: 'auto auto', gap: 14 }}>
        <AnatomyBlock label="What" kanji="何" tone="ink">
          <div style={{ fontSize: 13.5, color: 'var(--sumi)', lineHeight: 1.5 }}>
            {memory.what}
          </div>
        </AnatomyBlock>

        <AnatomyBlock label="Why · because" kanji="故" tone="ink">
          <div style={{ fontSize: 13, color: 'var(--sumi)', lineHeight: 1.55 }}>
            {memory.because}
          </div>
        </AnatomyBlock>

        <AnatomyBlock label="How · surface" kanji={how.glyph} tone="shu">
          <div style={{ fontSize: 12, color: 'var(--sumi-2)', lineHeight: 1.5,
                         marginBottom: 6 }}>
            sensei surfaces this as a{" "}
            <span style={{ color: 'var(--sumi)', fontWeight: 500 }}>{how.label}</span>
          </div>
          <div className="mono" style={{ fontSize: 11.5, padding: '6px 10px',
                       background: 'var(--paper)', borderRadius: 4,
                       border: 'var(--hairline)', color: 'var(--sumi)' }}>
            {how.target}
          </div>
          {memory.references.good_example && (
            <div className="mono" style={{ fontSize: 10.5, color: 'var(--jade)', marginTop: 8 }}>
              ✓ {memory.references.good_example}
            </div>
          )}
          {memory.references.bad_example && (
            <div className="mono" style={{ fontSize: 10.5, color: 'var(--shu)' }}>
              ✗ {memory.references.bad_example}
            </div>
          )}
        </AnatomyBlock>

        <AnatomyBlock label="Where · scope" kanji="域" tone="jade">
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5, marginBottom: 8 }}>
            {scopeChips(memory.scope, L).map((c, i) => (
              <span key={i} className={c.mono ? "mono" : ""}
                    style={{ fontSize: 10.5, padding: '2px 8px',
                              background: 'var(--paper)', border: 'var(--hairline)',
                              borderRadius: 12, color: c.dim ? 'var(--sumi-3)' : 'var(--sumi-2)',
                              textTransform: c.upper ? 'uppercase' : 'none',
                              letterSpacing: c.upper ? '0.12em' : 0 }}>
                {c.label}: {c.value}
              </span>
            ))}
          </div>
          <div style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
            Learned {memory.learned} · last relevant {memory.lastRelevant} · reinforced {memory.reinforced}× · violated {memory.violated}×
          </div>
        </AnatomyBlock>
      </div>

      {/* Actions */}
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', paddingTop: 4 }}>
        <FlatBtn glyph="昇" label="Promote to rule"/>
        <FlatBtn glyph="育" label="Enrich"/>
        <FlatBtn glyph="渡" label="Apply elsewhere"/>
        <FlatBtn glyph="疑" label="Challenge"/>
        <span style={{ flex: 1 }}/>
        <FlatBtn glyph="納" label="Archive" subtle/>
      </div>
    </div>
  );
}

function scopeChips(scope, L) {
  const chips = [{ label: "level", value: scope.level, upper: true, dim: true }];
  if (scope.project) {
    const p = L.projects[scope.project];
    chips.push({ label: "project", value: p ? p.name : scope.project });
  }
  (scope.modules || []).forEach(m => chips.push({ label: "module", value: m, mono: true }));
  (scope.taskTypes || []).forEach(t => chips.push({ label: "task", value: t }));
  (scope.stack || []).forEach(s => chips.push({ label: "stack", value: s, mono: true }));
  return chips;
}

function AnatomyBlock({ label, kanji, tone, children }) {
  const accent = tone === "shu" ? "var(--shu)" :
                 tone === "jade" ? "var(--jade)" : "var(--sumi-2)";
  return (
    <section style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 8, padding: '14px 16px',
                       display: 'flex', flexDirection: 'column' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 8 }}>
        <span className="kanji" style={{ fontSize: 16, color: accent, lineHeight: 1 }}>{kanji}</span>
        <span style={{ fontSize: 10, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                        textTransform: 'uppercase' }}>{label}</span>
      </div>
      {children}
    </section>
  );
}

function FlatBtn({ glyph, label, subtle }) {
  return (
    <button style={{ padding: '7px 12px', fontSize: 11.5,
                      background: 'var(--paper-2)', border: 'var(--hairline)',
                      borderRadius: 5, color: subtle ? 'var(--sumi-3)' : 'var(--sumi)',
                      cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 7 }}>
      <span className="kanji" style={{ fontSize: 13,
                    color: subtle ? 'var(--sumi-3)' : 'var(--shu)' }}>{glyph}</span>
      {label}
    </button>
  );
}

// Tiny chart of memories grouped by strength
function HealthChart({ memories }) {
  const buckets = [0, 0, 0, 0, 0]; // strength 1..5
  memories.forEach(m => { if (m.strength >= 1) buckets[m.strength - 1]++; });
  const max = Math.max(...buckets, 1);
  const W = 130, H = 46, gap = 4, bw = (W - gap*4) / 5;
  return (
    <div style={{ paddingLeft: 22, borderLeft: 'var(--hairline)' }}>
      <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-4)',
                     textTransform: 'uppercase', marginBottom: 6 }}>
        memory strength
      </div>
      <svg width={W} height={H + 14} style={{ display: 'block' }}>
        {buckets.map((n, i) => {
          const h = (n / max) * H;
          return (
            <g key={i}>
              <rect x={i*(bw+gap)} y={H - h} width={bw} height={h}
                    fill={i === 4 ? 'var(--jade)' : i >= 2 ? 'var(--sumi-2)' : 'var(--sumi-3)'}
                    rx={1}/>
              <text x={i*(bw+gap) + bw/2} y={H + 11}
                    fontSize={9} fill="var(--sumi-4)" textAnchor="middle"
                    fontFamily="JetBrains Mono">{i+1}</text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// OPTION C · BRIEF — single scrollable brief
//
// One column. Top: a small chart (strength distribution by category).
// Then two simple grouped lists:
//   ▸ "Things to act on"  — recommendations + corrections merged, sorted by impact
//   ▸ "What I've learned" — memories grouped by category, collapsed sections
// One filter: project. No tabs, no sort dropdown.
// ═══════════════════════════════════════════════════════════════════════
function LearningsBrief() {
  const L = window.LEARNINGS;
  const [project, setProject] = l2S("all");
  const [openSection, setOpenSection] = l2S("correctness");

  const inProj = (m) =>
    project === "all" ||
    m.scope?.project === project ||
    m.projects?.includes(project) ||
    m.projects?.includes("all");

  // Merge recs + corrections into a single "to act on" list, sorted by impact
  const impactOrder = { high: 0, medium: 1, low: 2 };
  const acts = [
    ...L.recommendations.map(r => ({
      kind: "rec", id: r.id, title: r.title, why: r.reasoning,
      impact: r.impact, target: `${r.targetKind} · ${r.targetName}`,
      cta: r.action, recKind: r.kind
    })),
    ...L.corrections.filter(c => inProj(c)).map(c => ({
      kind: "correction", id: c.id, title: c.text, why: c.suggestion,
      impact: c.count >= 5 ? "high" : c.count >= 3 ? "medium" : "low",
      target: `seen ${c.count}× · last ${c.lastSeen}`,
      cta: "Reinforce", recKind: "enrich-memory"
    }))
  ].sort((a, b) => impactOrder[a.impact] - impactOrder[b.impact]);

  const cats = [
    { id: "correctness", label: "Correctness",  glyph: "正", desc: "facts that prevent regressions" },
    { id: "convention",  label: "Conventions",  glyph: "流", desc: "house style and shape" },
    { id: "preference",  label: "Preferences",  glyph: "好", desc: "voice, response style" },
    { id: "pattern",     label: "Patterns",     glyph: "紋", desc: "code shapes — adopted, anti, emerging" }
  ];

  return (
    <div className="sensei" data-screen-label="Observatory · Learnings · Brief"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <L2Hero kanji="学" title="A daily brief of what sensei knows."
              sub="Two questions only — what should I act on, and what's been learned."/>

      <div style={{ flex: 1, overflow: 'auto', minHeight: 0,
                     padding: '20px 36px 40px', display: 'flex',
                     flexDirection: 'column', gap: 24 }}>

        {/* Memory shape chart */}
        <MemoryShapeChart memories={L.memories.filter(m => m.state !== "archived" && inProj(m))}/>

        {/* shared project filter */}
        <ProjectFilter value={project} onChange={setProject} projects={L.projects}/>

        {/* Things to act on */}
        <section>
          <BriefHeader kanji="令" title="Things to act on"
                       sub="recommendations and recurring corrections, by impact"
                       count={acts.length}/>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {acts.map(a => <ActRow key={a.id} act={a}/>)}
          </div>
        </section>

        {/* What I've learned — collapsible per category */}
        <section>
          <BriefHeader kanji="覚" title="What I've learned"
                       sub="memories grouped by what they govern"
                       count={L.memories.filter(m => m.state !== "archived" && inProj(m)).length}/>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {cats.map(c => {
              const items = L.memories.filter(m =>
                m.state !== "archived" && inProj(m) &&
                (m.category === c.id ||
                  (c.id === "pattern" && m.category === "anti_pattern")));
              return (
                <CategorySection key={c.id} cat={c} items={items}
                                  open={openSection === c.id}
                                  onToggle={() => setOpenSection(openSection === c.id ? null : c.id)}/>
              );
            })}
          </div>
        </section>
      </div>
    </div>
  );
}

function BriefHeader({ kanji, title, sub, count }) {
  return (
    <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 12,
                   paddingBottom: 8, borderBottom: 'var(--hairline)' }}>
      <span className="kanji" style={{ fontSize: 16, color: 'var(--shu)' }}>{kanji}</span>
      <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0,
                    color: 'var(--sumi)' }}>{title}</h3>
      <span style={{ fontSize: 11, color: 'var(--sumi-3)' }}>· {sub}</span>
      <span style={{ flex: 1 }}/>
      <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>{count}</span>
    </div>
  );
}

function ActRow({ act }) {
  const dot = act.impact === "high" ? 'var(--shu)' :
              act.impact === "medium" ? 'var(--amber)' : 'var(--sumi-4)';
  return (
    <article style={{ display: 'grid',
                       gridTemplateColumns: '14px 1fr auto',
                       gap: 14, alignItems: 'start',
                       padding: '11px 14px',
                       background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 5 }}>
      <span style={{ width: 7, height: 7, borderRadius: '50%', background: dot,
                      marginTop: 6 }}/>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 13, color: 'var(--sumi)', lineHeight: 1.4,
                       fontWeight: 500 }}>{act.title}</div>
        <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55,
                       marginTop: 4 }}>{act.why}</div>
        <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
                       marginTop: 6 }}>{act.target}</div>
      </div>
      <button style={{ padding: '6px 12px', fontSize: 11,
                        background: act.impact === "high" ? 'var(--sumi)' : 'transparent',
                        color: act.impact === "high" ? 'var(--paper)' : 'var(--sumi-2)',
                        border: act.impact === "high" ? 'none' : '1px solid var(--paper-edge)',
                        borderRadius: 4, cursor: 'pointer', whiteSpace: 'nowrap' }}>
        {act.cta} →
      </button>
    </article>
  );
}

function CategorySection({ cat, items, open, onToggle }) {
  return (
    <div style={{ border: 'var(--hairline)', borderRadius: 6,
                   background: 'var(--paper-2)', overflow: 'hidden' }}>
      <button onClick={onToggle}
              style={{ width: '100%', padding: '11px 16px',
                        display: 'flex', alignItems: 'center', gap: 12,
                        background: 'transparent', border: 'none', cursor: 'pointer',
                        textAlign: 'left' }}>
        <span className="kanji" style={{ fontSize: 14, color: 'var(--shu)' }}>{cat.glyph}</span>
        <span style={{ fontSize: 13, color: 'var(--sumi)', fontWeight: 500 }}>{cat.label}</span>
        <span style={{ fontSize: 11, color: 'var(--sumi-3)' }}>· {cat.desc}</span>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>{items.length}</span>
        <span style={{ fontSize: 11, color: 'var(--sumi-3)',
                        transform: open ? 'rotate(90deg)' : 'rotate(0deg)',
                        transition: 'transform 0.15s' }}>›</span>
      </button>
      {open && items.length > 0 && (
        <div style={{ borderTop: 'var(--hairline)', padding: '6px 16px 14px' }}>
          {items.map(m => <BriefMemoryLine key={m.id} memory={m}/>)}
        </div>
      )}
    </div>
  );
}

function BriefMemoryLine({ memory }) {
  const how = inferHow(memory);
  return (
    <div style={{ display: 'grid',
                   gridTemplateColumns: '52px 1fr 130px',
                   gap: 14, alignItems: 'start', padding: '9px 0',
                   borderBottom: '1px dashed var(--paper-edge)' }}>
      <StrengthBar value={memory.strength}/>
      <div>
        <div style={{ fontSize: 12.5, color: 'var(--sumi)', lineHeight: 1.45,
                       fontWeight: 500 }}>{memory.what}</div>
        <div style={{ fontSize: 11, color: 'var(--sumi-2)', lineHeight: 1.55,
                       marginTop: 3, fontStyle: 'italic' }}>
          because <span style={{ fontStyle: 'normal' }}>{memory.because}</span>
        </div>
        <div style={{ display: 'flex', gap: 12, marginTop: 5,
                       fontSize: 10, color: 'var(--sumi-3)' }}>
          <span>
            <span className="kanji" style={{ color: 'var(--shu)', marginRight: 4 }}>{how.glyph}</span>
            {how.label}
          </span>
          <L2WhereLine scope={memory.scope}/>
        </div>
      </div>
      <div style={{ textAlign: 'right' }}>
        {memory.violated > 0 ? (
          <span className="mono" style={{ fontSize: 10.5, color: 'var(--shu)' }}>
            {memory.violated}× violated
          </span>
        ) : (
          <span style={{ fontSize: 10, color: 'var(--sumi-4)',
                          letterSpacing: '0.12em', textTransform: 'uppercase' }}>
            {memory.state}
          </span>
        )}
        <div style={{ fontSize: 10, color: 'var(--sumi-4)', marginTop: 2 }}>
          {memory.lastRelevant}
        </div>
      </div>
    </div>
  );
}

// Stacked-bar chart: each category shows count of memories at strength 1..5
function MemoryShapeChart({ memories }) {
  const cats = [
    { id: "correctness", label: "correctness", color: 'var(--shu)' },
    { id: "convention",  label: "conventions", color: 'var(--sumi-2)' },
    { id: "preference",  label: "preferences", color: 'var(--jade)' },
    { id: "pattern",     label: "patterns",    color: 'var(--amber)' }
  ];

  const total = memories.length || 1;
  const W = 880, H = 60;
  let x = 0;
  const bars = cats.map(c => {
    const items = memories.filter(m => m.category === c.id ||
      (c.id === "pattern" && m.category === "anti_pattern"));
    const w = (items.length / total) * W;
    const seg = { ...c, items, x, w };
    x += w;
    return seg;
  });

  // Average strength by category
  const meta = bars.map(b => {
    const avg = b.items.length === 0 ? 0 :
      b.items.reduce((s, m) => s + m.strength, 0) / b.items.length;
    const violations = b.items.reduce((s, m) => s + (m.violated || 0), 0);
    return { ...b, avg, violations };
  });

  return (
    <section style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 8, padding: '18px 20px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 14, color: 'var(--shu)' }}>形</span>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                        textTransform: 'uppercase' }}>memory shape</span>
        <span style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
          · {memories.length} memories · proportions show category mix · each row tracks average strength
        </span>
      </div>

      {/* Composition bar */}
      <svg width={W} height={H} style={{ display: 'block', maxWidth: '100%' }}
           viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none">
        {meta.map(b => (
          <g key={b.id}>
            <rect x={b.x + 1} y={0} width={Math.max(0, b.w - 2)} height={H - 22}
                  fill={b.color} opacity={0.18} rx={3}/>
            <rect x={b.x + 1} y={H - 22 - (b.avg/5)*(H-22)}
                  width={Math.max(0, b.w - 2)} height={(b.avg/5)*(H-22)}
                  fill={b.color} opacity={0.55} rx={3}/>
            {b.violations > 0 && (
              <circle cx={b.x + b.w - 8} cy={8} r={3.5}
                      fill="var(--shu)"/>
            )}
            <text x={b.x + 6} y={H - 6} fontSize={10}
                  fill="var(--sumi-3)" fontFamily="Inter">
              {b.label} · {b.items.length}
            </text>
          </g>
        ))}
      </svg>

      <div style={{ display: 'flex', justifyContent: 'space-between',
                     fontSize: 10, color: 'var(--sumi-4)', marginTop: 8 }}>
        <span>area = share of memories · fill height = avg strength · ● = active violations</span>
        <span className="mono">avg {(memories.reduce((s,m)=>s+m.strength,0)/Math.max(memories.length,1)).toFixed(1)}/5</span>
      </div>
    </section>
  );
}

Object.assign(window, { LearningsTriage, LearningsAnatomy, LearningsBrief });
