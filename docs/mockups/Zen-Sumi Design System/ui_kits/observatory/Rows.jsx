// InsightRow / SessionRow / LearnedRow — the small list rows below the hero.

function InsightRow({ kanji, label, text, tag, tone = "mute" }) {
  const toneColor = tone === "warn"  ? 'var(--warning)' :
                    tone === "good"  ? 'var(--success)' :
                    tone === "accent" ? 'var(--accent)'  : 'var(--ink-3)';
  const tagBg    = tone === "warn"  ? 'var(--warning-soft)' :
                    tone === "good"  ? 'var(--success-soft)' :
                    tone === "accent" ? 'var(--accent-soft)'  : 'var(--paper-3)';
  return (
    <div className="border-b py-3 flex items-baseline gap-4">
      <span className="zs-kanji" style={{ fontSize: 18, color: toneColor, width: 24, flexShrink: 0 }}>
        {kanji}
      </span>
      <div style={{ flex: 1 }}>
        <div className="zs-eyebrow" style={{ marginBottom: 2, fontSize: 10 }}>{label}</div>
        <div className="zs-body-sm" style={{ color: 'var(--ink-2)' }}>{text}</div>
      </div>
      <span className="zs-mono text-xs" style={{ color: toneColor, padding: '3px 8px',
                    borderRadius: 'var(--radius-sm)', background: tagBg }}>
        {tag}
      </span>
    </div>
  );
}

function SessionRow({ project, title, time, duration, ftr }) {
  return (
    <div className="border-b py-3 flex items-center gap-3">
      <span className={`zs-dot ${ftr ? 'zs-dot-success' : 'zs-dot-warning'}`}/>
      <span className="zs-mono text-xs text-ink-3" style={{ width: 100 }}>{project}</span>
      <span className="text-sm text-ink" style={{ flex: 1 }}>{title}</span>
      <span className="zs-mono text-xs text-ink-3">{time} · {duration}</span>
    </div>
  );
}

function LearnedRow({ when, scope, what, source }) {
  return (
    <div className="border-b" style={{ padding: '14px 0' }}>
      <div className="flex items-baseline gap-2" style={{ marginBottom: 4 }}>
        <span className="zs-mono text-xs text-ink-3">{when}</span>
        <span className="text-xs text-ink-4">·</span>
        <span className="zs-mono text-xs text-accent">{scope}</span>
      </div>
      <div className="text-sm text-ink">{what}</div>
      <div className="text-xs text-ink-4" style={{ marginTop: 4 }}>{source}</div>
    </div>
  );
}

window.InsightRow = InsightRow;
window.SessionRow = SessionRow;
window.LearnedRow = LearnedRow;
