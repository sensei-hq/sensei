// ─── Brand marks ──────────────────────────────────────────────
// Monochrome SVG marks drawn with currentColor, so a single mark
// works in BOTH light and dark themes (it inherits --ink). To swap
// in an official asset later, replace the entry with:
//   claude: ({size}) => <img src="assets/logos/claude.svg" width={size} .../>
// and provide a -dark variant if the official mark isn't monochrome.
//
// These are simplified, recognizable marks — not pixel-exact trademarks.

const BrandMarks = {
  // Anthropic / Claude — radial sunburst
  claude: ({ size = 22 }) => {
    const cx = 12, cy = 12, inner = 2.3, n = 12;
    const rays = [];
    for (let i = 0; i < n; i++) {
      const ang = (Math.PI * 2 * i) / n - Math.PI / 2;
      const out = i % 2 === 0 ? 9.6 : 6.3;
      rays.push(<line key={i}
        x1={cx + Math.cos(ang) * inner} y1={cy + Math.sin(ang) * inner}
        x2={cx + Math.cos(ang) * out}   y2={cy + Math.sin(ang) * out}/>);
    }
    return (
      <svg viewBox="0 0 24 24" width={size} height={size}
           fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round">
        {rays}
      </svg>
    );
  },

  // Zed — geometric Z
  zed: ({ size = 22 }) => (
    <svg viewBox="0 0 24 24" width={size} height={size}
         fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
      <path d="M6 6 H18 L6 18 H18"/>
    </svg>
  ),

  // OpenAI — interlocking knot (simplified)
  openai: ({ size = 22 }) => (
    <svg viewBox="0 0 24 24" width={size} height={size}
         fill="none" stroke="currentColor" strokeWidth="1.6">
      <circle cx="12" cy="12" r="8"/>
      <path d="M12 4 C7 7 7 17 12 20 M12 4 C17 7 17 17 12 20" strokeWidth="1.4"/>
    </svg>
  ),

  // OpenCode — terminal window
  opencode: ({ size = 22 }) => (
    <svg viewBox="0 0 24 24" width={size} height={size}
         fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3.2" y="5" width="17.6" height="14" rx="2.4"/>
      <path d="M7 10 L9.8 12.4 L7 14.8"/>
      <line x1="12" y1="15" x2="16.5" y2="15"/>
    </svg>
  ),

  // Cursor — faceted cursor / cube
  cursor: ({ size = 22 }) => (
    <svg viewBox="0 0 24 24" width={size} height={size} stroke="currentColor" strokeLinejoin="round">
      <path d="M5 4.5 L19 12 L5 19.5 Z" fill="currentColor" fillOpacity="0.16" strokeWidth="1.5"/>
      <path d="M5 4.5 L12 12 L5 19.5" fill="none" strokeWidth="1.3" strokeOpacity="0.55"/>
    </svg>
  ),

  // JetBrains — concentric square corner
  jetbrains: ({ size = 22 }) => (
    <svg viewBox="0 0 24 24" width={size} height={size}
         fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" strokeLinecap="round">
      <rect x="4" y="4" width="16" height="16" rx="2.5"/>
      <line x1="8" y1="17" x2="14" y2="17" strokeWidth="2.2"/>
    </svg>
  ),

  // Continue — chevron
  continue: ({ size = 22 }) => (
    <svg viewBox="0 0 24 24" width={size} height={size}
         fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
      <path d="M8 6 L14 12 L8 18"/>
      <line x1="15.5" y1="18" x2="18" y2="18"/>
    </svg>
  ),

  // Generic fallback — monogram tile
  _mono: ({ size = 22, letter = "?" }) => (
    <svg viewBox="0 0 24 24" width={size} height={size}>
      <text x="12" y="16.5" textAnchor="middle" fontFamily="var(--font-display)"
            fontSize="14" fontWeight="500" fill="currentColor">{letter}</text>
    </svg>
  ),
};

function BrandMark({ id, letter, size = 22 }) {
  const M = BrandMarks[id];
  if (M) return <M size={size}/>;
  return <BrandMarks._mono size={size} letter={letter || "?"}/>;
}

window.BrandMark = BrandMark;
