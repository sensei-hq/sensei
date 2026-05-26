// Shared primitives used by all three directions.
// Exports to window so each direction file can pull them in.

const { useState, useEffect, useRef, useMemo } = React;

// ─── Sparkline ──────────────────────────────────────────────
function Sparkline({ data, width = 80, height = 22, color, fill, showDots = false, onHover }) {
  const min = Math.min(...data), max = Math.max(...data);
  const range = max - min || 1;
  const pad = 1.5;
  const step = (width - pad * 2) / (data.length - 1);
  const pts = data.map((v, i) => [
    pad + i * step,
    pad + (height - pad * 2) * (1 - (v - min) / range)
  ]);
  const d = pts.map((p, i) => (i ? "L" : "M") + p[0].toFixed(1) + " " + p[1].toFixed(1)).join(" ");
  const area = `M${pts[0][0]},${height} ${d.slice(1)} L${pts[pts.length-1][0]},${height} Z`;
  return (
    <svg width={width} height={height} style={{ color: color || 'currentColor', display: 'block', overflow: 'visible' }}>
      {fill && <path d={area} fill={fill} stroke="none" />}
      <path d={d} className="sparkline-path" />
      {showDots && pts.map(([x,y],i) => (
        <circle key={i} cx={x} cy={y} r={1.2} fill="currentColor" opacity={i === pts.length-1 ? 1 : 0.3}/>
      ))}
      {pts.length > 0 && (
        <circle cx={pts[pts.length-1][0]} cy={pts[pts.length-1][1]} r={2} fill="currentColor" />
      )}
    </svg>
  );
}

