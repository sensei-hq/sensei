// A miniature product screenshot — used in the hero and gallery.
// Not interactive; it's marketing.

function MockToday({ width = 900, height = 560 }) {
  return (
    <div style={{
      width, maxWidth: '100%',
      height, background: 'var(--paper)',
      border: 'var(--hairline)',
      borderRadius: 'var(--radius-lg)',
      overflow: 'hidden',
      display: 'flex', flexDirection: 'column',
      boxShadow: 'var(--shadow-sm)'
    }}>
      {/* Tauri chrome */}
      <div className="zs-chrome">
        <div className="zs-traffic"><span/><span/><span/></div>
        <div className="zs-chrome-title">Sensei  先生  ·  today</div>
        <div style={{ width: 54 }}/>
      </div>

      {/* Body */}
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        {/* Sidebar */}
        <aside style={{ width: 180, borderRight: 'var(--hairline)', padding: 16, fontSize: 12 }}>
          <div className="flex items-baseline gap-2 mb-4">
            <span className="zs-kanji" style={{ fontSize: 16, color: 'var(--accent)' }}>先</span>
            <span style={{ fontFamily: 'var(--font-display)' }}>Sensei</span>
          </div>
          <div className="zs-eyebrow mb-2" style={{ fontSize: 9 }}>Observatory</div>
          {[["今","Today",true],["場","Projects"],["刻","Sessions"],["察","Insights"],["覚","Memories"]].map(([k,l,active], i) => (
            <div key={i} style={{
              padding: '5px 8px', borderRadius: 4,
              background: active ? 'var(--paper-3)' : 'transparent',
              color: active ? 'var(--ink)' : 'var(--ink-2)',
              display: 'flex', gap: 8, alignItems: 'center', marginBottom: 1
            }}>
              <span className="zs-kanji" style={{ fontSize: 11, color: active ? 'var(--accent)' : 'var(--ink-3)' }}>{k}</span>
              <span>{l}</span>
            </div>
          ))}
        </aside>

        {/* Main */}
        <main style={{ flex: 1, padding: 24, overflow: 'hidden' }}>
          <div className="zs-eyebrow" style={{ fontSize: 10 }}>Wed · 22 Apr</div>
          <div style={{ fontFamily: 'var(--font-display)', fontSize: 22, fontWeight: 400, marginTop: 6, marginBottom: 24 }}>
            Good morning, Aiko.
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '72px 1fr', gap: 20, paddingTop: 16, borderTop: 'var(--hairline)' }}>
            <div className="zs-kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>聴</div>
            <div>
              <div style={{ fontFamily: 'var(--font-display)', fontSize: 18, marginBottom: 8 }}>
                The AI does not know your auth.
              </div>
              <div style={{ fontSize: 11.5, color: 'var(--ink-2)', lineHeight: 1.55 }}>
                Three sessions corrected this week in lumen-auth — all touched refresh or device flow.
              </div>
            </div>
          </div>

          <div style={{ marginTop: 20, paddingTop: 12, borderTop: 'var(--hairline)' }}>
            <div className="zs-eyebrow mb-2" style={{ fontSize: 9 }}>Also worth noticing</div>
            {[
              ["繰","Cache invalidation missed again in s-2891.","3rd time","warn"],
              ["昇","Canvas smoothing pattern promoted to rule.","+7%","good"]
            ].map(([k, t, tag, tone], i) => (
              <div key={i} className="flex items-baseline gap-3" style={{
                padding: '8px 0', borderBottom: 'var(--hairline)'
              }}>
                <span className="zs-kanji" style={{
                  fontSize: 14, width: 20,
                  color: tone === "warn" ? 'var(--warning)' : 'var(--success)'
                }}>{k}</span>
                <span style={{ flex: 1, fontSize: 11.5, color: 'var(--ink-2)' }}>{t}</span>
                <span className="zs-mono" style={{
                  fontSize: 9.5, padding: '2px 6px', borderRadius: 3,
                  color: tone === "warn" ? 'var(--warning)' : 'var(--success)',
                  background: tone === "warn" ? 'var(--warning-soft)' : 'var(--success-soft)'
                }}>{tag}</span>
              </div>
            ))}
          </div>
        </main>
      </div>
    </div>
  );
}

window.MockToday = MockToday;
