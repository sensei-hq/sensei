// Section — generic eyebrow + title (left) and body (right) layout.

function Section({ id, eyebrow, title, children, background, narrow }) {
  return (
    <section id={id} style={{
      borderTop: 'var(--hairline)',
      background: background || 'var(--paper)',
      padding: 'var(--space-8) var(--space-7)'
    }}>
      <div style={{
        maxWidth: narrow ? 760 : 1100, margin: '0 auto',
        display: narrow ? 'block' : 'grid',
        gridTemplateColumns: narrow ? undefined : '1fr 1.4fr',
        gap: 'var(--space-7)', alignItems: 'start',
        textAlign: narrow ? 'center' : 'left'
      }}>
        <div>
          {eyebrow && <div className="zs-eyebrow mb-3">{eyebrow}</div>}
          <h2 style={{
            fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)',
            fontWeight: 400, letterSpacing: '-0.015em', lineHeight: 1.25,
            margin: 0, maxWidth: narrow ? '100%' : undefined
          }}>{title}</h2>
        </div>
        <div className="zs-body">{children}</div>
      </div>
    </section>
  );
}

// HowItWorks — 3-column Watch · Notice · Adopt block
function HowItWorks() {
  const steps = [
    { kanji: "観", phase: "Watch",
      text: "Sensei sits beside your editor and AI tools, capturing the shape of each session — the prompts, the responses, the corrections.",
      sub: "Local only. Nothing leaves your machine." },
    { kanji: "察", phase: "Notice",
      text: "After a few days, patterns begin to surface. Recurring frictions. Idioms forming. Things you taught the assistant once and may want to teach it again.",
      sub: "You decide what's signal and what isn't." },
    { kanji: "覚", phase: "Adopt",
      text: "Worthy patterns become memories — small, named lessons sensei can apply to future sessions on your behalf, with your blessing.",
      sub: "Adopt, refine, or dismiss. Always your call." }
  ];
  return (
    <section id="how" style={{
      borderTop: 'var(--hairline)', padding: 'var(--space-8) var(--space-7)'
    }}>
      <div style={{ maxWidth: 1100, margin: '0 auto' }}>
        <div className="zs-eyebrow mb-3">How it works</div>
        <h2 style={{
          fontFamily: 'var(--font-display)', fontSize: 'var(--text-3xl)',
          fontWeight: 300, letterSpacing: '-0.02em', margin: 0, marginBottom: 'var(--space-7)'
        }}>
          観 · 察 · 覚 — watch, notice, adopt.
        </h2>
        <div className="grid grid-cols-3 gap-7">
          {steps.map((s, i) => (
            <div key={i}>
              <div className="flex items-baseline gap-3 mb-4">
                <span className="zs-kanji" style={{ fontSize: 36, color: 'var(--accent)', lineHeight: 1 }}>
                  {s.kanji}
                </span>
                <div className="zs-eyebrow">{s.phase}</div>
              </div>
              <div className="text-sm text-ink" style={{ lineHeight: 1.65, marginBottom: 'var(--space-3)' }}>
                {s.text}
              </div>
              <div className="text-xs text-ink-3" style={{ fontStyle: 'italic' }}>
                {s.sub}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// Philosophy — centered single-kanji statement
function Philosophy() {
  return (
    <section id="philosophy" style={{
      borderTop: 'var(--hairline)', padding: '120px var(--space-7)'
    }}>
      <div style={{ maxWidth: 760, margin: '0 auto', textAlign: 'center' }}>
        <span className="zs-kanji" style={{ fontSize: 80, color: 'var(--accent)', lineHeight: 1 }}>静</span>
        <div className="zs-eyebrow mt-4 mb-5">Sei · stillness</div>
        <h2 style={{
          fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)',
          fontWeight: 300, lineHeight: 1.3, letterSpacing: '-0.02em',
          margin: '0 0 var(--space-5)'
        }}>
          The master observes for a long time before teaching.
        </h2>
        <p className="zs-body" style={{ maxWidth: 600, margin: '0 auto' }}>
          AI tools are getting louder. More suggestions, more autocompletes, more interrupting. Sensei moves the other way. It speaks rarely, and only when it has something specific to say. Most days it is completely silent — and that is the feature.
        </p>
      </div>
    </section>
  );
}

window.Section = Section;
window.HowItWorks = HowItWorks;
window.Philosophy = Philosophy;
