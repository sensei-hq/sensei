// site/dojo-for-teams.jsx — the Dōjō · for teams governance section.
// Extracted from the (discarded) variant-a so the live site (variant-b)
// can mount it without loading the whole discarded page.
const { useState: dtS, useEffect: dtE } = React;

// ─── Dōjō · for teams (governance) ──────────────────────────────
function useTeamsBP() {
  const [w, setW] = dtS(() => (typeof window !== 'undefined' ? window.innerWidth : 1200));
  dtE(() => {
    const on = () => setW(window.innerWidth);
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  }, []);
  return { md: w <= 900, sm: w <= 620 };
}
function DojoForTeams() {
  const bp = useTeamsBP();
  const eyebrow = { fontSize: 11, letterSpacing: '0.22em', color: 'var(--ink-3)', textTransform: 'uppercase' };
  const sub = { fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)', textTransform: 'uppercase', fontWeight: 600 };
  const loop = [
    ["01","Contribute","Individual","A dev marks a memory, pattern or guard to share, scoped to where it's true."],
    ["02","Accumulate","The hive","Contributions pool on the company server, clustered and deduped against existing knowledge."],
    ["03","Triage","Maintainer","Candidates are scored, conflicts surfaced, near-duplicates merged — the trust step."],
    ["04","Approve","Maintainer","A named approval publishes it at its scope, with attribution and a regression note."],
    ["05","Distribute","Everyone","Approved practice lands automatically in every matching scope's Today / Upgrades."],
  ];
  const artifacts = [
    ["理","Guiding principles","Durable engineering values and ‘how we build here’ statements."],
    ["紋","Patterns","Constructive shapes promoted to rules — and the anti-patterns to avoid."],
    ["問","Prompts","Vetted prompt templates and personas for recurring tasks."],
    ["守","Guards","Lints, checks and safety rails that gate risky changes."],
    ["技","Skills","Packaged capabilities an assistant can pick up for a task."],
    ["使","Agents","Configured agents with tools and scope, shared ready-to-run."],
  ];
  const members = [["社","Employer"],["客","Clients"],["群","Communities"],["己","Personal"]];
  const ladders = [
    { k:"守", name:"Security", rungs:[["P0","Never log secrets"],["P1","Auth needs a test persona"],["P2","Parameterize queries"]] },
    { k:"構", name:"Architecture", rungs:[["P0","APIs stay compatible"],["P1","Depend inward"],["P2","Co-locate state"]] },
    { k:"験", name:"Testing", rungs:[["P0","Hold the coverage floor"],["P1","Bug fix ships a test"],["P2","Table-driven tests"]] },
  ];
  const ptone = { P0:'var(--accent)', P1:'var(--warning)', P2:'var(--ink-mute)' };
  const vs = {
    left:  { k:"群", h:"Collective", sub:"global · public commons", rows:[
      ["Hosting","Sensei community cloud"],
      ["Audience","All users · anonymized"],
      ["Membership","One public commons"],
      ["Scope","Stack-agnostic, generalised"],
      ["Trust","Reputation + aggregate signal"],
      ["Direction","Opt-in share · pull upgrades"],
      ["Control","Per-category filters, cadence"],
    ]},
    right: { k:"結", h:"Dōjō", sub:"private · governed · company-hosted", rows:[
      ["Hosting","Your infrastructure / VPC"],
      ["Audience","Your orgs · attributed"],
      ["Membership","Many — employer · clients · communities"],
      ["Scope","Company → team → project → repo → stack"],
      ["Trust","Triage + named approval"],
      ["Direction","Governed upstream + downstream loop"],
      ["Control","Roles, policies, approval gates"],
    ]},
  };
  return (
    <section id="teams" style={{ borderTop: 'var(--hairline)', background: 'var(--paper-2)' }} className="py-9 px-7" >
      <div style={{ maxWidth: 1200 }} className="mx-auto" >
        <div style={eyebrow} className="mb-3" >For teams · 結 Dōjō</div>
        <h2 className="display mt-0 mb-4" style={{ fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em' }}>
          Your team's hard-won lessons — shared, governed, routed.
        </h2>
        <p style={{ fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.65, maxWidth: 620 }} className="mb-8" >
          Sensei is local-first for one developer. <span style={{ color: 'var(--ink)' }}>Dōjō</span> is its company-hosted counterpart — a private, governed collective that turns what individuals learn into team practice, with nothing leaking.
        </p>

        {/* the loop — lifecycle cards + distributed-back return */}
        <div style={sub} className="mb-3" >The loop</div>
        <div style={{ display: 'flex', flexDirection: bp.md ? 'column' : 'row', alignItems: 'stretch', gap: bp.md ? 8 : 0 }}>
          {loop.map(([n, t, who, d], i) => (
            <React.Fragment key={n}>
              {i > 0 && <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', padding: bp.md ? '2px 0' : '0 6px', color: 'var(--accent)', fontFamily: 'var(--font-mono)', fontSize: 13, transform: bp.md ? 'rotate(90deg)' : 'none' }}>→</div>}
              <div style={{ flex: 1, background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 12, padding: '14px 14px', display: 'flex', flexDirection: 'column', gap: 5 }}>
                <span className="mono" style={{ fontSize: 10, color: 'var(--accent)', letterSpacing: '0.06em' }}>{n}</span>
                <span className="display" style={{ fontSize: 16, fontWeight: 500, letterSpacing: '-0.01em', color: 'var(--ink)' }}>{t}</span>
                <span style={{ fontSize: 9.5, letterSpacing: '0.1em', textTransform: 'uppercase', color: 'var(--ink-3)', fontWeight: 600 }}>{who}</span>
                <span style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.5 }}>{d}</span>
              </div>
            </React.Fragment>
          ))}
        </div>
        <div style={{ position: 'relative', height: 36, marginTop: 4 }} className="mb-8" >
          <svg viewBox="0 0 1000 36" preserveAspectRatio="none" style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', overflow: 'visible' }}>
            <path d="M 962 2 C 962 28, 942 32, 900 32 L 100 32 C 58 32, 38 28, 38 4" fill="none" stroke="var(--accent)" strokeWidth="1.4" strokeDasharray="5 4"/>
            <path d="M 38 1 L 33 11 L 43 11 Z" fill="var(--accent)"/>
          </svg>
          <div style={{ position: 'absolute', left: '50%', top: 9, transform: 'translateX(-50%)', background: 'var(--paper-2)', padding: '0 12px', fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--accent)', letterSpacing: '0.04em' }}>↑ distributed back · to every matching scope</div>
        </div>

        {/* what flows — artifact cards with descriptions */}
        <div style={sub} className="mb-3" >What flows through it</div>
        <div style={{ display: 'grid', gridTemplateColumns: bp.sm ? '1fr' : bp.md ? 'repeat(2, 1fr)' : 'repeat(3, 1fr)' }} className="gap-3 mb-8" >
          {artifacts.map(([k, n, d]) => (
            <div key={n} style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 10, padding: '15px 16px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)', lineHeight: 1 }} >{k}</span>
                <span className="display" style={{ fontSize: 14.5, fontWeight: 500, letterSpacing: '-0.01em', color: 'var(--ink)' }}>{n}</span>
              </div>
              <div style={{ fontSize: 11.5, color: 'var(--ink-3)', lineHeight: 1.5 }} className="mt-2" >{d}</div>
            </div>
          ))}
        </div>

        {/* membership & routing */}
        <div style={{ display: 'grid', gridTemplateColumns: bp.md ? '1fr' : '1fr 1fr', alignItems: 'start' }} className="gap-8 mb-8" >
          <div>
            <div style={sub} className="mb-3" >One developer, many orgs</div>
            <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6 }} className="mb-3" >
              A dev can belong to several at once. Every project is bound to exactly one — so findings route only where they belong and never pollute an unrelated hive-mind.
            </p>
            <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-2" >
              {members.map(([k, n]) => (
                <span key={n} style={{ display: 'inline-flex', alignItems: 'center', gap: 6, fontSize: 12, color: 'var(--ink-2)',
                              background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 20, padding: '5px 12px' }}>
                  <span className="kanji" style={{ color: 'var(--accent)', fontSize: 13 }}>{k}</span>{n}
                </span>
              ))}
            </div>
          </div>
          <div>
            <div style={sub} className="mb-3" >Ranked by priority, not hierarchy</div>
            <div style={{ display: 'grid', gridTemplateColumns: bp.sm ? '1fr' : 'repeat(3, 1fr)' }} className="gap-2" >
              {ladders.map(l => (
                <div key={l.name} style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 10, padding: '11px 11px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }} className="mb-2" >
                    <span className="kanji" style={{ fontSize: 14, color: 'var(--accent)' }}>{l.k}</span>
                    <span style={{ fontSize: 12, color: 'var(--ink)', fontWeight: 500 }}>{l.name}</span>
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
                    {l.rungs.map(([p, t]) => (
                      <div key={p} style={{ display: 'flex', gap: 6, alignItems: 'baseline' }}>
                        <span className="mono" style={{ fontSize: 9, fontWeight: 600, color: ptone[p] }}>{p}</span>
                        <span style={{ fontSize: 10.5, color: 'var(--ink-2)', lineHeight: 1.35 }}>{t}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', fontStyle: 'italic' }} className="mt-2" >When principles compete, the higher rung wins.</div>
          </div>
        </div>

        {/* governance-as-onboarding + confidentiality */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))' }} className="gap-3 mb-8">
          <div style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 14 }} className="p-5">
            <span className="kanji" style={{ fontSize: 26, color: 'var(--accent)' }}>迎</span>
            <div style={{ fontSize: 16, fontWeight: 600, color: 'var(--ink)' }} className="mt-3 mb-1">Onboarding is inheritance</div>
            <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.65 }} className="m-0">
              The Dōjō holds your governance model — rules, skills, agents, commands and each project's memory,
              authored once per scope. A developer who joins a project inherits the composed set the moment they
              connect. Day one feels like month three.
            </p>
          </div>
          <div style={{ background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 14 }} className="p-5">
            <span className="kanji" style={{ fontSize: 26, color: 'var(--accent)' }}>盾</span>
            <div style={{ fontSize: 16, fontWeight: 600, color: 'var(--ink)' }} className="mt-3 mb-1">Client work never leaks</div>
            <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.65 }} className="m-0">
              Lessons from client engagements are anonymized before anything leaves — the rule and its why travel;
              client, repo and identifiers are dropped. If a lesson can't stand without them, it doesn't leave.
              Every anonymization lands in an immutable audit trail.
            </p>
          </div>
        </div>

        {/* collective vs dojo */}
        <div style={sub} className="mb-3" >The global Collective vs. your Dōjō</div>
        <div style={{ display: 'grid', gridTemplateColumns: bp.md ? '1fr' : '1fr 1fr', border: 'var(--hairline)', borderRadius: 14, overflow: 'hidden' }}>
          {[vs.left, vs.right].map((col, ci) => (
            <div key={ci} style={{ borderLeft: ci === 1 && !bp.md ? 'var(--hairline)' : 'none', borderTop: ci === 1 && bp.md ? 'var(--hairline)' : 'none',
                          background: ci === 1 ? 'color-mix(in srgb, var(--accent) 5%, var(--paper))' : 'var(--paper)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 11, padding: '16px 20px 13px', borderBottom: 'var(--hairline)' }}>
                <span className="kanji" style={{ fontSize: 24, color: ci === 1 ? 'var(--accent)' : 'var(--ink-3)', lineHeight: 1 }}>{col.k}</span>
                <div>
                  <div className="display" style={{ fontSize: 17, fontWeight: 500, letterSpacing: '-0.01em', color: 'var(--ink)' }}>{col.h}</div>
                  <div className="mono" style={{ fontSize: 10.5, color: 'var(--ink-3)' }}>{col.sub}</div>
                </div>
              </div>
              {col.rows.map(([l, v], i) => (
                <div key={l} style={{ padding: '11px 20px', borderBottom: i < col.rows.length - 1 ? '1px solid var(--edge)' : 'none' }}>
                  <div style={{ fontSize: 9.5, letterSpacing: '0.1em', textTransform: 'uppercase', color: 'var(--ink-4)', fontWeight: 600, marginBottom: 3 }}>{l}</div>
                  <div style={{ fontSize: 12.5, color: ci === 1 ? 'var(--ink)' : 'var(--ink-2)' }}>{v}</div>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

Object.assign(window, { DojoForTeams });
