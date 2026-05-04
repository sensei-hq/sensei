// Sessions — Digest v2 (zen edition)
//
// Keeps the retro cards on top. Replaces the dense session list with one
// quiet chart that answers the question "what shape are my sessions
// taking?" — and lets the user flip between four visual treatments:
//
//   A · STREAM        stacked area over 7 days · good / corrected / abandoned
//   B · CONSTELLATION scatter · x = duration · y = day · color = quality
//   C · BANDS         horizontal stacked bars per day
//   D · PULSE         minimal · each session = a tick · height = duration
//
// All four use the existing window.SESSIONS fixture and tokens.

const { useState: ss2S } = React;

// ─── helpers ──────────────────────────────────────────────────────────
function parseDuration(d) {
  // "12m" | "1h 04m" | "2h 11m"
  let h = 0, m = 0;
  const hm = d.match(/(\d+)h/); if (hm) h = +hm[1];
  const mm = d.match(/(\d+)m/); if (mm) m = +mm[1];
  return h * 60 + m;
}

// good = first-try-right shipped · ugly = abandoned · bad = shipped with corrections
function quality(s) {
  if (s.outcome === "abandoned") return "ugly";
  if (s.ftr) return "good";
  return "bad";
}

const QUALITY = {
  good: { color: 'var(--jade)',     label: "first-try"    },
  bad:  { color: 'var(--amber)',    label: "corrected"    },
  ugly: { color: 'var(--shu)',      label: "abandoned"    }
};

// Day axis covers up to 30d; the time-range filter just narrows what we render.
const DAYS_30 = (() => {
  const labels = [];
  for (let i = 30; i >= 8; i--) labels.push(`${i} days ago`);
  labels.push("1 week ago","6 days ago","5 days ago","4 days ago",
              "3 days ago","2 days ago","yesterday","today");
  return labels;
})();
const DAYS_ORDERED = DAYS_30.slice(-8);
const DAYS_SHORT   = (label) => {
  if (label === "today")      return "Today";
  if (label === "yesterday")  return "Yest";
  if (label === "1 week ago") return "−7";
  const m = label.match(/(\d+) days? ago/);
  return m ? `−${m[1]}` : label;
};

const RANGE_DAYS = { "7d": 8, "30d": 30, "90d": 30, "all": 30 }; // fixture maxes at 30
function daysForRange(r) { return DAYS_30.slice(-RANGE_DAYS[r]); }

// Synthesize prior-week sessions on the fly so 30d view has data to compare.
function synthesizeHistory(real) {
  const out = [...real];
  const projects = ["lumen-cloud","koto-editor","lumen-auth","tabi-sdk","ginkgo","lumen-studio"];
  for (let i = 8; i <= 30; i++) {
    const day = `${i} days ago`;
    // 1-3 synthetic sessions per day; degrade FTR mildly going back
    const n = 1 + (hash(day) % 3);
    for (let k = 0; k < n; k++) {
      const seed = hash(day + k);
      const ftr  = (seed % 100) > 55 - Math.min(20, i);  // older = lower FTR
      const abandoned = (seed % 100) > 92;
      const mins = 25 + (seed % 110);
      out.push({
        id: `s-h${i}${k}`,
        title: "—",
        project: projects[seed % projects.length],
        when: day,
        time: `${9 + (seed % 8)}:${(seed * 7) % 60}`.padStart(5, "0"),
        duration: `${Math.floor(mins/60) ? Math.floor(mins/60)+"h " : ""}${mins%60}m`,
        corrections: abandoned ? 4 : ftr ? 0 : 1 + (seed % 3),
        ftr,
        agent: "claude-code",
        outcome: abandoned ? "abandoned" : "shipped",
        synthetic: true
      });
    }
  }
  return out;
}

// ─── Page ─────────────────────────────────────────────────────────────
function SessionsDigestZen({ initialChart = "trend", projectFilter = null, projectLabel = null }) {
  const D = window.SESSIONS;
  const [chart, setChart] = ss2S(initialChart);
  const [range, setRange] = ss2S("7d");
  const [collapsed, setCollapsed] = ss2S(false);
  const [miniMode, setMiniMode]   = ss2S("trend"); // numbers | trend | stream | constellation | bands | pulse

  const allSessions = synthesizeHistory(D.sessions);
  const days = daysForRange(range);
  const dayset = new Set(days);
  const enriched = allSessions
    .filter(s => dayset.has(s.when))
    .filter(s => !projectFilter || s.project === projectFilter || s.solution === projectFilter)
    .map(s => ({ ...s, mins: parseDuration(s.duration), q: quality(s) }));
  const totals = {
    count: enriched.length,
    projects: new Set(enriched.map(s => s.project)).size,
    good: enriched.filter(s => s.q === "good").length,
    bad:  enriched.filter(s => s.q === "bad").length,
    ugly: enriched.filter(s => s.q === "ugly").length,
    medianMins: median(enriched.map(s => s.mins))
  };

  return (
    <div className="sensei" data-screen-label="Observatory · Sessions · Digest"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <ZenHero
        totals={totals} range={range} setRange={setRange}
        collapsed={collapsed}
        miniMode={miniMode}
        onCycleMiniMode={() => setMiniMode(m => nextMode(m))}
        sessions={enriched}
        days={days}
        checkpoints={D.checkpoints}
        projectLabel={projectFilter ? (projectLabel || projectFilter) : null}/>

      <ZenScrollLayout
        chart={chart} setChart={setChart}
        chartNode={
          chart === "stream"        ? <StreamChart sessions={enriched} days={days} checkpoints={D.checkpoints}/> :
          chart === "trend"         ? <TrendChart sessions={enriched} days={days} checkpoints={D.checkpoints}/> :
          chart === "constellation" ? <ConstellationChart sessions={enriched} days={days}/> :
          chart === "bands"         ? <BandsChart sessions={enriched} days={days}/> :
                                       <PulseChart sessions={enriched} days={days}/>
        }
        sessions={enriched}
        days={days}
        totals={totals}
        retro={D.retro}
        collapsed={collapsed}
        setCollapsed={setCollapsed}/>
    </div>
  );
}

// ─── Scroll layout: chart on top (collapses to a slim sticky header
//      on scroll), retrospective below (grows freely). When collapsed,
//      click the header to toggle between a tiny chart preview and a
//      retro stats summary.
// ─────────────────────────────────────────────────────────────────────
const { useRef: ssRef } = React;

function ZenScrollLayout({ chart, setChart, chartNode, sessions, days, totals, retro,
                            collapsed, setCollapsed }) {
  const scrollRef = ssRef(null);

  React.useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const onScroll = () => setCollapsed(el.scrollTop > 30);
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [setCollapsed]);

  const totalNotGoing = retro.not_going.length;
  const totalGoingWell = retro.going_well.length;
  const totalInsights  = retro.insights.length;

  return (
    <div ref={scrollRef}
         style={{ flex: 1, overflow: 'auto', minHeight: 0, position: 'relative' }}>

      {/* ── Chart (top of scroll area, fades to 0 when collapsed —
              the mini version lives inside the hero above) ────────── */}
      <div style={{ overflow: 'hidden',
                     maxHeight: collapsed ? 0 : 720,
                     opacity: collapsed ? 0 : 1,
                     transition: 'max-height 0.32s ease, opacity 0.18s ease',
                     padding: collapsed ? '0 40px' : '22px 40px 18px' }}>
        <ZenChartFrame chart={chart} setChart={setChart}>
          {chartNode}
        </ZenChartFrame>
      </div>

      {/* ── Retrospective (below the chart, grows freely) ─────── */}
      <section style={{ padding: '24px 40px 56px', minHeight: 1100 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 14,
                       paddingBottom: 8, borderBottom: 'var(--hairline)' }}>
          <span className="kanji" style={{ fontSize: 16, color: 'var(--shu)' }}>省</span>
          <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0,
                        color: 'var(--sumi)' }}>Retrospective · last 7 days</h3>
          <span style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
            · what sensei sees across your sessions
          </span>
          <span style={{ flex: 1 }}/>
          <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-4)' }}>
            {totalGoingWell + totalNotGoing + totalInsights} observations
          </span>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 14 }}>
          <RetroLane title="Going well"     accent="var(--jade)"   items={retro.going_well} positive/>
          <RetroLane title="Not going well" accent="var(--shu)"    items={retro.not_going}/>
          <RetroLane title="Insights"       accent="var(--sumi-2)" items={retro.insights}/>
        </div>
      </section>
    </div>
  );
}

