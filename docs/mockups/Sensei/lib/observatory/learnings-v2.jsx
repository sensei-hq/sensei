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
// Delegates to the shared ScreenHeader so the Learnings band is identical to
// every other destination's pinned header.
function L2Hero({ title, sub, kanji, right }) {
  return (
    <ScreenHeader kanji={kanji} eyebrow="Observatory · Learnings"
                  title={title} sub={sub} right={right}/>
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
    <span className="mono text-ink-3" style={{ fontSize: 11 }}>
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Learnings · Triage"
 >
      <L2Hero kanji="学" title="What needs you, what's working, what's quiet."
              sub="Memories sorted by immediacy. Most days you only need the first column."
              right={<TriageStats counts={L.counts}/>}/>

      {/* shared project filter */}
      <div className="py-3 px-8 border-b" >
        <ProjectFilter value={project} onChange={setProject} projects={L.projects}/>
      </div>

      <div style={{ gridTemplateColumns: '1fr 1fr 1fr'
 }} className="gap-0 flex-1 overflow-auto min-h-0 grid" >
        <TriageColumn
          accent="var(--accent)"
          kanji="今"
          title="Now"
          sub="violations · high-impact recs · this week"
          count={now.violations.length + now.recs.length + now.corrections.length}>
          {now.violations.map((m, i) => <ViolationCard key={m.id} memory={m} focal={i === 0}/>)}
          {now.recs.map((r, i) => <RecCardSlim key={r.id} rec={r} focal={i === 0 && now.violations.length === 0}/>)}
          {now.corrections.map(c => <CorrectionMini key={c.id} c={c}/>)}
          {(now.violations.length + now.recs.length + now.corrections.length === 0) &&
            <Quiet text="nothing urgent."/>}
        </TriageColumn>

        <TriageColumn
          accent="var(--warning)"
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
          accent="var(--success)"
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
    <div className="gap-6 pl-6 flex border-l" >
      <Mini n={counts.recs} l="to act" accent/>
      <Mini n={counts.memories} l="memories"/>
      <Mini n={`+${Math.round(counts.ftrFromMemory*100)}%`} l="ftr lift" mono/>
    </div>
  );
}
function Mini({ n, l, accent, mono }) {
  return (
    <div className="text-center" >
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 17, lineHeight: 1, fontWeight: 300,
                     color: accent ? 'var(--accent)' : 'var(--ink)',
                     fontFeatureSettings: '"tnum"' }}>{n}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mt-1 text-ink-4 uppercase" >{l}</div>
    </div>
  );
}

function TriageColumn({ accent, kanji, title, sub, count, children }) {
  return (
    <div className="border-r flex flex-col min-h-0" >
      <div className="gap-2 pt-4 pb-3 px-6 border-b flex items-baseline" >
        <span className="kanji" style={{ fontSize: 22, color: accent, lineHeight: 1 }}>{kanji}</span>
        <div className="flex-1" >
          <div className="text-ink font-medium" style={{ fontSize: 13 }}>{title}</div>
          <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >{sub}</div>
        </div>
        {count != null && (
          <span className="mono" style={{ fontSize: 11, color: accent,
                        fontFeatureSettings: '"tnum"' }}>{count}</span>
        )}
      </div>
      <div className="py-3 px-4 gap-2 flex-1 overflow-auto flex flex-col" >
        {children}
      </div>
    </div>
  );
}

function Quiet({ text }) {
  return (
    <div style={{ fontSize: 11 }} className="py-8 px-2 text-center text-ink-4 italic" >{text}</div>
  );
}

