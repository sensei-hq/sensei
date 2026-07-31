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
    <section id="teams" className="py-24 px-12 border-t bg-paper-2" >
      <div style={{ maxWidth: 1200 }} className="mx-auto" >
        <div style={eyebrow} className="mb-3" >For teams · 結 Dōjō</div>
        <h2 className="display mt-0 mb-4 font-light" style={{ fontSize: 40, letterSpacing: '-0.02em' }}>
          Your team's hard-won lessons — shared, governed, routed.
        </h2>
        <p style={{ fontSize: 15, lineHeight: 1.65, maxWidth: 620 }} className="mb-16 text-ink-2" >
          Sensei is local-first for one developer. <span className="text-ink" >Dōjō</span> is its company-hosted counterpart — a private, governed collective that turns what individuals learn into team practice, with nothing leaking.
        </p>

        {/* the loop — lifecycle cards + distributed-back return */}
        <div style={sub} className="mb-3" >The loop</div>
        <div className="flex items-stretch" style={{ flexDirection: bp.md ? 'column' : 'row', gap: bp.md ? 8 : 0 }}>
          {loop.map(([n, t, who, d], i) => (
            <React.Fragment key={n}>
              {i > 0 && <div className="flex items-center justify-center text-accent" style={{ padding: bp.md ? '2px 0' : '0 6px', fontFamily: 'var(--font-mono)', fontSize: 13, transform: bp.md ? 'rotate(90deg)' : 'none' }}>→</div>}
              <div className="flex-1 bg-paper border border-paper-edge flex flex-col" style={{ borderRadius: 12, padding: '14px 14px', gap: 5 }}>
                <span className="mono text-accent" style={{ fontSize: 10, letterSpacing: '0.06em' }}>{n}</span>
                <span className="display font-medium text-ink" style={{ fontSize: 16, letterSpacing: '-0.01em' }}>{t}</span>
                <span className="uppercase text-ink-3 font-semibold" style={{ fontSize: 9.5, letterSpacing: '0.1em' }}>{who}</span>
                <span className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.5 }}>{d}</span>
              </div>
            </React.Fragment>
          ))}
        </div>
        <div style={{ height: 36, marginTop: 4 }} className="mb-16 relative" >
          <svg className="absolute w-full h-full overflow-visible" viewBox="0 0 1000 36" preserveAspectRatio="none" style={{ inset: 0 }}>
            <path d="M 962 2 C 962 28, 942 32, 900 32 L 100 32 C 58 32, 38 28, 38 4" fill="none" stroke="var(--accent)" strokeWidth="1.4" strokeDasharray="5 4"/>
            <path d="M 38 1 L 33 11 L 43 11 Z" fill="var(--accent)"/>
          </svg>
          <div className="absolute bg-paper-2 text-accent" style={{ left: '50%', top: 9, transform: 'translateX(-50%)', padding: '0 12px', fontFamily: 'var(--font-mono)', fontSize: 11, letterSpacing: '0.04em' }}>↑ distributed back · to every matching scope</div>
        </div>

        {/* what flows — artifact cards with descriptions */}
        <div style={sub} className="mb-3" >What flows through it</div>
        <div style={{ gridTemplateColumns: bp.sm ? '1fr' : bp.md ? 'repeat(2, 1fr)' : 'repeat(3, 1fr)' }} className="gap-3 mb-16 grid" >
          {artifacts.map(([k, n, d]) => (
            <div className="bg-paper border border-paper-edge" key={n} style={{ borderRadius: 10, padding: '15px 16px' }}>
              <div className="flex items-center" style={{ gap: 8 }}>
                <span className="kanji text-accent" style={{ fontSize: 15, lineHeight: 1 }} >{k}</span>
                <span className="display font-medium text-ink" style={{ fontSize: 14.5, letterSpacing: '-0.01em' }}>{n}</span>
              </div>
              <div style={{ fontSize: 11.5, lineHeight: 1.5 }} className="mt-2 text-ink-3" >{d}</div>
            </div>
          ))}
        </div>

        {/* membership & routing */}
        <div style={{ gridTemplateColumns: bp.md ? '1fr' : '1fr 1fr' }} className="gap-16 mb-16 grid items-start" >
          <div>
            <div style={sub} className="mb-3" >One developer, many orgs</div>
            <p style={{ fontSize: 13, lineHeight: 1.6 }} className="mb-3 text-ink-2" >
              A dev can belong to several at once. Every project is bound to exactly one — so findings route only where they belong and never pollute an unrelated hive-mind.
            </p>
            <div className="gap-2 flex flex-wrap" >
              {members.map(([k, n]) => (
                <span className="inline-flex items-center text-ink-2 bg-paper border border-paper-edge" key={n} style={{ gap: 6, fontSize: 12, borderRadius: 20, padding: '5px 12px' }}>
                  <span className="kanji text-accent" style={{ fontSize: 13 }}>{k}</span>{n}
                </span>
              ))}
            </div>
          </div>
          <div>
            <div style={sub} className="mb-3" >Ranked by priority, not hierarchy</div>
            <div style={{ gridTemplateColumns: bp.sm ? '1fr' : 'repeat(3, 1fr)' }} className="gap-2 grid" >
              {ladders.map(l => (
                <div className="bg-paper border border-paper-edge" key={l.name} style={{ borderRadius: 10, padding: '11px 11px' }}>
                  <div style={{ gap: 6 }} className="mb-2 flex items-center" >
                    <span className="kanji text-accent" style={{ fontSize: 14 }}>{l.k}</span>
                    <span className="text-ink font-medium" style={{ fontSize: 12 }}>{l.name}</span>
                  </div>
                  <div className="gap-1 flex flex-col" >
                    {l.rungs.map(([p, t]) => (
                      <div className="flex items-baseline" key={p} style={{ gap: 6 }}>
                        <span className="mono font-semibold" style={{ fontSize: 9, color: ptone[p] }}>{p}</span>
                        <span className="text-ink-2" style={{ fontSize: 10.5, lineHeight: 1.35 }}>{t}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
            <div style={{ fontSize: 11 }} className="mt-2 text-ink-3 italic" >When principles compete, the higher rung wins.</div>
          </div>
        </div>

        {/* governance-as-onboarding + confidentiality */}
        <div style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))' }} className="gap-3 mb-16 grid">
          <div style={{ borderRadius: 14 }} className="p-6 bg-paper border border-paper-edge">
            <span className="kanji text-accent" style={{ fontSize: 26 }}>迎</span>
            <div style={{ fontSize: 16 }} className="mt-3 mb-1 font-semibold text-ink">Onboarding is inheritance</div>
            <p style={{ fontSize: 13, lineHeight: 1.65 }} className="m-0 text-ink-2">
              The Dōjō holds your governance model — rules, skills, agents, commands and each project's memory,
              authored once per scope. A developer who joins a project inherits the composed set the moment they
              connect. Day one feels like month three.
            </p>
          </div>
          <div style={{ borderRadius: 14 }} className="p-6 bg-paper border border-paper-edge">
            <span className="kanji text-accent" style={{ fontSize: 26 }}>盾</span>
            <div style={{ fontSize: 16 }} className="mt-3 mb-1 font-semibold text-ink">Client work never leaks</div>
            <p style={{ fontSize: 13, lineHeight: 1.65 }} className="m-0 text-ink-2">
              Lessons from client engagements are anonymized before anything leaves — the rule and its why travel;
              client, repo and identifiers are dropped. If a lesson can't stand without them, it doesn't leave.
              Every anonymization lands in an immutable audit trail.
            </p>
          </div>
        </div>

        {/* collective vs dojo */}
        <div style={sub} className="mb-3" >The global Collective vs. your Dōjō</div>
        <div className="grid border border-paper-edge overflow-hidden" style={{ gridTemplateColumns: bp.md ? '1fr' : '1fr 1fr', borderRadius: 14 }}>
          {[vs.left, vs.right].map((col, ci) => (
            <div key={ci} style={{ borderLeft: ci === 1 && !bp.md ? 'var(--hairline)' : 'none', borderTop: ci === 1 && bp.md ? 'var(--hairline)' : 'none',
                          background: ci === 1 ? 'color-mix(in srgb, var(--accent) 5%, var(--paper))' : 'var(--paper)' }}>
              <div className="flex items-center border-b" style={{ gap: 11, padding: '16px 20px 13px' }}>
                <span className="kanji" style={{ fontSize: 24, color: ci === 1 ? 'var(--accent)' : 'var(--ink-3)', lineHeight: 1 }}>{col.k}</span>
                <div>
                  <div className="display font-medium text-ink" style={{ fontSize: 17, letterSpacing: '-0.01em' }}>{col.h}</div>
                  <div className="mono text-ink-3" style={{ fontSize: 10.5 }}>{col.sub}</div>
                </div>
              </div>
              {col.rows.map(([l, v], i) => (
                <div key={l} style={{ padding: '11px 20px', borderBottom: i < col.rows.length - 1 ? '1px solid var(--edge)' : 'none' }}>
                  <div className="uppercase text-ink-4 font-semibold" style={{ fontSize: 9.5, letterSpacing: '0.1em', marginBottom: 3 }}>{l}</div>
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