// Mini chart preview shown in the collapsed header — labels + tiny sparkline.
// ─── Mini visualizations for the collapsed hero ───────────────────────
// Modes the floating cycler steps through.
const MINI_MODES = ["numbers", "trend", "stream", "constellation", "bands", "pulse"];

function nextMode(m) {
  const i = MINI_MODES.indexOf(m);
  return MINI_MODES[(i + 1) % MINI_MODES.length];
}

// Glyphs for the floating cycler — kanji per mode.
const MINI_GLYPH = {
  numbers: "数", trend: "線", stream: "流",
  constellation: "星", bands: "帯", pulse: "脈"
};

// Tiny FTR sparkline — used by the "trend" mode.
// Renders adoption checkpoints as dotted vertical line + soft circle, and shows
// the before/after FTR mean as horizontal segments separated by each marker.
// Hover any marker to reveal one-line detail; click routes to the memory view.
function MiniTrend({ sessions, days, checkpoints = [] }) {
  const W = 168, H = 56;
  const padT = 4, padB = 3;
  const innerH = H - padT - padB;

  // Per-day FTR — days[] is chronological oldest → newest already
  const ordered = days;
  const byDay = ordered.map(d => {
    const xs = sessions.filter(s => s.when === d);
    const good = xs.filter(s => s.q === "good").length;
    return xs.length ? good / xs.length : null;
  });
  const xStep = ordered.length > 1 ? W / (ordered.length - 1) : 0;
  const xAt = i => i * xStep;
  const yAt = v => padT + (1 - v) * innerH;

  // Sparkline path (only valid days)
  const valid = byDay.map((v, i) => ({ v, i })).filter(p => p.v != null);
  const linePts = valid.map(p => [xAt(p.i), yAt(p.v)]);
  const path = linePts.length
    ? "M " + linePts.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join(" L ")
    : "";
  const areaPath = path
    ? `${path} L ${linePts[linePts.length-1][0].toFixed(1)} ${H-padB} L ${linePts[0][0].toFixed(1)} ${H-padB} Z`
    : "";

  // Map each checkpoint to a day index in the visible window. We try the
  // checkpoint's own .when first; if not in range, drop it.
  const cks = checkpoints
    .map(c => ({ ...c, idx: ordered.indexOf(c.when) }))
    .filter(c => c.idx >= 0)
    .sort((a, b) => a.idx - b.idx);

  // Segments between checkpoints → average FTR per segment for the baseline lines
  const segments = [];
  let segStart = 0;
  cks.forEach(c => {
    segments.push({ from: segStart, to: c.idx });
    segStart = c.idx + 1;
  });
  segments.push({ from: segStart, to: ordered.length - 1 });
  const segs = segments.map(seg => {
    const slice = byDay.slice(seg.from, seg.to + 1).filter(v => v != null);
    return {
      ...seg,
      avg: slice.length ? slice.reduce((a, b) => a + b, 0) / slice.length : null
    };
  }).filter(s => s.avg !== null && s.from <= s.to);

  const [hover, setHover] = React.useState(null); // { ck, x }

  const onCkClick = (ck) => {
    // Route to memory / timeline view for that day. The host listens for this
    // and can switch screens or navigate. Falls back to console if unhandled.
    try {
      window.parent?.postMessage(
        { type: "sensei:open-memory", checkpoint: ck.id, when: ck.when }, "*");
    } catch (_) {}
    window.dispatchEvent(new CustomEvent("sensei:open-memory",
      { detail: { checkpoint: ck.id, when: ck.when } }));
  };

  return (
    <div style={{ position: 'relative', width: W, height: H }}>
      <svg width={W} height={H} style={{ overflow: 'visible', display: 'block' }}>
        {/* Sparkline */}
        <path d={areaPath} fill="var(--jade)" opacity="0.10"/>
        <path d={path} fill="none" stroke="var(--jade)" strokeWidth="1.6"
              strokeLinecap="round" strokeLinejoin="round"/>
        {linePts.map(([x, y], k) => (
          <circle key={k} cx={x} cy={y}
                  r={k === linePts.length - 1 ? 3 : 1.6}
                  fill="var(--jade)"/>
        ))}

        {/* Per-segment mean line — the "before / after" reading */}
        {segs.map((seg, i) => {
          // Stop the line just shy of any flanking marker so the dotted
          // verticals read as separators.
          const x1 = xAt(seg.from) + (i === 0 ? 0 : 4);
          const x2 = xAt(seg.to)   - (i === segs.length - 1 ? 0 : 4);
          if (x2 <= x1) return null;
          const y0 = yAt(seg.avg);
          return (
            <line key={i} x1={x1} x2={x2} y1={y0} y2={y0}
                  stroke="var(--shu)" strokeWidth={1} opacity={0.55}
                  strokeDasharray="4 2"/>
          );
        })}

        {/* Checkpoint markers — dotted vertical + semi-transparent circle */}
        {cks.map(c => {
          const cx = xAt(c.idx);
          const isHovered = hover?.ck.id === c.id;
          return (
            <g key={c.id}
               style={{ cursor: 'pointer' }}
               onMouseEnter={() => setHover({ ck: c, x: cx })}
               onMouseLeave={() => setHover(null)}
               onClick={() => onCkClick(c)}>
              <line x1={cx} x2={cx} y1={padT - 2} y2={H - padB}
                    stroke="var(--sumi-2)" strokeWidth={1}
                    strokeDasharray="2 2"
                    opacity={isHovered ? 0.85 : 0.55}/>
              <circle cx={cx} cy={padT + innerH * 0.5} r={5}
                      fill="var(--shu)"
                      opacity={isHovered ? 0.45 : 0.25}/>
              <circle cx={cx} cy={padT + innerH * 0.5} r={2.2}
                      fill="var(--shu)"
                      opacity={isHovered ? 1 : 0.85}/>
              {/* Hit area */}
              <rect x={cx - 8} y={0} width={16} height={H}
                    fill="transparent"/>
            </g>
          );
        })}
      </svg>

      {/* Hover tooltip — single detail at a time */}
      {hover && (
        <div style={{
          position: 'absolute',
          left: Math.max(0, Math.min(W - 200, hover.x - 100)),
          bottom: H + 6,
          width: 200,
          padding: '7px 10px 8px',
          background: 'var(--paper)',
          border: 'var(--hairline)',
          borderRadius: 4,
          boxShadow: '0 4px 14px rgba(0,0,0,0.08)',
          fontSize: 11, lineHeight: 1.4,
          pointerEvents: 'none',
          zIndex: 5
        }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 6, marginBottom: 2 }}>
            <span style={{ fontSize: 9, letterSpacing: '0.14em',
                            textTransform: 'uppercase', color: 'var(--sumi-3)' }}>
              {hover.ck.when}
            </span>
            <span style={{ fontSize: 9, color: 'var(--shu)',
                            letterSpacing: '0.1em', textTransform: 'uppercase' }}>
              · adopted
            </span>
          </div>
          <div style={{ color: 'var(--sumi)', fontWeight: 500 }}>
            {hover.ck.title}
          </div>
          <div style={{ fontSize: 10, color: 'var(--sumi-3)',
                          marginTop: 3, fontStyle: 'italic' }}>
            click for memory view →
          </div>
        </div>
      )}
    </div>
  );
}

