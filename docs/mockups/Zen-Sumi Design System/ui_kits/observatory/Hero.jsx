// The hero koan — the single daily teaching.
// Big kanji on the left, koan + body + action on the right.

function HeroKoan({ kanji = "聴", phase = "Listen", koan, body, action, source }) {
  return (
    <section style={{
      display: 'grid', gridTemplateColumns: '128px 1fr', gap: 32,
      padding: 'var(--space-7) 0',
      borderTop: 'var(--hairline)', borderBottom: 'var(--hairline)',
    }}>
      {/* Kanji block */}
      <div style={{ position: 'relative' }}>
        <div className="zs-kanji" style={{
          fontSize: 96, color: 'var(--accent)', lineHeight: 1
        }}>{kanji}</div>
        <div className="zs-eyebrow" style={{
          position: 'absolute', left: -4, top: -2,
          writingMode: 'vertical-rl', transform: 'rotate(180deg)',
          fontSize: 9, height: 96
        }}>{phase}</div>
      </div>

      {/* Body */}
      <div className="flex flex-col">
        <div style={{
          fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)',
          fontWeight: 400, letterSpacing: '-0.01em', lineHeight: 1.2,
          marginBottom: 'var(--space-3)',
        }}>{koan}</div>

        <p className="zs-body" style={{ margin: 0, marginBottom: 'var(--space-4)', maxWidth: 620 }}>
          {body}
        </p>

        <div className="flex items-center gap-4" style={{ marginTop: 'var(--space-2)' }}>
          {action && (
            <button className="zs-btn zs-btn-primary">
              {action} <span className="zs-kanji" style={{ color: 'var(--accent)', fontSize: 14 }}>→</span>
            </button>
          )}
          <div className="flex items-center gap-2" style={{ color: 'var(--accent)' }}>
            <span className="zs-ink-dot"/>
            <span style={{ fontSize: 'var(--text-sm)' }}>Projected FTR + 14% in Lumen Cloud</span>
          </div>
          <span style={{ flex: 1 }}/>
          <span className="zs-mono text-xs text-ink-3">{source}</span>
        </div>
      </div>
    </section>
  );
}

window.HeroKoan = HeroKoan;