// ─── Brush-stroke circular progress (enso-style ring) ───────
// Single open arc like a sumi brushstroke. progress 0..1
function EnsoRing({ progress = 0.82, size = 120, stroke = 9, color, trackColor, startAngle = -140, sweep = 300, label }) {
  const r = (size - stroke) / 2;
  const cx = size / 2, cy = size / 2;
  const toXY = (deg) => {
    const rad = (deg * Math.PI) / 180;
    return [cx + r * Math.cos(rad), cy + r * Math.sin(rad)];
  };
  const clamp = Math.max(0.02, Math.min(0.99, progress));
  const endAngle = startAngle + sweep * clamp;
  const fullEnd = startAngle + sweep;
  const arcPath = (a0, a1) => {
    const [x0, y0] = toXY(a0);
    const [x1, y1] = toXY(a1);
    const large = a1 - a0 > 180 ? 1 : 0;
    return `M ${x0.toFixed(2)} ${y0.toFixed(2)} A ${r} ${r} 0 ${large} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
  };
  return (
    <svg width={size} height={size} style={{ display: 'block', overflow: 'visible' }}>
      <defs>
        <filter id={`ink-${size}-${Math.round(progress*100)}`}>
          <feTurbulence baseFrequency="0.9" numOctaves="2" seed="3"/>
          <feDisplacementMap in="SourceGraphic" scale="0.6"/>
        </filter>
      </defs>
      <path d={arcPath(startAngle, fullEnd)}
            stroke={trackColor || 'currentColor'}
            strokeOpacity="0.12"
            strokeWidth={stroke}
            strokeLinecap="round"
            fill="none"/>
      <path d={arcPath(startAngle, endAngle)}
            stroke={color || 'currentColor'}
            strokeWidth={stroke}
            strokeLinecap="round"
            fill="none"/>
      {/* tiny brush dot at the start */}
      <circle cx={toXY(startAngle)[0]} cy={toXY(startAngle)[1]} r={stroke * 0.62} fill={color || 'currentColor'} />
      {label != null && (
        <g>
          <text x={cx} y={cy} textAnchor="middle" dominantBaseline="central"
                fontFamily="var(--font-display)" fontSize={size * 0.32} fontWeight="400">
            {label}
          </text>
        </g>
      )}
    </svg>
  );
}

// ─── Bar mini-chart ─────────────────────────────────────────
function BarRow({ data, height = 28, width = 120, color }) {
  const max = Math.max(...data) || 1;
  const gap = 2;
  const bw = (width - gap * (data.length - 1)) / data.length;
  return (
    <svg width={width} height={height} style={{ color: color || 'currentColor', display: 'block' }}>
      {data.map((v, i) => {
        const h = Math.max(1, (v / max) * (height - 2));
        return <rect key={i} x={i * (bw + gap)} y={height - h} width={bw} height={h} fill="currentColor" opacity={0.22 + (v/max)*0.78}/>;
      })}
    </svg>
  );
}

// ─── Kanji marks (decorative, small) ────────────────────────
const KANJI = {
  studio: "工",       // craft
  cloud:  "雲",       // cloud
  brand:  "紋",       // crest
  sensei: "先生",      // teacher
  ma:     "間",       // negative space
  enso:   "円",       // circle
  shoji:  "障子",      // paper screen
  session:"刻",       // moment
  code:   "碼",
  ftr:    "一",       // one
};

// ─── Tauri chrome ───────────────────────────────────────────
function TauriChrome({ title }) {
  return (
    <div className="tauri-chrome">
      <div className="tauri-traffic"><span/><span/><span/></div>
      <div className="tauri-title">{title}</div>
      <div style={{ width: 54 }}/>
    </div>
  );
}

// ─── Avatar dot (initial) ───────────────────────────────────
function Avatar({ name, size = 24 }) {
  const letter = (name || "?").charAt(0).toUpperCase();
  return (
    <div style={{
      width: size, height: size, borderRadius: '50%',
      background: 'var(--paper-3)', color: 'var(--ink-2)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontSize: size * 0.45, fontWeight: 500, fontFamily: 'var(--font-ui)',
      border: 'var(--ink-line)'
    }}>{letter}</div>
  );
}

// ─── Format helpers ─────────────────────────────────────────
const pct = (v) => Math.round(v * 100) + "%";
const signedPct = (a, b) => {
  const d = Math.round((a - b) * 100);
  return (d >= 0 ? "+" : "") + d + "%";
};

// ─── Event icon for session timeline ────────────────────────
function EventGlyph({ kind, size = 14 }) {
  // Minimal vector glyphs, not emoji.
  const s = size;
  const c = 'currentColor';
  switch (kind) {
    case "start":      return <svg width={s} height={s} viewBox="0 0 16 16"><circle cx="8" cy="8" r="3" fill={c}/></svg>;
    case "end":        return <svg width={s} height={s} viewBox="0 0 16 16"><circle cx="8" cy="8" r="5" fill="none" stroke={c} strokeWidth="1.2"/><circle cx="8" cy="8" r="2" fill={c}/></svg>;
    case "context":    return <svg width={s} height={s} viewBox="0 0 16 16"><rect x="3" y="4" width="10" height="8" fill="none" stroke={c} strokeWidth="1.2"/><line x1="3" y1="7" x2="13" y2="7" stroke={c} strokeWidth="1.2"/></svg>;
    case "edit":       return <svg width={s} height={s} viewBox="0 0 16 16"><path d="M3 12 L10 5 L12 7 L5 14 Z" fill="none" stroke={c} strokeWidth="1.2"/></svg>;
    case "test":       return <svg width={s} height={s} viewBox="0 0 16 16"><path d="M4 10 L7 13 L13 4" fill="none" stroke={c} strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"/></svg>;
    case "correction": return <svg width={s} height={s} viewBox="0 0 16 16"><path d="M8 3 L13 13 L3 13 Z" fill="none" stroke={c} strokeWidth="1.2"/><line x1="8" y1="7" x2="8" y2="10" stroke={c} strokeWidth="1.2"/><circle cx="8" cy="11.5" r="0.6" fill={c}/></svg>;
    default:           return <svg width={s} height={s} viewBox="0 0 16 16"><circle cx="8" cy="8" r="1.5" fill={c}/></svg>;
  }
}

// ─── Zen-Sumi text + header primitives ─────────────────────
// Reusable building blocks for the recurring header / label shapes.
// One place to tweak; no more 122 inline copies of the eyebrow.

// Small uppercase label — replaces 100+ inline copies of the shape
//   fontSize:11, letterSpacing:0.18em, textTransform:uppercase, color:ink-3
function Eyebrow({ children, className = '', as: As = 'div', style }) {
  return (
    <As className={'zs-eyebrow ' + className} style={style}>
      {children}
    </As>
  );
}

// A kanji glyph with consistent sizing tokens.
// size: "xs" 11 · "sm" 13 · "base" 15 · "lg" 17 · "xl" 22 · "2xl" 28 · "3xl" 40 · "4xl" 56
const KANJI_SIZE = { xs: 11, sm: 13, base: 15, lg: 17, xl: 22, '2xl': 28, '3xl': 40, '4xl': 56 };
function Kanji({ size = 'base', color = 'var(--accent)', children, className = '', style }) {
  const px = typeof size === 'number' ? size : (KANJI_SIZE[size] || KANJI_SIZE.base);
  return (
    <span className={'kanji ' + className}
          style={{ fontSize: px, color, lineHeight: 1, flexShrink: 0, ...style }}>
      {children}
    </span>
  );
}

// The canonical kanji + eyebrow + title block.
//   variant="h1" → page header   (kanji 40, eyebrow xs, title 2xl)
//   variant="h2" → identity header (kanji 28, eyebrow xs, title lg)
//   variant="h3" → minor section header (kanji 22, eyebrow xs, title base)
// Description (if present) sits BELOW the kanji+title block, never inside the
// alignment — same rule across every screen. Right slot is an optional inline
// action that aligns with the eyebrow row.
function KanjiHeader({ variant = 'h1', kanji, eyebrow, title, description, right,
                       accent = 'var(--accent)', className = '', style }) {
  const spec = {
    h1: { k: '3xl', t: 28, w: 400, mb: 4 },
    h2: { k: '2xl', t: 17, w: 400, mb: 4 },
    h3: { k: 'xl',  t: 15, w: 400, mb: 4 },
  }[variant] || { k: '2xl', t: 17, w: 400, mb: 4 };
  return (
    <div className={(className) + ' gap-3'} style={style}>
      <div style={{ display: 'flex', alignItems: 'flex-start' }}>
        {kanji && <Kanji size={spec.k} color={accent}>{kanji}</Kanji>}
        <div style={{ flex: 1, minWidth: 0 }}>
          {eyebrow && <Eyebrow style={{ marginBottom: spec.mb }}>{eyebrow}</Eyebrow>}
          {title && (
            <h1 className="display m-0" style={{
              fontSize: spec.t, fontWeight: spec.w,
              letterSpacing: '-0.02em', lineHeight: 1, color: 'var(--ink)'
}}>{title}</h1>
          )}
        </div>
        {right && <div style={{ flexShrink: 0 }}>{right}</div>}
      </div>
      {description && (
        <p style={{
 fontSize: 13, color: 'var(--ink-3)',
                     lineHeight: 1.5, maxWidth: 540
}} className="mt-3 mb-0" >
          {description}
        </p>
      )}
    </div>
  );
}

// Small section label used inside sidebars and column heads.
// Same shape as Eyebrow but with built-in horizontal padding for the typical
// sidebar group context.
function SectionLabel({ children, className = '', style }) {
  return (
    <Eyebrow className={(className) + ' pt-0 pb-2 px-2'} style={{ display: 'block', ...style }}>
      {children}
    </Eyebrow>
  );
}

// Small colored status dot. Tone: 'accent' | 'success' | 'warning' | 'ink-3'
function StatusDot({ tone = 'accent', size = 7, className = '', style }) {
  const color = tone.startsWith('var(') ? tone :
                tone === 'success' ? 'var(--success)' :
                tone === 'warning' ? 'var(--warning)' :
                tone === 'ink-3'   ? 'var(--ink-3)'  :
                tone === 'accent'  ? 'var(--accent)' : tone;
  return (
    <span className={className}
          style={{ width: size, height: size, borderRadius: '50%',
                   background: color, display: 'inline-block', flexShrink: 0,
                   ...style }}/>
  );
}

// ─── Nav item (generic, direction-agnostic) ─────────────────
const PAGES = [
  { id: "overview",     label: "Overview",     kanji: "全" },
  { id: "observatory",  label: "Observatory",  kanji: "観" },
  { id: "sessions",     label: "Sessions",     kanji: "刻" },
  { id: "codebase",     label: "Codebase",     kanji: "構" },
  { id: "coaching",     label: "Coaching",     kanji: "師" },
  { id: "config",       label: "Configuration",kanji: "設" },
  { id: "onboarding",   label: "Setup",        kanji: "初" },
];

Object.assign(window, {
  Sparkline, EnsoRing, BarRow, TauriChrome, Avatar, EventGlyph,
  KANJI, PAGES, pct, signedPct,
  // Zen-Sumi primitives
  Eyebrow, Kanji, KanjiHeader, SectionLabel, StatusDot, KANJI_SIZE,
});