// Tiny stacked area — quality counts per day.
function MiniStream({ sessions, days }) {
  const W = 168, H = 56;
  const counts = days.map(d => {
    const xs = sessions.filter(s => s.when === d);
    return {
      good: xs.filter(s => s.q === "good").length,
      bad:  xs.filter(s => s.q === "bad").length,
      ugly: xs.filter(s => s.q === "ugly").length
    };
  });
  const max = Math.max(1, ...counts.map(c => c.good + c.bad + c.ugly));
  const xStep = days.length > 1 ? W / (days.length - 1) : 0;
  const layers = ["ugly","bad","good"];
  const colors = { good: QUALITY.good.color, bad: QUALITY.bad.color, ugly: QUALITY.ugly.color };
  let baseAcc = days.map(_ => 0);
  return (
    <svg width={W} height={H} style={{ overflow: 'visible' }}>
      {layers.map(layer => {
        const top = counts.map((c, i) => baseAcc[i] + c[layer]);
        const path = "M " + top.map((v, i) =>
          `${(i*xStep).toFixed(1)} ${(H - (v/max) * (H-2)).toFixed(1)}`
        ).join(" L ") +
          " L " + ((days.length-1)*xStep).toFixed(1) + " " + H +
          " L 0 " + H + " Z";
        baseAcc = top;
        return <path key={layer} d={path} fill={colors[layer]} opacity="0.85"/>;
      })}
    </svg>
  );
}

// Tiny scatter — duration vs day.
function MiniConstellation({ sessions, days }) {
  const W = 168, H = 56;
  const maxMin = Math.max(60, ...sessions.map(s => s.mins));
  const xStep = days.length > 1 ? W / (days.length - 1) : 0;
  return (
    <svg width={W} height={H} style={{ overflow: 'visible' }}>
      {sessions.map((s, i) => {
        const di = days.indexOf(s.when);
        if (di < 0) return null;
        const cx = di * xStep + (hash(s.id) % 9 - 4) * 0.5;
        const cy = H - 2 - (s.mins / maxMin) * (H - 4);
        return <circle key={s.id} cx={cx} cy={cy} r={2.2}
                       fill={QUALITY[s.q].color} opacity="0.85"/>;
      })}
    </svg>
  );
}

// Tiny stacked bars per day.
function MiniBands({ sessions, days }) {
  const W = 168, H = 56;
  const bw = (W - (days.length - 1) * 2) / days.length;
  const counts = days.map(d => {
    const xs = sessions.filter(s => s.when === d);
    return {
      good: xs.filter(s => s.q === "good").length,
      bad:  xs.filter(s => s.q === "bad").length,
      ugly: xs.filter(s => s.q === "ugly").length
    };
  });
  const max = Math.max(1, ...counts.map(c => c.good + c.bad + c.ugly));
  return (
    <svg width={W} height={H} style={{ overflow: 'visible' }}>
      {counts.map((c, i) => {
        const x = i * (bw + 2);
        let yAcc = H;
        return (
          <g key={i}>
            {["good","bad","ugly"].map(q => {
              const h = (c[q] / max) * (H - 2);
              const y = yAcc - h;
              yAcc = y;
              return <rect key={q} x={x} y={y} width={bw} height={h}
                            fill={QUALITY[q].color} opacity="0.85"/>;
            })}
          </g>
        );
      })}
    </svg>
  );
}

// Tiny pulse — one tick per session.
function MiniPulse({ sessions, days }) {
  const W = 168, H = 56;
  const xStep = days.length > 1 ? W / (days.length - 1) : 0;
  return (
    <svg width={W} height={H} style={{ overflow: 'visible' }}>
      <line x1="0" x2={W} y1={H-2} y2={H-2} stroke="var(--paper-edge)" strokeWidth="0.5"/>
      {sessions.map(s => {
        const di = days.indexOf(s.when);
        if (di < 0) return null;
        const cx = di * xStep + (hash(s.id) % 11 - 5) * 0.4;
        const len = Math.max(4, Math.min(H - 4, s.mins / 10));
        return (
          <line key={s.id} x1={cx} x2={cx} y1={H - 2 - len} y2={H - 2}
                stroke={QUALITY[s.q].color} strokeWidth="1.4"
                strokeLinecap="round" opacity="0.85"/>
        );
      })}
    </svg>
  );
}

// Numbers mode — original 3 dots + median.
function MiniNumbers({ totals }) {
  return (
    <div style={{ display: 'flex', gap: 16 }}>
      <DotStat dot={QUALITY.good.color} n={totals.good}  l="first-try"/>
      <DotStat dot={QUALITY.bad.color}  n={totals.bad}   l="corrected"/>
      <DotStat dot={QUALITY.ugly.color} n={totals.ugly}  l="abandoned"/>
      <div style={{ width: 1, background: 'var(--paper-edge)' }}/>
      <DotStat n={`${Math.floor(totals.medianMins/60)}h ${totals.medianMins % 60}m`}
               l="median" mono/>
    </div>
  );
}

// Renders the mini view for a given mode (chart) + headline number.
function MiniView({ mode, totals, sessions, days, checkpoints = [] }) {
  if (mode === "numbers") return <MiniNumbers totals={totals}/>;

  const ChartEl = {
    trend: MiniTrend, stream: MiniStream, constellation: MiniConstellation,
    bands: MiniBands, pulse: MiniPulse
  }[mode];

  // Headline number per mode.
  const headline = (() => {
    if (mode === "trend") {
      const last = days.slice().reverse().find(d =>
        sessions.some(s => s.when === d));
      if (!last) return { v: "—", l: "first-try · 7d" };
      const xs = sessions.filter(s => s.when === last);
      const good = xs.filter(s => s.q === "good").length;
      return { v: `${Math.round((good / xs.length) * 100)}%`, l: "first-try · today" };
    }
    if (mode === "constellation" || mode === "pulse") {
      return { v: sessions.length, l: "sessions · 7d" };
    }
    // stream, bands → daily mean of session count
    const perDay = days.map(d => sessions.filter(s => s.when === d).length);
    const mean = Math.round(perDay.reduce((a,b)=>a+b,0) / Math.max(1, perDay.length));
    return { v: mean, l: "per day · avg" };
  })();

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
      <ChartEl sessions={sessions} days={days} checkpoints={checkpoints}/>
      <div style={{ minWidth: 56 }}>
        <div style={{ fontSize: 18, lineHeight: 1, fontWeight: 300, color: 'var(--sumi)',
                       fontFeatureSettings: '"tnum"' }}>{headline.v}</div>
        <div style={{ fontSize: 9, letterSpacing: '0.12em', color: 'var(--sumi-4)',
                       marginTop: 3, textTransform: 'uppercase' }}>{headline.l}</div>
      </div>
    </div>
  );
}

// Floating cycler — small kanji glyph that advances the mode.
function MiniCycler({ mode, onCycle }) {
  return (
    <button onClick={onCycle} title={`viewing ${mode} · click to cycle`}
            style={{ width: 32, height: 32, borderRadius: '50%',
                      background: 'var(--paper-2)',
                      border: 'var(--hairline)',
                      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                      cursor: 'pointer', padding: 0, marginLeft: 12,
                      transition: 'background 0.15s' }}
            onMouseEnter={e => e.currentTarget.style.background = 'var(--paper)'}
            onMouseLeave={e => e.currentTarget.style.background = 'var(--paper-2)'}>
      <span className="kanji" style={{ fontSize: 14, color: 'var(--shu)',
                                         lineHeight: 1 }}>
        {MINI_GLYPH[mode]}
      </span>
    </button>
  );
}

