// FtrPanel — the 14-day "First-Try-Right" bar strip in the top right of Today.

function FtrPanel({ value = 78, delta = 6, data }) {
  const d = data || [0.71, 0.69, 0.74, 0.72, 0.68, 0.70, 0.73,
                     0.75, 0.72, 0.78, 0.74, 0.79, 0.76, 0.78];
  const w = 168, h = 56, gap = 2;
  const barW = (w - gap * (d.length - 1)) / d.length;
  return (
    <div className="flex items-center gap-4" style={{ paddingTop: 4 }}>
      <div className="text-right">
        <div className="zs-eyebrow" style={{ fontSize: 10 }}>First-try-right · 14d</div>
        <div className="flex items-baseline justify-end gap-1" style={{ marginTop: 4 }}>
          <span style={{ fontFamily: 'var(--font-display)', fontSize: 'var(--text-3xl)',
                         fontWeight: 400, lineHeight: 1 }}>{value}</span>
          <span className="text-xs text-ink-3">%</span>
          <span className="zs-mono text-xs"
                style={{ marginLeft: 4, color: delta >= 0 ? 'var(--success)' : 'var(--warning)' }}>
            {delta >= 0 ? "↑" : "↓"} {Math.abs(delta)}%
          </span>
        </div>
      </div>
      <svg width={w} height={h + 14} style={{ display: 'block', overflow: 'visible' }}>
        <line x1="0" x2={w} y1={h * 0.5} y2={h * 0.5}
              stroke="var(--edge)" strokeDasharray="2 3"/>
        {d.map((v, i) => {
          const bh = Math.max(3, v * h);
          const isLast = i === d.length - 1;
          return (
            <rect key={i} x={i * (barW + gap)} y={h - bh} width={barW} height={bh}
                  fill={isLast ? 'var(--accent)' : 'var(--ink-3)'}
                  opacity={isLast ? 1 : 0.45}/>
          );
        })}
        <text x={0} y={h + 11} fontSize="9" fill="var(--ink-3)"
              fontFamily="var(--font-ui)" letterSpacing="0.08em">14d ago</text>
        <text x={w} y={h + 11} fontSize="9" fill="var(--ink-3)" textAnchor="end"
              fontFamily="var(--font-ui)" letterSpacing="0.08em">today</text>
      </svg>
    </div>
  );
}

window.FtrPanel = FtrPanel;
