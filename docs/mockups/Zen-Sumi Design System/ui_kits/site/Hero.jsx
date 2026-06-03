// Hero — kanji anchor, eyebrow, big display headline, lead, CTA + product mock.

function Hero() {
  const { DownloadCTA, MockToday } = window;
  return (
    <section style={{
      maxWidth: 1100, margin: '0 auto',
      padding: 'var(--space-6) var(--space-7) var(--space-8)'
    }}>
      <div className="flex items-baseline gap-3 mb-5">
        <span className="zs-kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>観</span>
        <div className="zs-eyebrow">Kan · to observe</div>
      </div>

      <h1 style={{
        fontFamily: 'var(--font-display)', fontSize: 'var(--text-4xl)', fontWeight: 300,
        lineHeight: 1.1, letterSpacing: '-0.025em', maxWidth: 820, margin: 0
      }}>
        A quiet companion for AI-assisted work.
      </h1>

      <p className="zs-body" style={{ marginTop: 'var(--space-5)', maxWidth: 560 }}>
        Sensei watches your sessions with AI assistants — then surfaces the patterns you're too close to see. Not a chatbot. Not a copilot. A patient observer.
      </p>

      <div className="flex items-center gap-4" style={{ marginTop: 'var(--space-6)' }}>
        <DownloadCTA size="lg"/>
        <a href="#how" className="text-ink-2" style={{ fontSize: 'var(--text-sm)' }}>
          See how it works ↓
        </a>
      </div>
      <div className="text-xs text-ink-3 mt-3">
        Free · Local-first · No account
      </div>

      <div style={{ marginTop: 'var(--space-7)', display: 'flex', justifyContent: 'center' }}>
        <MockToday width={900} height={520}/>
      </div>
    </section>
  );
}

window.Hero = Hero;