// (Old single-purpose ZenMiniChart kept as alias for any leftover refs.)
function ZenMiniChart({ chart, sessions = [], days = [] }) {
  // Build per-day FTR (good / total) — same metric as the full Trend chart.
  const W = 140, H = 36;
  const byDay = days.map(d => {
    const xs = sessions.filter(s => s.when === d);
    const good = xs.filter(s => s.q === "good").length;
    return xs.length ? good / xs.length : null;
  });
  const valid = byDay.map((v, i) => ({ v, i })).filter(p => p.v != null);
  const xStep = valid.length > 1 ? W / (valid.length - 1) : 0;
  const points = valid.map((p, k) => [k * xStep, H - 4 - p.v * (H - 8)]);
  const path = points.length
    ? "M " + points.map(([x, y]) => `${x.toFixed(1)} ${y.toFixed(1)}`).join(" L ")
    : "";
  const areaPath = path
    ? `${path} L ${points[points.length-1][0].toFixed(1)} ${H} L ${points[0][0].toFixed(1)} ${H} Z`
    : "";

  // Latest day's FTR as the headline number.
  const latest = valid.length ? valid[valid.length - 1].v : null;
  const latestPct = latest != null ? Math.round(latest * 100) : null;

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
      {points.length > 0 && (
        <svg width={W} height={H} style={{ overflow: 'visible' }}>
          <path d={areaPath} fill="var(--jade)" opacity="0.12"/>
          <path d={path} fill="none" stroke="var(--jade)" strokeWidth="1.4"
                strokeLinecap="round" strokeLinejoin="round"/>
          {points.map(([x, y], k) => (
            <circle key={k} cx={x} cy={y}
                    r={k === points.length - 1 ? 2.4 : 1.4}
                    fill="var(--jade)"/>
          ))}
        </svg>
      )}
      <div style={{ minWidth: 56 }}>
        <div style={{ fontSize: 18, lineHeight: 1, fontWeight: 300, color: 'var(--sumi)',
                       fontFeatureSettings: '"tnum"' }}>
          {latestPct != null ? `${latestPct}%` : "—"}
        </div>
        <div style={{ fontSize: 9, letterSpacing: '0.12em', color: 'var(--sumi-4)',
                       marginTop: 3, textTransform: 'uppercase' }}>
          first-try · 7d
        </div>
      </div>
    </div>
  );
}

