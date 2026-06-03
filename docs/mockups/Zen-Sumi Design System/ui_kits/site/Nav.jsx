// Site nav — thin, brand mark on the left, links on the right.

function Nav() {
  const links = [
    ["#how", "How it works"],
    ["#screens", "Screens"],
    ["#philosophy", "Philosophy"],
    ["#privacy", "Privacy"],
    ["#faq", "FAQ"]
  ];
  return (
    <nav style={{
      maxWidth: 1100, margin: '0 auto',
      padding: 'var(--space-6) var(--space-7)',
      display: 'flex', alignItems: 'center', justifyContent: 'space-between'
    }}>
      <div className="flex items-baseline gap-2">
        <span className="zs-kanji" style={{ fontSize: 20, letterSpacing: '-0.04em' }}>先生</span>
        <span style={{ fontFamily: 'var(--font-display)', fontSize: 17 }}>Sensei</span>
      </div>
      <div className="flex gap-7" style={{ fontSize: 'var(--text-xs)' }}>
        {links.map(([href, label]) => (
          <a key={href} href={href} className="text-ink-2"
             style={{ transition: 'color var(--dur-fast) var(--ease)' }}
             onMouseEnter={e => e.currentTarget.style.color = 'var(--ink)'}
             onMouseLeave={e => e.currentTarget.style.color = 'var(--ink-2)'}>
            {label}
          </a>
        ))}
      </div>
    </nav>
  );
}

// Auto-detected OS download CTA
function DownloadCTA({ size = "lg" }) {
  const [os, setOs] = React.useState("macOS");
  React.useEffect(() => {
    const ua = navigator.userAgent || "";
    if (/Win/.test(ua))         setOs("Windows");
    else if (/Linux/.test(ua))  setOs("Linux");
    else if (/Mac/.test(ua))    setOs("macOS");
  }, []);
  return (
    <a href={`#download-${os.toLowerCase()}`}
       className={`zs-btn zs-btn-primary ${size === "lg" ? "zs-btn-lg" : ""}`}
       style={{ textDecoration: 'none' }}>
      <span className="zs-kanji" style={{ color: 'var(--accent)', fontSize: 16 }}>下</span>
      Download for {os}
    </a>
  );
}

window.Nav = Nav;
window.DownloadCTA = DownloadCTA;