function ViolationCard({ memory, focal = false }) {
  const how = inferHow(memory);
  const [status, setStatus] = l2S(null); // null | "applied" | "dismissed"

  if (status) {
    const applied = status === "applied";
    return (
      <div style={{ gap: 8,
 background: applied ? 'var(--success-soft)' : 'var(--paper-2)', borderRadius: 5, opacity: applied ? 1 : 0.6
 }} className="py-2 px-3 flex items-center border border-paper-edge" >
        <span className="kanji" style={{ fontSize: 12,
                      color: applied ? 'var(--success)' : 'var(--ink-4)' }}>
          {applied ? "✓" : "—"}
        </span>
        <span className="flex-1 text-ink-2 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 11, lineHeight: 1.4 }}>
          {applied ? <>Reinforced · <span className="mono">{how.target}</span></> : "Muted for now"}
        </span>
        <button className="text-ink-3" onClick={() => setStatus(null)}
 style={{ fontSize: 11 }}>Undo</button>
      </div>
    );
  }

  return (
    <div style={{ border: '1px solid transparent',
 borderLeft: '2px solid var(--accent)', borderRadius: 5
 }} className="py-2 px-3 bg-accent-soft" >
      <div className="gap-1 mb-1 flex items-center" >
        <span className="text-accent uppercase font-medium" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
          violated · {memory.violated}×
        </span>
        <span className="flex-1" />
        {focal
          ? <span className="uppercase text-accent font-medium" style={{ fontSize: 11, letterSpacing: '0.12em' }}>do first</span>
          : <span className="mono text-ink-4" style={{ fontSize: 11 }}>{memory.lastRelevant}</span>}
      </div>
      <div className="text-ink font-medium" style={{ fontSize: 13, lineHeight: 1.4 }}>
        {memory.what}
      </div>
      <div style={{ fontSize: 11, lineHeight: 1.4 }} className="mt-2 text-ink-4" >
        <span className="kanji text-accent" >{how.glyph}</span>{" "}
        Reinforces it in <span className="mono text-ink-3" >{how.target}</span>
      </div>

      {/* same action row as recommendations — one clear default everywhere */}
      <div className="gap-2 mt-2 flex items-center" >
        <button onClick={() => setStatus("applied")}
 style={{ fontSize: 12,
 borderRadius: 5, letterSpacing: 0.2 }} className="py-1 px-3 bg-ink text-paper" >
          Reinforce
        </button>
        <button style={{ fontSize: 12, border: 'var(--ink-line)',
 borderRadius: 5 }} className="py-1 px-3 text-ink-2" >
          Review
        </button>
        <span className="flex-1" />
        <button onClick={() => setStatus("dismissed")}
 style={{ fontSize: 11 }} className="py-1 px-1 text-ink-4" >
          Mute
        </button>
      </div>
    </div>
  );
}

// Every recommendation collapses to ONE decision, identical on every card:
//   Apply   — accept sensei's suggestion (the consistent default)
//   Review  — open / edit the target before committing
//   Dismiss — decline
// The old per-rec verb (promote · draft · scaffold · archive · reinforce ·
// clone) is no longer THE action — it becomes a typed descriptor chip plus a
// plain caption of exactly what Apply will do. The single highest-impact rec
// is marked "do first", so there's always one obvious next move.
const REC_KINDS = {
  "promote-pattern": { tag: "rule",    glyph: "則", effect: "Promotes the pattern to a rule in" },
  "create-agent":    { tag: "agent",   glyph: "作", effect: "Drafts the agent" },
  "write-skill":     { tag: "skill",   glyph: "技", effect: "Scaffolds" },
  "archive-memory":  { tag: "memory",  glyph: "納", effect: "Archives" },
  "cross-project":   { tag: "project", glyph: "渡", effect: "Clones it into" },
  "enrich-memory":   { tag: "memory",  glyph: "育", effect: "Reinforces" },
};