// Mini retro stats — 3 dots + counts, calm and matched to the lanes.
function ZenMiniRetro({ going, notGoing, insights }) {
  const dots = [
    { c: 'var(--jade)',  n: going,    l: "going well" },
    { c: 'var(--shu)',   n: notGoing, l: "not going" },
    { c: 'var(--sumi-2)', n: insights, l: "insights" }
  ];
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 18 }}>
      <span className="kanji" style={{ fontSize: 22, color: 'var(--shu)',
                                         lineHeight: 1 }}>省</span>
      <div style={{ display: 'flex', gap: 18 }}>
        {dots.map(d => (
          <div key={d.l} style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
            <span style={{ width: 7, height: 7, borderRadius: '50%', background: d.c }}/>
            <span className="mono" style={{ fontSize: 13, color: 'var(--sumi)',
                          fontFeatureSettings: '"tnum"' }}>{d.n}</span>
            <span style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>{d.l}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── Hero ─────────────────────────────────────────────────────────────
function ZenHero({ totals, range, setRange,
                   collapsed = false, miniMode = "trend", onCycleMiniMode,
                   sessions = [], days = [], checkpoints = [], projectLabel = null }) {
  return (
    <div style={{ padding: '22px 40px 16px',
                   borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center', gap: 24 }}>
      <div className="kanji"
           style={{ fontSize: 42, color: 'var(--shu)', lineHeight: 1 }}>録</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', marginBottom: 5 }}>
          {projectLabel ? `Project · ${projectLabel} · Sessions` : 'Observatory · Sessions'}
        </div>
        <h1 className="display"
            style={{ fontSize: 22, fontWeight: 400, margin: 0 }}>
          {projectLabel ? `Sessions in ${projectLabel}.` : 'The shape of your week.'}
        </h1>
        <p style={{ fontSize: 12, color: 'var(--sumi-2)', margin: '4px 0 0',
                     maxWidth: 720, lineHeight: 1.55 }}>
          {projectLabel
            ? 'A retrospective scoped to this project. Same shape as the collective view, filtered down.'
            : 'A retrospective. Then one quiet chart — drawn the way you want to read it.'}
        </p>
        <div style={{ marginTop: 12 }}>
          <RangeFilter value={range} onChange={setRange}/>
        </div>
      </div>

      {/* Right cluster — mini view + floating cycler.
            When not collapsed, shows the original numbers stats; the cycler is
            still there so users can preview different chart styles up here too. */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 0,
                      paddingLeft: 24, borderLeft: '1px solid var(--paper-edge)' }}>
        {collapsed ? (
          <MiniView mode={miniMode} totals={totals} sessions={sessions} days={days}
                     checkpoints={checkpoints}/>
        ) : (
          <MiniNumbers totals={totals}/>
        )}
        {collapsed && (
          <MiniCycler mode={miniMode} onCycle={onCycleMiniMode}/>
        )}
      </div>
    </div>
  );
}
function DotStat({ dot, n, l, mono }) {
  return (
    <div style={{ textAlign: 'center', minWidth: 56 }}>
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 18, lineHeight: 1, fontWeight: 300, color: 'var(--sumi)',
                     fontFeatureSettings: '"tnum"',
                     display: 'inline-flex', alignItems: 'center', gap: 6 }}>
        {dot && <span style={{ width: 7, height: 7, borderRadius: '50%', background: dot }}/>}
        {n}
      </div>
      <div style={{ fontSize: 9, letterSpacing: '0.12em', color: 'var(--sumi-4)',
                     marginTop: 3, textTransform: 'uppercase' }}>{l}</div>
    </div>
  );
}

// ─── Retro Lane (same as digest v1) ───────────────────────────────────
function RetroLane({ title, accent, items, positive }) {
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
        <span style={{ width: 8, height: 8, borderRadius: '50%', background: accent }}/>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                        textTransform: 'uppercase', fontWeight: 500 }}>{title}</span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {items.map(it => <RetroCard key={it.id} item={it} accent={accent} positive={positive}/>)}
      </div>
    </div>
  );
}
function RetroCard({ item, accent, positive }) {
  return (
    <article style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${accent}`, borderRadius: 6,
                       padding: '12px 14px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 6 }}>
        <span className="kanji" style={{ fontSize: 14, color: accent }}>{item.kanji}</span>
        <div style={{ fontSize: 12.5, color: 'var(--sumi)', fontWeight: 500,
                       lineHeight: 1.4, flex: 1 }}>{item.title}</div>
      </div>
      <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55 }}>{item.body}</div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 8,
                     paddingTop: 8, borderTop: '1px dashed var(--paper-edge)' }}>
        {item.delta && (
          <span className="mono" style={{ fontSize: 10.5,
                        color: positive ? 'var(--jade)' :
                               item.delta.startsWith('−') ? 'var(--shu)' :
                               item.tone === "positive" ? 'var(--jade)' : 'var(--sumi-2)' }}>
            {item.delta}
          </span>
        )}
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
          {item.evidence.length} session{item.evidence.length === 1 ? "" : "s"}
        </span>
        <span style={{ flex: 1 }}/>
        {item.action && (
          <button style={{ fontSize: 10.5, color: 'var(--shu)' }}>{item.action} →</button>
        )}
      </div>
    </article>
  );
}

// ─── Chart frame with switcher ────────────────────────────────────────
function ZenChartFrame({ chart, setChart, children }) {
  const meta = {
    trend:         { kanji: "線", label: "trend",          sub: "FTR over time · with checkpoints" },
    stream:        { kanji: "流", label: "stream",        sub: "stacked over time" },
    constellation: { kanji: "星", label: "constellation", sub: "duration vs day"   },
    bands:         { kanji: "帯", label: "bands",         sub: "stacked per day"   },
    pulse:         { kanji: "脈", label: "pulse",         sub: "one tick per session" }
  };
  const m = meta[chart];
  return (
    <section>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 14,
                     paddingBottom: 8, borderBottom: 'var(--hairline)' }}>
        <span className="kanji" style={{ fontSize: 16, color: 'var(--shu)' }}>{m.kanji}</span>
        <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0,
                      color: 'var(--sumi)' }}>Sessions · {m.label}</h3>
        <span style={{ fontSize: 11, color: 'var(--sumi-3)' }}>· {m.sub}</span>
        <span style={{ flex: 1 }}/>

        {/* Tab switch */}
        <div style={{ display: 'flex', gap: 0, padding: 2, background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 6 }}>
          {["trend","stream","constellation","bands","pulse"].map(c => (
            <button key={c} onClick={() => setChart(c)}
                    style={{ padding: '4px 11px', fontSize: 10.5,
                              background: chart === c ? 'var(--paper)' : 'transparent',
                              color: chart === c ? 'var(--sumi)' : 'var(--sumi-3)',
                              border: chart === c ? '1px solid var(--paper-edge)' : '1px solid transparent',
                              borderRadius: 4, cursor: 'pointer', letterSpacing: '0.04em' }}>
              {c}
            </button>
          ))}
        </div>
      </div>
      <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                     borderRadius: 10, padding: '24px 28px 20px' }}>
        {children}
      </div>
      <ChartLegend/>
    </section>
  );
}

function ChartLegend() {
  return (
    <div style={{ display: 'flex', gap: 18, marginTop: 12, fontSize: 10.5,
                   color: 'var(--sumi-3)', justifyContent: 'flex-end' }}>
      {Object.entries(QUALITY).map(([k, v]) => (
        <span key={k} style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
          <span style={{ width: 8, height: 8, borderRadius: 2, background: v.color }}/>
          {v.label}
        </span>
      ))}
    </div>
  );
}

// ─── Range filter pill row ─────────────────────────────────────────────────
function RangeFilter({ value, onChange }) {
  const opts = [
    { id: "7d",  label: "7 days"  },
    { id: "30d", label: "30 days" },
    { id: "90d", label: "90 days" },
    { id: "all", label: "all"     }
  ];
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <span style={{ fontSize: 10, color: 'var(--sumi-4)', letterSpacing: '0.14em',
                       textTransform: 'uppercase' }}>range</span>
      {opts.map(o => {
        const active = value === o.id;
        return (
          <button key={o.id} onClick={() => onChange(o.id)}
                  style={{ padding: '3px 10px', fontSize: 11,
                            background: active ? 'var(--sumi)' : 'transparent',
                            color: active ? 'var(--paper)' : 'var(--sumi-2)',
                            border: active
                              ? '1px solid var(--sumi)'
                              : '1px solid var(--paper-edge)',
                            borderRadius: 20, cursor: 'pointer',
                            fontFamily: 'inherit' }}>
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// TREND — daily FTR rate as a smooth line, with checkpoint markers
//        and a faint area showing problem-vs-success volume per day.
// ═══════════════════════════════════════════════════════════════════════
function TrendChart({ sessions, days = DAYS_ORDERED, checkpoints = [] }) {
  const W = 1100, H = 300, padL = 50, padR = 50, padT = 30, padB = 40;
  const innerW = W - padL - padR, innerH = H - padT - padB;

  const points = days.map(d => {
    const ofDay = sessions.filter(s => s.when === d);
    const total = ofDay.length;
    const good  = ofDay.filter(s => s.q === "good").length;
    const bad   = ofDay.filter(s => s.q === "bad").length;
    const ugly  = ofDay.filter(s => s.q === "ugly").length;
    return { day: d, total, good, bad, ugly,
             ftr: total ? good / total : null };
  });

  const x = (i) => padL + (i / Math.max(days.length - 1, 1)) * innerW;
  const yFtr = (v) => padT + innerH - v * innerH;
  const dayIndex = (label) => days.indexOf(label);

  // Smooth path through ftr points (skipping null days)
  const trendPts = points
    .map((p, i) => p.ftr === null ? null : [x(i), yFtr(p.ftr)])
    .filter(Boolean);
  const linePath = (pts) => {
    if (pts.length === 0) return "";
    let d = `M ${pts[0][0]} ${pts[0][1]}`;
    for (let i = 1; i < pts.length; i++) {
      const [px, py] = pts[i - 1], [cx, cy] = pts[i];
      const mx = (px + cx) / 2;
      d += ` C ${mx} ${py}, ${mx} ${cy}, ${cx} ${cy}`;
    }
    return d;
  };

  // Rolling-7 average for a quieter signal
  const rolling = points.map((_, i) => {
    const slice = points.slice(Math.max(0, i - 6), i + 1).filter(p => p.ftr !== null);
    if (slice.length === 0) return null;
    return slice.reduce((a, p) => a + p.ftr, 0) / slice.length;
  });
  const rollingPts = rolling
    .map((v, i) => v === null ? null : [x(i), yFtr(v)])
    .filter(Boolean);

  // Compute averages between checkpoints to draw step "baseline" segments.
  const sortedCks = checkpoints
    .map(c => ({ ...c, idx: dayIndex(c.when) }))
    .filter(c => c.idx >= 0)
    .sort((a, b) => a.idx - b.idx);
  const segments = [];
  let segStart = 0;
  sortedCks.forEach(c => {
    segments.push({ from: segStart, to: c.idx, ck: c });
    segStart = c.idx + 1;
  });
  segments.push({ from: segStart, to: days.length - 1, ck: null });
  const segAverages = segments.map(seg => {
    const slice = points.slice(seg.from, seg.to + 1).filter(p => p.ftr !== null);
    return { ...seg, avg: slice.length ? slice.reduce((a, p) => a + p.ftr, 0) / slice.length : null };
  });

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ display: 'block' }}>
      {/* Y grid · 0 / 0.5 / 1.0 */}
      {[0, 0.5, 1].map(t => (
        <g key={t}>
          <line x1={padL} x2={W - padR} y1={yFtr(t)} y2={yFtr(t)}
                stroke="var(--paper-edge)" strokeDasharray="2 4"/>
          <text x={padL - 8} y={yFtr(t) + 3} fontSize={10}
                fill="var(--sumi-4)" textAnchor="end" fontFamily="JetBrains Mono">
            {Math.round(t * 100)}%
          </text>
        </g>
      ))}
      <text x={padL - 8} y={padT - 10} fontSize={9.5} fill="var(--sumi-4)"
            textAnchor="end" fontFamily="Inter">
        ftr
      </text>

      {/* Per-segment baseline (avg between checkpoints) — the "before/after" reading */}
      {segAverages.map((seg, i) => {
        if (seg.avg === null || seg.from > seg.to) return null;
        const x1 = x(seg.from), x2 = x(seg.to), y0 = yFtr(seg.avg);
        return (
          <g key={i}>
            <line x1={x1} x2={x2} y1={y0} y2={y0}
                  stroke="var(--shu)" strokeWidth={1.5} opacity={0.55}/>
            <text x={(x1 + x2) / 2} y={y0 - 5} fontSize={10}
                  fill="var(--shu)" textAnchor="middle" fontFamily="JetBrains Mono">
              {Math.round(seg.avg * 100)}%
            </text>
          </g>
        );
      })}

      {/* Daily FTR — thin line + dots */}
      <path d={linePath(trendPts)} fill="none"
            stroke="var(--sumi-3)" strokeWidth={1} opacity={0.4}/>

      {/* Rolling-7 avg — the quiet narrative line */}
      <path d={linePath(rollingPts)} fill="none"
            stroke="var(--jade)" strokeWidth={2.2}/>

      {/* Daily dots colored by majority quality */}
      {points.map((p, i) => {
        if (p.ftr === null) return null;
        const dominant = p.ugly > 0 ? "ugly"
                       : p.bad > p.good ? "bad" : "good";
        return (
          <circle key={p.day} cx={x(i)} cy={yFtr(p.ftr)} r={3.5}
                  fill={QUALITY[dominant].color}
                  stroke="var(--paper-2)" strokeWidth={1}/>
        );
      })}

      {/* Checkpoint markers */}
      {sortedCks.map(c => {
        const cx = x(c.idx);
        return (
          <g key={c.id}>
            <line x1={cx} x2={cx} y1={padT} y2={H - padB}
                  stroke="var(--sumi)" strokeWidth={1} strokeDasharray="3 3" opacity={0.55}/>
            <circle cx={cx} cy={padT - 12} r={11} fill="var(--paper)"
                    stroke="var(--sumi)" strokeWidth={1}/>
            <text x={cx} y={padT - 8} fontSize={12} fill="var(--shu)"
                  textAnchor="middle" fontFamily="'Noto Sans JP', sans-serif">
              {c.kanji}
            </text>
          </g>
        );
      })}

      {/* X labels */}
      {points.map((p, i) => {
        const stride = days.length > 14 ? Math.ceil(days.length / 8) : 1;
        if (i % stride !== 0 && i !== days.length - 1) return null;
        return (
          <text key={p.day} x={x(i)} y={H - 10} fontSize={10}
                fill="var(--sumi-3)" textAnchor="middle" fontFamily="Inter">
            {DAYS_SHORT(p.day)}
          </text>
        );
      })}

      {/* Caption */}
      <text x={padL} y={padT - 18} fontSize={10} fill="var(--sumi-4)"
            fontFamily="Inter">
        first-try-right · daily (faint) · 7-day rolling avg (jade) · checkpoint baselines (shu)
      </text>
    </svg>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// A · STREAM — stacked area over N days · with checkpoint markers
// ═══════════════════════════════════════════════════════════════════════
function StreamChart({ sessions, days = DAYS_ORDERED, checkpoints = [] }) {
  const W = 1100, H = 280, padL = 40, padR = 40, padT = 26, padB = 36;
  const innerW = W - padL - padR, innerH = H - padT - padB;

  // For each day, aggregate minutes by quality
  const points = days.map(d => {
    const ofDay = sessions.filter(s => s.when === d);
    return {
      day: d,
      good: ofDay.filter(s => s.q === "good").reduce((a, s) => a + s.mins, 0),
      bad:  ofDay.filter(s => s.q === "bad").reduce((a, s) => a + s.mins, 0),
      ugly: ofDay.filter(s => s.q === "ugly").reduce((a, s) => a + s.mins, 0)
    };
  });

  const maxTotal = Math.max(...points.map(p => p.good + p.bad + p.ugly), 60);
  const x = (i) => padL + (i / Math.max(days.length - 1, 1)) * innerW;
  const y = (v) => padT + innerH - (v / maxTotal) * innerH;
  const dayIndex = (label) => days.indexOf(label);

  // Build paths bottom-up: ugly, then bad on top, then good on top
  const buildBand = (key, baseKeys) => {
    const top = points.map((p, i) => {
      const stacked = baseKeys.reduce((sum, k) => sum + p[k], 0) + p[key];
      return [x(i), y(stacked)];
    });
    const bot = points.map((p, i) => {
      const stacked = baseKeys.reduce((sum, k) => sum + p[k], 0);
      return [x(i), y(stacked)];
    }).reverse();
    return [...top, ...bot];
  };

  const smoothPath = (pts) => {
    if (pts.length === 0) return "";
    let d = `M ${pts[0][0]} ${pts[0][1]}`;
    for (let i = 1; i < pts.length; i++) {
      const [px, py] = pts[i - 1], [cx, cy] = pts[i];
      const mx = (px + cx) / 2;
      d += ` C ${mx} ${py}, ${mx} ${cy}, ${cx} ${cy}`;
    }
    return d + " Z";
  };

  const uglyPts = buildBand("ugly", []);
  const badPts  = buildBand("bad",  ["ugly"]);
  const goodPts = buildBand("good", ["ugly","bad"]);

  // Y-axis ticks
  const ticks = [0, Math.round(maxTotal/2), maxTotal];

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ display: 'block' }}>
      {/* Y grid */}
      {ticks.map(t => (
        <g key={t}>
          <line x1={padL} x2={W - padR} y1={y(t)} y2={y(t)}
                stroke="var(--paper-edge)" strokeDasharray="2 4"/>
          <text x={padL - 8} y={y(t) + 3} fontSize={10}
                fill="var(--sumi-4)" textAnchor="end" fontFamily="JetBrains Mono">
            {t < 60 ? `${t}m` : `${Math.floor(t/60)}h${t % 60 ? ` ${t%60}m` : ""}`}
          </text>
        </g>
      ))}

      {/* Bands */}
      <path d={smoothPath(uglyPts)} fill={QUALITY.ugly.color} opacity={0.6}/>
      <path d={smoothPath(badPts)}  fill={QUALITY.bad.color}  opacity={0.65}/>
      <path d={smoothPath(goodPts)} fill={QUALITY.good.color} opacity={0.7}/>

      {/* Crisp stroke on top of good band */}
      <path d={smoothPath(goodPts).replace(/ Z$/, "")}
            fill="none" stroke={QUALITY.good.color} strokeWidth={1.2} opacity={0.9}/>

      {/* Checkpoint markers */}
      {checkpoints.map((c, i) => {
        const idx = dayIndex(c.when);
        if (idx < 0) return null;
        const cx = x(idx);
        return (
          <g key={c.id}>
            <line x1={cx} x2={cx} y1={padT - 6} y2={H - padB}
                  stroke="var(--sumi)" strokeWidth={1} strokeDasharray="3 3" opacity={0.55}/>
            <circle cx={cx} cy={padT - 8} r={9} fill="var(--paper)"
                    stroke="var(--sumi)" strokeWidth={1}/>
            <text x={cx} y={padT - 5} fontSize={11} fill="var(--shu)"
                  textAnchor="middle" fontFamily="'Noto Sans JP', sans-serif">
              {c.kanji}
            </text>
          </g>
        );
      })}

      {/* X labels */}
      {points.map((p, i) => {
        // when range is wide, show every Nth label
        const stride = days.length > 14 ? Math.ceil(days.length / 8) : 1;
        if (i % stride !== 0 && i !== days.length - 1) return null;
        return (
          <text key={p.day} x={x(i)} y={H - 10} fontSize={10}
                fill="var(--sumi-3)" textAnchor="middle" fontFamily="Inter">
            {DAYS_SHORT(p.day)}
          </text>
        );
      })}

      {/* Today marker */}
      <line x1={x(days.length-1)} x2={x(days.length-1)}
            y1={padT} y2={H - padB}
            stroke="var(--shu)" strokeWidth={1} opacity={0.4} strokeDasharray="2 3"/>

      {/* Caption */}
      <text x={padL} y={padT + 3} fontSize={10} fill="var(--sumi-4)"
            fontFamily="Inter">
        time spent · stacked
      </text>
    </svg>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// B · CONSTELLATION — scatter, x = duration, y = day
// ═══════════════════════════════════════════════════════════════════════
function ConstellationChart({ sessions, days = DAYS_ORDERED }) {
  const W = 1100, H = 320, padL = 80, padR = 40, padT = 26, padB = 40;
  const innerW = W - padL - padR, innerH = H - padT - padB;

  const dayList = [...days].reverse();
  const maxMins = Math.max(...sessions.map(s => s.mins), 60);
  // X axis ticks
  const xTicks = [0, 30, 60, 90, 120, Math.ceil(maxMins / 30) * 30];
  const x = (m) => padL + (m / xTicks[xTicks.length - 1]) * innerW;
  const y = (i) => padT + (i / Math.max(dayList.length - 1, 1)) * innerH;

  // Trend line: average duration of good vs bad sessions
  const goodAvg = avg(sessions.filter(s => s.q === "good").map(s => s.mins));
  const badAvg  = avg(sessions.filter(s => s.q !== "good").map(s => s.mins));

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ display: 'block' }}>
      {/* Y row guides */}
      {dayList.map((d, i) => (
        <g key={d}>
          <line x1={padL} x2={W - padR} y1={y(i)} y2={y(i)}
                stroke="var(--paper-edge)" strokeDasharray="1 3"/>
          <text x={padL - 10} y={y(i) + 3} fontSize={10.5}
                fill="var(--sumi-3)" textAnchor="end" fontFamily="Inter">
            {DAYS_SHORT(d)}
          </text>
        </g>
      ))}

      {/* X ticks */}
      {xTicks.map(t => (
        <g key={t}>
          <line x1={x(t)} x2={x(t)} y1={padT - 4} y2={H - padB}
                stroke="var(--paper-edge)" strokeDasharray="1 3"/>
          <text x={x(t)} y={H - padB + 16} fontSize={10}
                fill="var(--sumi-4)" textAnchor="middle" fontFamily="JetBrains Mono">
            {t === 0 ? "0" : t < 60 ? `${t}m` : `${Math.floor(t/60)}h${t % 60 ? ` ${t%60}m` : ""}`}
          </text>
        </g>
      ))}
      <text x={(padL + W - padR) / 2} y={H - 8} fontSize={10}
            fill="var(--sumi-3)" textAnchor="middle" fontFamily="Inter">
        ← shorter session · longer session →
      </text>

      {/* Avg duration markers */}
      <line x1={x(goodAvg)} x2={x(goodAvg)} y1={padT - 2} y2={H - padB}
            stroke={QUALITY.good.color} strokeWidth={1} opacity={0.55} strokeDasharray="3 3"/>
      <line x1={x(badAvg)} x2={x(badAvg)} y1={padT - 2} y2={H - padB}
            stroke={QUALITY.bad.color} strokeWidth={1} opacity={0.55} strokeDasharray="3 3"/>
      <text x={x(goodAvg)} y={padT - 8} fontSize={9.5}
            fill={QUALITY.good.color} textAnchor="middle" fontFamily="JetBrains Mono">
        good avg
      </text>
      <text x={x(badAvg)} y={padT - 8} fontSize={9.5}
            fill={QUALITY.bad.color} textAnchor="middle" fontFamily="JetBrains Mono">
        rework avg
      </text>

      {/* Dots */}
      {sessions.map(s => {
        const dayIdx = dayList.indexOf(s.when);
        if (dayIdx < 0) return null;
        const cx = x(s.mins), cy = y(dayIdx) + (hash(s.id) % 10) - 5; // tiny jitter
        return (
          <g key={s.id}>
            <circle cx={cx} cy={cy} r={6} fill={QUALITY[s.q].color} opacity={0.85}/>
            <circle cx={cx} cy={cy} r={6} fill="none"
                    stroke="var(--paper)" strokeWidth={1.5}/>
          </g>
        );
      })}

      {/* Caption */}
      <text x={padL} y={14} fontSize={10} fill="var(--sumi-4)"
            fontFamily="Inter">
        each dot = one session · color = quality · dashed lines = avg duration by class
      </text>
    </svg>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// C · BANDS — horizontal stacked bar per day
// ═══════════════════════════════════════════════════════════════════════
function BandsChart({ sessions, days = DAYS_ORDERED }) {
  const W = 1100, H = 320, padL = 80, padR = 80, padT = 22, padB = 28;
  const innerW = W - padL - padR;
  const dayList = [...days].reverse();
  const rowH = (H - padT - padB) / dayList.length;

  const rows = dayList.map(d => {
    const ofDay = sessions.filter(s => s.when === d);
    return {
      day: d,
      good: ofDay.filter(s => s.q === "good").reduce((a, s) => a + s.mins, 0),
      bad:  ofDay.filter(s => s.q === "bad").reduce((a, s) => a + s.mins, 0),
      ugly: ofDay.filter(s => s.q === "ugly").reduce((a, s) => a + s.mins, 0),
      sessions: ofDay
    };
  });
  const maxTotal = Math.max(...rows.map(r => r.good + r.bad + r.ugly), 60);
  const w = (mins) => (mins / maxTotal) * innerW;

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ display: 'block' }}>
      {/* Caption */}
      <text x={padL} y={14} fontSize={10} fill="var(--sumi-4)" fontFamily="Inter">
        each row = one day · widths are minutes spent in each class
      </text>

      {rows.map((r, i) => {
        const cy = padT + i * rowH + rowH / 2;
        const top = cy - 9, h = 18;
        let cx = padL;
        return (
          <g key={r.day}>
            {/* Day label */}
            <text x={padL - 10} y={cy + 3} fontSize={10.5}
                  fill="var(--sumi-3)" textAnchor="end" fontFamily="Inter">
              {DAYS_SHORT(r.day)}
            </text>
            {/* Track */}
            <rect x={padL} y={top} width={innerW} height={h}
                  fill="var(--paper-3)" rx={3} opacity={0.5}/>
            {/* Segments */}
            {["good","bad","ugly"].map(k => {
              if (r[k] === 0) return null;
              const seg = (
                <rect key={k} x={cx} y={top} width={w(r[k])} height={h}
                      fill={QUALITY[k].color} opacity={0.85} rx={2}/>
              );
              cx += w(r[k]);
              return seg;
            })}
            {/* Right caption: count + total */}
            <text x={padL + innerW + 10} y={cy + 3} fontSize={10}
                  fill="var(--sumi-3)" fontFamily="JetBrains Mono">
              {r.sessions.length}× · {Math.round((r.good+r.bad+r.ugly))}m
            </text>
          </g>
        );
      })}
    </svg>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// D · PULSE — minimal · each session = a vertical tick on a 7-day axis
// ═══════════════════════════════════════════════════════════════════════
function PulseChart({ sessions, days = DAYS_ORDERED }) {
  const W = 1100, H = 240, padL = 30, padR = 30, padT = 30, padB = 50;
  const innerW = W - padL - padR, innerH = H - padT - padB;
  const maxMins = Math.max(...sessions.map(s => s.mins), 60);

  // Each session gets an x position: day index + small offset based on time-of-day
  const x = (s) => {
    const idx = days.indexOf(s.when);
    const base = padL + (idx / (days.length - 1)) * innerW;
    // mild horizontal offset by time so ticks don't pile up
    const t = parseInt((s.time || "12:00").split(":")[0], 10);
    const offset = ((t - 9) / 8) * (innerW / (days.length - 1)) * 0.4;
    return base + offset;
  };

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ display: 'block' }}>
      {/* Baseline */}
      <line x1={padL} x2={W - padR} y1={padT + innerH} y2={padT + innerH}
            stroke="var(--paper-edge)" strokeWidth={1}/>

      {/* Day labels */}
      {days.map((d, i) => (
        <text key={d}
              x={padL + (i / (days.length - 1)) * innerW}
              y={padT + innerH + 18}
              fontSize={10.5} fill="var(--sumi-3)"
              textAnchor="middle" fontFamily="Inter">
          {DAYS_SHORT(d)}
        </text>
      ))}

      {/* Hour bands (faint ref) */}
      {[30, 60, 120].map(m => (
        <line key={m}
              x1={padL} x2={W - padR}
              y1={padT + innerH - (m / maxMins) * innerH}
              y2={padT + innerH - (m / maxMins) * innerH}
              stroke="var(--paper-edge)" strokeDasharray="1 4" opacity={0.7}/>
      ))}
      {[30, 60, 120].map(m => (
        <text key={m} x={W - padR + 4}
              y={padT + innerH - (m / maxMins) * innerH + 3}
              fontSize={9} fill="var(--sumi-4)" fontFamily="JetBrains Mono">
          {m < 60 ? `${m}m` : `${m/60}h`}
        </text>
      ))}

      {/* Ticks */}
      {sessions.map(s => {
        const cx = x(s);
        const h = (s.mins / maxMins) * innerH;
        const y1 = padT + innerH;
        const y2 = y1 - h;
        return (
          <g key={s.id}>
            <line x1={cx} x2={cx} y1={y1} y2={y2}
                  stroke={QUALITY[s.q].color} strokeWidth={2.4}
                  strokeLinecap="round" opacity={0.85}/>
            <circle cx={cx} cy={y2} r={3} fill={QUALITY[s.q].color}/>
          </g>
        );
      })}

      {/* Caption */}
      <text x={padL} y={18} fontSize={10} fill="var(--sumi-4)" fontFamily="Inter">
        each tick = one session · height = duration · color = quality
      </text>
    </svg>
  );
}

// ─── tiny utils ───────────────────────────────────────────────────────
function avg(xs) { return xs.length === 0 ? 0 : xs.reduce((a,b)=>a+b,0) / xs.length; }
function median(xs) {
  if (xs.length === 0) return 0;
  const sorted = [...xs].sort((a,b)=>a-b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid-1] + sorted[mid]) / 2;
}
function hash(s) {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
  return Math.abs(h);
}

// ─── Harness — one component per default chart, for the canvas ────────
function SessionsDigestZenTrend()         { return <SessionsDigestZen initialChart="trend"/>; }
function SessionsDigestZenStream()        { return <SessionsDigestZen initialChart="stream"/>; }
function SessionsDigestZenConstellation() { return <SessionsDigestZen initialChart="constellation"/>; }
function SessionsDigestZenBands()         { return <SessionsDigestZen initialChart="bands"/>; }
function SessionsDigestZenPulse()         { return <SessionsDigestZen initialChart="pulse"/>; }

// ─── Static preview: shows the hero expanded vs collapsed (mini graph
//      lives where the 8/5/2 + 1h 4m stats normally sit). ──────────────
function SessionsDigestZenMiniHeaderPreview() {
  const D = window.SESSIONS;
  const days = daysForRange("7d");
  const dayset = new Set(days);
  const allSessions = synthesizeHistory(D.sessions);
  const enriched = allSessions
    .filter(s => dayset.has(s.when))
    .map(s => ({ ...s, mins: parseDuration(s.duration), q: quality(s) }));
  const totals = {
    count: enriched.length,
    projects: new Set(enriched.map(s => s.project)).size,
    good: enriched.filter(s => s.q === "good").length,
    bad:  enriched.filter(s => s.q === "bad").length,
    ugly: enriched.filter(s => s.q === "ugly").length,
    medianMins: median(enriched.map(s => s.mins))
  };

  const [miniMode, setMiniMode] = ss2S("trend"); // numbers | trend | stream | constellation | bands | pulse

  const Caption = ({ tag, title, body }) => (
    <div style={{ marginBottom: 10 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
                      letterSpacing: '0.14em' }}>{tag}</span>
        <span style={{ fontSize: 13, color: 'var(--sumi)', fontWeight: 500 }}>{title}</span>
      </div>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 4,
                      paddingLeft: 12, borderLeft: '2px solid var(--paper-edge)' }}>{body}</div>
    </div>
  );

  // A framed shell that mimics the artboard chrome around the hero, so
  // it reads as a screenshot of the page.
  const Frame = ({ children }) => (
    <div style={{ borderRadius: 10, border: 'var(--hairline)',
                    background: 'var(--paper)',
                    boxShadow: '0 4px 14px rgba(0,0,0,0.04)',
                    overflow: 'hidden' }}>
      {children}
    </div>
  );

  return (
    <div className="sensei" data-screen-label="Observatory · Sessions · Mini header preview"
         style={{ width: '100%', minHeight: '100%', background: 'var(--paper)',
                  display: 'flex', flexDirection: 'column' }}>

      {/* Spec hero */}
      <div style={{ padding: '22px 40px 16px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'baseline', gap: 18 }}>
        <span className="kanji" style={{ fontSize: 28, color: 'var(--shu)' }}>縮</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            Spec · header collapse
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0 }}>
            The mini graph slots into the header.
          </h1>
          <p style={{ fontSize: 12.5, color: 'var(--sumi-2)', margin: '4px 0 0',
                       maxWidth: 760, lineHeight: 1.55 }}>
            The header stays full-size on scroll — kanji, title, copy, and range pills all hold their place. Only the right cluster swaps: where 8 / 5 / 2 / 1h 4m normally sits, the mini view takes over, and a small floating icon next to it cycles between the numbers and the different chart styles.
          </p>
        </div>
      </div>

      <div style={{ flex: 1, padding: '28px 40px 40px',
                     display: 'flex', flexDirection: 'column', gap: 32 }}>

        {/* BEFORE — expanded */}
        <div>
          <Caption
            tag="BEFORE · expanded"
            title="Default header · stats on the right"
            body="At rest, the right cluster shows 3 quality dots and the median session length."/>
          <Frame>
            <ZenHero
              totals={totals} range={"7d"} setRange={() => {}}
              collapsed={false}/>
          </Frame>
        </div>

        {/* AFTER — collapsed with mode cycler */}
        <div>
          <Caption
            tag="AFTER · scrolled"
            title="Mini view slots in · floating icon cycles styles"
            body="Header keeps its full size — same kanji, title, copy, range pills. Only the right cluster swaps to the mini view, with a small floating kanji button beside it that cycles: numbers · trend · stream · constellation · bands · pulse."/>
          <Frame>
            <ZenHero
              totals={totals} range={"7d"} setRange={() => {}}
              collapsed={true}
              miniMode={miniMode}
              onCycleMiniMode={() => setMiniMode(m => nextMode(m))}
              sessions={enriched}
              days={days}
              checkpoints={D.checkpoints}/>
          </Frame>
          <div style={{ marginTop: 10, fontSize: 11, color: 'var(--sumi-3)',
                          display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 6 }}>
            <span style={{ marginRight: 4 }}>jump to a mode:</span>
            {MINI_MODES.map(m => (
              <button key={m} onClick={() => setMiniMode(m)}
                      style={{ padding: '3px 10px', fontSize: 10.5,
                                background: miniMode === m ? 'var(--paper-2)' : 'transparent',
                                border: 'var(--hairline)', borderRadius: 4,
                                cursor: 'pointer',
                                color: miniMode === m ? 'var(--sumi)' : 'var(--sumi-3)',
                                display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                <span className="kanji" style={{ fontSize: 11, color: 'var(--shu)' }}>
                  {MINI_GLYPH[m]}
                </span>
                {m}
              </button>
            ))}
          </div>
        </div>

        {/* IN CONTEXT — the collapsed hero pinned above the retrospective */}
        <div>
          <Caption
            tag="IN CONTEXT"
            title="Pinned above the retrospective"
            body="As you scroll past the chart, the hero stays at the top of the page with the mini view living in it. The retrospective lanes flow below."/>
          <Frame>
            <ZenHero
              totals={totals} range={"7d"} setRange={() => {}}
              collapsed={true}
              miniMode={miniMode}
              onCycleMiniMode={() => setMiniMode(m => nextMode(m))}
              sessions={enriched}
              days={days}
              checkpoints={D.checkpoints}/>
            <div style={{ padding: '20px 28px 24px' }}>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 14,
                              paddingBottom: 8, borderBottom: 'var(--hairline)' }}>
                <span className="kanji" style={{ fontSize: 16, color: 'var(--shu)' }}>省</span>
                <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0,
                              color: 'var(--sumi)' }}>Retrospective · last 7 days</h3>
                <span style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
                  · what sensei sees across your sessions
                </span>
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 14 }}>
                <RetroLane title="Going well"     accent="var(--jade)"   items={D.retro.going_well} positive/>
                <RetroLane title="Not going well" accent="var(--shu)"    items={D.retro.not_going}/>
                <RetroLane title="Insights"       accent="var(--sumi-2)" items={D.retro.insights}/>
              </div>
            </div>
          </Frame>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, {
  SessionsDigestZen,
  SessionsDigestZenTrend, SessionsDigestZenStream,
  SessionsDigestZenConstellation,
  SessionsDigestZenBands, SessionsDigestZenPulse,
  SessionsDigestZenMiniHeaderPreview
});
