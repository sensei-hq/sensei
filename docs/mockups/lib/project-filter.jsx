// Shared project filter — used everywhere a screen filters by project.
//
// Layout: "all" + the N most-recent projects as inline pills on the left,
//         a search input on the right that finds anything in the list.
//
// When the search input has focus AND text, a small popover under it lists
// matching projects (by name + client). Clicking one selects it and clears
// the search. If a project not in the inline pills is the active value,
// its pill appears at the end of the inline row so the active state is
// always visible.
//
// Props:
//   value      — current project key, or "all"
//   onChange   — (key) => void
//   projects   — object keyed by id (uses window.LEARNINGS.projects by default;
//                callers can pass window.SESSIONS.projects etc.)
//   limit      — how many recents to show inline (default 5)
//   label      — eyebrow label (default "project"; pass null to hide)
//   align      — "left" | "right" — popover alignment (default "left")

const { useState: pfS, useRef: pfR, useEffect: pfE } = React;

function ProjectFilter({
  value, onChange,
  projects,
  limit = 5,
  label = "project",
  align = "left"
}) {
  const all = projects || (window.LEARNINGS && window.LEARNINGS.projects) || {};
  const keys = Object.keys(all);
  const recents = keys.slice(0, limit);

  const [query, setQuery] = pfS("");
  const [focused, setFocused] = pfS(false);
  const popRef = pfR(null);

  pfE(() => {
    if (!focused) return;
    const onDoc = (e) => {
      if (popRef.current && !popRef.current.contains(e.target)) setFocused(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [focused]);

  const display = (k) => {
    if (k === "all") return "all";
    return all[k]?.name?.replace(/-.*/, "") || all[k]?.name || k;
  };
  const fullName = (k) => all[k]?.name || k;

  // If the active value isn't in the recents row, show it as an extra pill
  // so the active state is always visible.
  const inlineKeys = (value !== "all" && !recents.includes(value))
    ? [...recents, value]
    : recents;

  const ql = query.toLowerCase().trim();
  const matches = ql
    ? keys.filter(k => fullName(k).toLowerCase().includes(ql) ||
                       (all[k]?.client || "").toLowerCase().includes(ql))
    : [];

  const showPopover = focused && ql.length > 0;

  return (
    <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap' }} className="gap-2" >
      {label && (
        <span style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                        textTransform: 'uppercase' }}>{label}</span>
      )}

      <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap' }} className="gap-1" >
        <PfChip active={value === "all"} onClick={() => onChange("all")}>all</PfChip>
        {inlineKeys.map(k => (
          <PfChip key={k} active={value === k} onClick={() => onChange(k)}>
            {display(k)}
          </PfChip>
        ))}
      </div>

      <span style={{ flex: 1, minWidth: 8 }}/>

      {/* Search input — to the right */}
      <div style={{ position: 'relative' }} ref={popRef}>
        <div style={{
 display: 'flex', alignItems: 'center',
                       background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 16,
                       minWidth: 170
}} className="gap-1 py-1 px-2" >
          <svg width="11" height="11" viewBox="0 0 16 16" fill="none"
               style={{ flexShrink: 0, opacity: 0.55 }}>
            <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.3"/>
            <line x1="11" y1="11" x2="14" y2="14"
                  stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
          </svg>
          <input value={query}
                 onChange={e => setQuery(e.target.value)}
                 onFocus={() => setFocused(true)}
                 placeholder="search projects…"
                 style={{
 flex: 1, fontSize: 11,
                           background: 'transparent', border: 'none',
                           color: 'var(--ink)', fontFamily: 'inherit',
                           outline: 'none', minWidth: 0
}} className="p-0" />
          {query && (
            <button onClick={() => setQuery("")}
                    style={{
 background: 'transparent', border: 'none',
                              color: 'var(--ink-4)', cursor: 'pointer', fontSize: 13, lineHeight: 1,
                              fontFamily: 'inherit'
}} className="p-0" >×</button>
          )}
        </div>

        {showPopover && (
          <div style={{
            position: 'absolute', top: 'calc(100% + 4px)',
            [align === 'right' ? 'right' : 'left']: 0,
            width: 240, background: 'var(--paper)',
            border: 'var(--hairline)', borderRadius: 6,
            boxShadow: '0 6px 18px rgba(0,0,0,0.06)', zIndex: 30, maxHeight: 240, overflow: 'auto'
}} className="p-1" >
            {matches.length === 0 && (
              <div style={{
 fontSize: 11,
                              color: 'var(--ink-4)', textAlign: 'center'
}} className="py-2 px-2" >
                no matches
              </div>
            )}
            {matches.map(k => {
              const active = value === k;
              return (
                <button key={k}
                        onClick={() => {
                          onChange(k); setQuery(""); setFocused(false);
                        }}
                        style={{
 width: '100%', textAlign: 'left', fontSize: 11,
                                  background: active ? 'var(--paper-2)' : 'transparent',
                                  border: 'none', borderRadius: 4,
                                  color: active ? 'var(--ink)' : 'var(--ink-2)',
                                  cursor: 'pointer',
                                  display: 'flex', alignItems: 'center'
}} className="py-1 px-2 gap-2" >
                  {all[k]?.kanji && (
                    <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>
                      {all[k].kanji}
                    </span>
                  )}
                  <span style={{ flex: 1 }}>{fullName(k)}</span>
                  {all[k]?.client && (
                    <span style={{ fontSize: 11, color: 'var(--ink-4)',
                                     letterSpacing: '0.1em',
                                     textTransform: 'uppercase' }}>
                      {all[k].client}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

function PfChip({ active, onClick, children }) {
  return (
    <button onClick={onClick}
            style={{
 fontSize: 11,
                      background: active ? 'var(--ink)' : 'transparent',
                      color: active ? 'var(--paper)' : 'var(--ink-2)',
                      border: active
                        ? '1px solid var(--ink)'
                        : '1px solid var(--edge)',
                      borderRadius: 20, cursor: 'pointer',
                      fontFamily: 'inherit', whiteSpace: 'nowrap'
}} className="py-1 px-2" >
      {children}
    </button>
  );
}

window.ProjectFilter = ProjectFilter;