function RecCardSlim({ rec, focal = false }) {
  const k = REC_KINDS[rec.kind] || { tag: rec.targetKind, glyph: "•", effect: "Applies to" };
  const [status, setStatus] = l2S(null); // null | "applied" | "dismissed"

  // Resolved state — the card collapses to a quiet one-liner so handled
  // recommendations stop competing for attention.
  if (status) {
    const applied = status === "applied";
    return (
      <div style={{ gap: 8,
 background: applied ? 'var(--success-soft)' : 'var(--paper-2)', borderRadius: 5, opacity: applied ? 1 : 0.6
 }} className="py-2 px-3 flex items-center border border-paper-edge" >
        <span className="kanji" style={{ fontSize: 12,
                      color: applied ? 'var(--success)' : 'var(--ink-4)' }}>
          {applied ? "✓" : "—"}
        </span>
        <span className="flex-1 text-ink-2 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 11, lineHeight: 1.4 }}>
          {applied ? <>Applied · <span className="mono">{rec.targetName}</span></> : "Dismissed"}
        </span>
        <button className="text-ink-3" onClick={() => setStatus(null)}
 style={{ fontSize: 11 }}>Undo</button>
      </div>
    );
  }

  return (
    <div style={{
 borderLeft: focal ? '2px solid var(--accent)' : 'var(--hairline)',
 borderRadius: 5
 }} className="py-2 px-3 bg-paper-2 border border-paper-edge" >
      {/* typed descriptor + (focal) do-first marker — the kind, not the verb */}
      <div className="gap-2 mb-1 flex items-center" >
        <span className="inline-flex items-center text-ink-3" style={{ gap: 4,
 fontSize: 11 }}>
          <span className="kanji text-accent" style={{ fontSize: 12 }}>{k.glyph}</span>
          <span className="uppercase" style={{ letterSpacing: '0.08em' }}>{k.tag}</span>
        </span>
        <span className="flex-1" />
        {focal && (
          <span className="uppercase text-accent font-medium" style={{ fontSize: 11, letterSpacing: '0.12em' }}>do first</span>
        )}
      </div>

      <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.4 }}>{rec.title}</div>
      <div style={{ fontSize: 11, lineHeight: 1.5,
 display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }} className="mt-1 text-ink-3 overflow-hidden" >
        {rec.reasoning}
      </div>

      {/* exactly what Apply will do — the old verb, demoted to a caption */}
      <div style={{ fontSize: 11, lineHeight: 1.4 }} className="mt-2 text-ink-4" >
        {k.effect} <span className="mono text-ink-3" >{rec.targetName}</span>
      </div>

      {/* one consistent action row — identical on every card */}
      <div className="gap-2 mt-2 flex items-center" >
        <button onClick={() => setStatus("applied")}
 style={{ fontSize: 12,
 borderRadius: 5, letterSpacing: 0.2 }} className="py-1 px-3 bg-ink text-paper" >
          Apply
        </button>
        <button style={{ fontSize: 12, border: 'var(--ink-line)',
 borderRadius: 5 }} className="py-1 px-3 text-ink-2" >
          Review
        </button>
        <span className="flex-1" />
        <button onClick={() => setStatus("dismissed")}
 style={{ fontSize: 11 }} className="py-1 px-1 text-ink-4" >
          Dismiss
        </button>
      </div>
    </div>
  );
}

function CorrectionMini({ c }) {
  return (
    <div style={{ borderRadius: 5
 }} className="gap-2 py-2 px-3 flex items-baseline bg-paper-2 border border-paper-edge" >
      <span className="kanji text-warning" style={{ fontSize: 11 }}>直</span>
      <div className="flex-1 min-w-0" >
        <div className="text-ink overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 11, lineHeight: 1.4 }}>
          {c.text}
        </div>
      </div>
      <span className="mono text-ink-2" style={{ fontSize: 11 }}>{c.count}×</span>
    </div>
  );
}

function PatternMini({ pattern }) {
  return (
    <div style={{
 borderLeft: '2px solid var(--warning)', borderRadius: 5
 }} className="py-2 px-3 bg-paper-2 border border-paper-edge" >
      <div className="gap-2 mb-1 flex items-center" >
        <span className="mono text-ink" style={{ fontSize: 13 }}>{pattern.name}</span>
        <span className="flex-1" />
        <span className="mono text-success" style={{ fontSize: 11 }}>
          FTR +{Math.round(pattern.ftrDelta*100)}%
        </span>
      </div>
      <div className="text-ink-3" style={{ fontSize: 11, lineHeight: 1.45 }}>
        {pattern.occurrences} places · candidate to promote
      </div>
    </div>
  );
}

