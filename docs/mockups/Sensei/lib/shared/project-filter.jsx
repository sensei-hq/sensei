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
    <div className="gap-2 flex items-center flex-wrap" >
      {label && (
        <span className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{label}</span>
      )}

      <div className="gap-1 flex items-center flex-wrap" >
        <PfChip active={value === "all"} onClick={() => onChange("all")}>all</PfChip>
        {inlineKeys.map(k => (
          <PfChip key={k} active={value === k} onClick={() => onChange(k)}>
            {display(k)}
          </PfChip>
        ))}
      </div>

      <span className="flex-1" style={{ minWidth: 8 }}/>

      {/* Search input — to the right */}
      <div className="relative" ref={popRef}>
        <div style={{ borderRadius: 16,
 minWidth: 170
 }} className="gap-1 py-1 px-2 flex items-center bg-paper-2 border border-paper-edge" >
          <svg className="shrink-0" width="11" height="11" viewBox="0 0 16 16" fill="none"
 style={{ opacity: 0.55 }}>
            <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.3"/>
            <line x1="11" y1="11" x2="14" y2="14"
                  stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
          </svg>
          <input value={query}
 onChange={e => setQuery(e.target.value)}
 onFocus={() => setFocused(true)}
 placeholder="search projects…"
 style={{ fontSize: 11, fontFamily: 'inherit',
 outline: 'none' }} className="p-0 flex-1 bg-transparent border-0 text-ink min-w-0" />
          {query && (
            <button onClick={() => setQuery("")}
 style={{ fontSize: 13, lineHeight: 1,
 fontFamily: 'inherit'
 }} className="p-0 bg-transparent border-0 text-ink-4 cursor-pointer" >×</button>
          )}
        </div>

        {showPopover && (
          <div style={{ top: 'calc(100% + 4px)',
 [align === 'right' ? 'right' : 'left']: 0,
 width: 240, borderRadius: 6, zIndex: 30, maxHeight: 240 }} className="p-1 absolute bg-paper border border-paper-edge shadow overflow-auto" >
            {matches.length === 0 && (
              <div style={{
 fontSize: 11 }} className="py-2 px-2 text-ink-4 text-center" >
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
 style={{ fontSize: 11,
 background: active ? 'var(--paper-2)' : 'transparent', borderRadius: 4,
 color: active ? 'var(--ink)' : 'var(--ink-2)' }} className="py-1 px-2 gap-2 w-full text-left border-0 cursor-pointer flex items-center" >
                  {all[k]?.kanji && (
                    <span className="kanji text-accent" style={{ fontSize: 13 }}>
                      {all[k].kanji}
                    </span>
                  )}
                  <span className="flex-1" >{fullName(k)}</span>
                  {all[k]?.client && (
                    <span className="text-ink-4 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.1em' }}>
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
 borderRadius: 20,
 fontFamily: 'inherit' }} className="py-1 px-2 cursor-pointer whitespace-nowrap" >
      {children}
    </button>
  );
}

window.ProjectFilter = ProjectFilter;
