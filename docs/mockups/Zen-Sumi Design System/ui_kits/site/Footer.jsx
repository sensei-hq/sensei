// FAQ — expandable hairline-divided rows.

function Faq() {
  const qs = [
    { q: "Which AI assistants does it observe?",
      a: "Any AI assistant that speaks the Model Context Protocol. The list grows as MCP grows." },
    { q: "Does sensei see my code?",
      a: "Only what passes through your AI tool's session. It runs locally and stores everything in a SQLite file you can inspect or delete at any time." },
    { q: "Will it slow down my machine?",
      a: "Sensei is a Tauri app — small binary, low memory. The observer is event-driven; it only does work when a session happens." },
    { q: "Can I export my memories?",
      a: "Yes. Settings → Export gives you a JSON dump of every pattern, memory, and adopted teaching. Import is also supported." },
    { q: "What's the long-term plan?",
      a: "Sensei stays local-first and free. We may add an optional paid tier later for cross-machine sync, but the core promise — quiet, local, observant — never changes." }
  ];
  return (
    <section id="faq" style={{
      borderTop: 'var(--hairline)', padding: 'var(--space-8) var(--space-7)'
    }}>
      <div style={{ maxWidth: 880, margin: '0 auto' }}>
        <div className="zs-eyebrow mb-3">Frequently asked</div>
        <h2 style={{
          fontFamily: 'var(--font-display)', fontSize: 'var(--text-2xl)',
          fontWeight: 400, letterSpacing: '-0.015em', margin: '0 0 var(--space-6)'
        }}>Common questions, plain answers.</h2>
        <div>
          {qs.map((it, i) => (
            <details key={i} style={{
              borderTop: 'var(--hairline)',
              padding: '20px 0',
              ...(i === qs.length - 1 ? { borderBottom: 'var(--hairline)' } : {})
            }}>
              <summary style={{
                cursor: 'pointer', listStyle: 'none',
                display: 'flex', justifyContent: 'space-between',
                fontSize: 'var(--text-base)', color: 'var(--ink)'
              }}>
                <span>{it.q}</span>
                <span className="zs-kanji text-ink-3">+</span>
              </summary>
              <div className="text-sm text-ink-2" style={{ lineHeight: 1.7, marginTop: 'var(--space-3)', maxWidth: 640 }}>
                {it.a}
              </div>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer style={{
      borderTop: 'var(--hairline)', padding: 'var(--space-6) var(--space-7)',
      fontSize: 'var(--text-xs)', color: 'var(--ink-3)'
    }}>
      <div style={{ maxWidth: 1100, margin: '0 auto',
                    display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div className="flex items-baseline gap-2">
          <span className="zs-kanji" style={{ fontSize: 13, letterSpacing: '-0.04em' }}>先生</span>
          <span style={{ fontFamily: 'var(--font-display)', fontSize: 13, color: 'var(--ink-2)' }}>Sensei</span>
          <span className="zs-mono" style={{ marginLeft: 12 }}>v0.4.2</span>
        </div>
        <div className="flex gap-5">
          <a href="#privacy">Privacy</a>
          <a href="#faq">FAQ</a>
          <a href="#github">GitHub</a>
          <a href="#twitter">Twitter</a>
        </div>
      </div>
    </footer>
  );
}

window.Faq = Faq;
window.Footer = Footer;