function ChallengedMini({ memory }) {
  return (
    <div style={{ border: '1px solid transparent',
 borderRadius: 5
 }} className="py-2 px-3 bg-warning-soft" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-warning uppercase" >challenged</div>
      <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.4 }}>{memory.what}</div>
    </div>
  );
}

function SettledRow({ memory }) {
  return (
    <div className="gap-2 py-1 px-0 flex items-baseline" >
      <StrengthBar value={memory.strength} compact/>
      <div className="flex-1 min-w-0" >
        <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.4 }}>{memory.what}</div>
        <L2WhereLine scope={memory.scope}/>
      </div>
    </div>
  );
}

function StrengthBar({ value, compact }) {
  return (
    <div title={`strength ${value}/5`}
 style={{
 width: compact ? 24 : 60, height: 3, borderRadius: 2,
 background: 'var(--edge)' }} className="mt-1 relative shrink-0" >
      <div className="absolute" style={{ inset: 0,
 width: `${(value/5)*100}%`,
 background: value === 5 ? 'var(--success)' : 'var(--ink-2)',
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Learnings · Anatomy"
 >
      <L2Hero kanji="覚" title="Every memory has the same anatomy."
              sub="What it is · why it matters · how it's surfaced · where it applies."
              right={<HealthChart memories={active}/>}/>

      <div className="flex-1 grid min-h-0" style={{
 gridTemplateColumns: '260px 1fr' }}>
        {/* List rail */}
        <aside className="py-3 px-0 border-r overflow-auto" >
          {groups.map(g => {
            const items = active.filter(m => m.category === g.id ||
              (g.id === "pattern" && m.category === "anti_pattern"));
            if (items.length === 0) return null;
            return (
              <div key={g.id} className="mb-3" >
                <div className="gap-2 py-1 px-4 flex items-baseline" >
                  <span className="kanji text-accent" style={{ fontSize: 13 }}>{g.glyph}</span>
                  <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{g.label}</span>
                  <span className="mono text-ink-4" style={{ fontSize: 11 }}>{items.length}</span>
                </div>
                {items.map(m => (
                  <button key={m.id} onClick={() => setOpen(m.id)}
 style={{
 background: openId === m.id ? 'var(--paper-2)' : 'transparent',
 borderLeft: openId === m.id ? '2px solid var(--accent)' : '2px solid transparent' }} className="py-2 pl-6 pr-4 w-full text-left cursor-pointer" >
                    <div className="text-ink overflow-hidden" style={{ fontSize: 13, lineHeight: 1.4,
 display: '-webkit-box', WebkitLineClamp: 2,
 WebkitBoxOrient: 'vertical' }}>
                      {m.what}
                    </div>
                    <div className="gap-1 mt-1 flex items-center" >
                      <StrengthBar value={m.strength} compact/>
                      {m.violated > 0 && (
                        <span className="mono text-accent" style={{ fontSize: 11 }}>
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
        <main className="pt-6 pb-8 px-8 overflow-auto" >
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
    <div className="gap-4 flex flex-col" >
      {/* Title + meta */}
      <div>
        <div className="gap-2 mb-1 flex items-center" >
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{memory.category.replace("_", "-")}</span>
          <span className="text-ink-4" style={{ fontSize: 11 }}>·</span>
          <span className="text-ink-3" style={{ fontSize: 11 }}>{memory.state}</span>
          <span className="flex-1" />
          <StrengthBar value={memory.strength}/>
          <span className="mono text-ink-2" style={{ fontSize: 11 }}>
            {memory.strength}/5
          </span>
        </div>
        <h2 className="display m-0 font-normal text-ink" style={{
 fontSize: 22, lineHeight: 1.3
 }}>
          {memory.what}
        </h2>
      </div>

      {/* 2×2 anatomy grid */}
      <div style={{ gridTemplateColumns: '1fr 1fr',
 gridTemplateRows: 'auto auto'
 }} className="gap-3 grid" >
        <AnatomyBlock label="What" kanji="何" tone="ink">
          <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.5 }}>
            {memory.what}
          </div>
        </AnatomyBlock>

        <AnatomyBlock label="Why · because" kanji="故" tone="ink">
          <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.55 }}>
            {memory.because}
          </div>
        </AnatomyBlock>

        <AnatomyBlock label="How · surface" kanji={how.glyph} tone="shu">
          <div style={{
 fontSize: 13, lineHeight: 1.5
 }} className="mb-1 text-ink-2" >
            sensei surfaces this as a{" "}
            <span className="text-ink font-medium" >{how.label}</span>
          </div>
          <div className="mono py-1 px-2 bg-paper border border-paper-edge text-ink" style={{
 fontSize: 11, borderRadius: 4 }}>
            {how.target}
          </div>
          {memory.references.good_example && (
            <div className="mono mt-2 text-success" style={{ fontSize: 11 }}>
              ✓ {memory.references.good_example}
            </div>
          )}
          {memory.references.bad_example && (
            <div className="mono text-accent" style={{ fontSize: 11 }}>
              ✗ {memory.references.bad_example}
            </div>
          )}
        </AnatomyBlock>

        <AnatomyBlock label="Where · scope" kanji="域" tone="jade">
          <div className="gap-1 mb-2 flex flex-wrap" >
            {scopeChips(memory.scope, L).map((c, i) => (
              <span key={i} className={(c.mono ? "mono" : "") + ' py-1 px-2'}
                    style={{
 fontSize: 11,
                              background: 'var(--paper)', border: 'var(--hairline)',
                              borderRadius: 12, color: c.dim ? 'var(--ink-3)' : 'var(--ink-2)',
                              textTransform: c.upper ? 'uppercase' : 'none',
                              letterSpacing: c.upper ? '0.12em' : 0
}}>
                {c.label}: {c.value}
              </span>
            ))}
          </div>
          <div className="text-ink-3" style={{ fontSize: 11 }}>
            Learned {memory.learned} · last relevant {memory.lastRelevant} · reinforced {memory.reinforced}× · violated {memory.violated}×
          </div>
        </AnatomyBlock>
      </div>

      {/* Actions */}
      <div className="gap-2 pt-1 flex flex-wrap" >
        <FlatBtn glyph="昇" label="Promote to rule"/>
        <FlatBtn glyph="育" label="Enrich"/>
        <FlatBtn glyph="渡" label="Apply elsewhere"/>
        <FlatBtn glyph="疑" label="Challenge"/>
        <span className="flex-1" />
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
  const accent = tone === "shu" ? "var(--accent)" :
                 tone === "jade" ? "var(--success)" : "var(--ink-2)";
  return (
    <section style={{
 borderRadius: 8 }} className="py-3 px-4 bg-paper-2 border border-paper-edge flex flex-col" >
      <div className="gap-2 mb-2 flex items-baseline" >
        <span className="kanji" style={{ fontSize: 15, color: accent, lineHeight: 1 }}>{kanji}</span>
        <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>{label}</span>
      </div>
      {children}
    </section>
  );
}

function FlatBtn({ glyph, label, subtle }) {
  return (
    <button style={{
 fontSize: 11,
 borderRadius: 5, color: subtle ? 'var(--ink-3)' : 'var(--ink)' }} className="py-2 px-3 gap-2 bg-paper-2 border border-paper-edge cursor-pointer flex items-center" >
      <span className="kanji" style={{ fontSize: 13,
                    color: subtle ? 'var(--ink-3)' : 'var(--accent)' }}>{glyph}</span>
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
    <div className="pl-6 border-l" >
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-4 uppercase" >
        memory strength
      </div>
      <svg className="block" width={W} height={H + 14} >
        {buckets.map((n, i) => {
          const h = (n / max) * H;
          return (
            <g key={i}>
              <rect x={i*(bw+gap)} y={H - h} width={bw} height={h}
                    fill={i === 4 ? 'var(--success)' : i >= 2 ? 'var(--ink-2)' : 'var(--ink-3)'}
                    rx={1}/>
              <text x={i*(bw+gap) + bw/2} y={H + 11}
                    fontSize={9} fill="var(--ink-4)" textAnchor="middle"
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Learnings · Brief"
 >
      <L2Hero kanji="学" title="A daily brief of what sensei knows."
              sub="Two questions only — what should I act on, and what's been learned."/>

      <div className="gap-6 pt-4 pb-8 px-8 flex-1 overflow-auto min-h-0 flex flex-col" >

        {/* Memory shape chart */}
        <MemoryShapeChart memories={L.memories.filter(m => m.state !== "archived" && inProj(m))}/>

        {/* shared project filter */}
        <ProjectFilter value={project} onChange={setProject} projects={L.projects}/>

        {/* Things to act on */}
        <section>
          <BriefHeader kanji="令" title="Things to act on"
                       sub="recommendations and recurring corrections, by impact"
                       count={acts.length}/>
          <div className="gap-1 flex flex-col" >
            {acts.map(a => <ActRow key={a.id} act={a}/>)}
          </div>
        </section>

        {/* What I've learned — collapsible per category */}
        <section>
          <BriefHeader kanji="覚" title="What I've learned"
                       sub="memories grouped by what they govern"
                       count={L.memories.filter(m => m.state !== "archived" && inProj(m)).length}/>
          <div className="gap-1 flex flex-col" >
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
    <div className="gap-3 mb-3 pb-2 flex items-baseline border-b" >
      <span className="kanji text-accent" style={{ fontSize: 15 }}>{kanji}</span>
      <h3 className="display m-0 font-normal text-ink" style={{
 fontSize: 15 }}>{title}</h3>
      <span className="text-ink-3" style={{ fontSize: 11 }}>· {sub}</span>
      <span className="flex-1" />
      <span className="mono text-ink-3" style={{ fontSize: 11 }}>{count}</span>
    </div>
  );
}

function ActRow({ act }) {
  const dot = act.impact === "high" ? 'var(--accent)' :
              act.impact === "medium" ? 'var(--warning)' : 'var(--ink-4)';
  return (
    <article style={{
 gridTemplateColumns: '14px 1fr auto', borderRadius: 5
 }} className="gap-3 py-3 px-3 grid items-start bg-paper-2 border border-paper-edge" >
      <span style={{
 width: 7, height: 7, background: dot
 }} className="mt-1 rounded-full" />
      <div className="min-w-0" >
        <div className="text-ink font-medium" style={{ fontSize: 13, lineHeight: 1.4 }}>{act.title}</div>
        <div style={{
 fontSize: 11, lineHeight: 1.55
 }} className="mt-1 text-ink-2" >{act.why}</div>
        <div className="mono mt-1 text-ink-4" style={{
 fontSize: 11 }}>{act.target}</div>
      </div>
      <button style={{
 fontSize: 11,
 background: act.impact === "high" ? 'var(--ink)' : 'transparent',
 color: act.impact === "high" ? 'var(--paper)' : 'var(--ink-2)',
 border: act.impact === "high" ? 'none' : '1px solid var(--edge)',
 borderRadius: 4 }} className="py-1 px-3 cursor-pointer whitespace-nowrap" >
        {act.cta} →
      </button>
    </article>
  );
}

function CategorySection({ cat, items, open, onToggle }) {
  return (
    <div className="border border-paper-edge bg-paper-2 overflow-hidden" style={{ borderRadius: 6 }}>
      <button onClick={onToggle}
 className="py-3 px-4 gap-3 w-full flex items-center bg-transparent border-0 cursor-pointer text-left" >
        <span className="kanji text-accent" style={{ fontSize: 13 }}>{cat.glyph}</span>
        <span className="text-ink font-medium" style={{ fontSize: 13 }}>{cat.label}</span>
        <span className="text-ink-3" style={{ fontSize: 11 }}>· {cat.desc}</span>
        <span className="flex-1" />
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{items.length}</span>
        <span className="text-ink-3" style={{ fontSize: 11,
 transform: open ? 'rotate(90deg)' : 'rotate(0deg)',
 transition: 'transform 0.15s' }}>›</span>
      </button>
      {open && items.length > 0 && (
        <div className="pt-1 pb-3 px-4 border-t" >
          {items.map(m => <BriefMemoryLine key={m.id} memory={m}/>)}
        </div>
      )}
    </div>
  );
}

function BriefMemoryLine({ memory }) {
  const how = inferHow(memory);
  return (
    <div style={{
 gridTemplateColumns: '52px 1fr 130px',
 borderBottom: '1px dashed var(--edge)'
 }} className="gap-3 py-2 px-0 grid items-start" >
      <StrengthBar value={memory.strength}/>
      <div>
        <div className="text-ink font-medium" style={{ fontSize: 13, lineHeight: 1.45 }}>{memory.what}</div>
        <div style={{
 fontSize: 11, lineHeight: 1.55 }} className="mt-1 text-ink-2 italic" >
          because <span className="not-italic" >{memory.because}</span>
        </div>
        <div style={{
 fontSize: 11 }} className="gap-3 mt-1 flex text-ink-3" >
          <span>
            <span className="kanji mr-1 text-accent" >{how.glyph}</span>
            {how.label}
          </span>
          <L2WhereLine scope={memory.scope}/>
        </div>
      </div>
      <div className="text-right" >
        {memory.violated > 0 ? (
          <span className="mono text-accent" style={{ fontSize: 11 }}>
            {memory.violated}× violated
          </span>
        ) : (
          <span className="text-ink-4 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
            {memory.state}
          </span>
        )}
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
          {memory.lastRelevant}
        </div>
      </div>
    </div>
  );
}

// Stacked-bar chart: each category shows count of memories at strength 1..5
function MemoryShapeChart({ memories }) {
  const cats = [
    { id: "correctness", label: "correctness", color: 'var(--accent)' },
    { id: "convention",  label: "conventions", color: 'var(--ink-2)' },
    { id: "preference",  label: "preferences", color: 'var(--success)' },
    { id: "pattern",     label: "patterns",    color: 'var(--warning)' }
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
    <section style={{
 borderRadius: 8
 }} className="py-4 px-4 bg-paper-2 border border-paper-edge" >
      <div className="gap-2 mb-3 flex items-baseline" >
        <span className="kanji text-accent" style={{ fontSize: 13 }}>形</span>
        <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>memory shape</span>
        <span className="text-ink-3" style={{ fontSize: 11 }}>
          · {memories.length} memories · proportions show category mix · each row tracks average strength
        </span>
      </div>

      {/* Composition bar */}
      <svg className="block max-w-full" width={W} height={H} 
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
                      fill="var(--accent)"/>
            )}
            <text x={b.x + 6} y={H - 6} fontSize={10}
                  fill="var(--ink-3)" fontFamily="Inter">
              {b.label} · {b.items.length}
            </text>
          </g>
        ))}
      </svg>

      <div style={{
 fontSize: 11 }} className="mt-2 flex justify-between text-ink-4" >
        <span>area = share of memories · fill height = avg strength · ● = active violations</span>
        <span className="mono">avg {(memories.reduce((s,m)=>s+m.strength,0)/Math.max(memories.length,1)).toFixed(1)}/5</span>
      </div>
    </section>
  );
}

Object.assign(window, { LearningsTriage, LearningsAnatomy, LearningsBrief, FlatBtn });
