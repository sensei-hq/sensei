// Extensions browser — Skills · Commands · Agents · Personas · Hooks · Plugins
//
// Lives at top-level "Extensions" in the Collective sidebar. Two views:
//   ▸ Collective view — every extension; install, pin per project, publish
//     local ones to the collective.
//   ▸ Project view — only what's enabled for THIS project; globals appear
//     as inherited rows; project-pinned ones get their own group.
//
// Layout: kind chips → list/detail two-pane. The detail pane shows
// metadata, scope envelope, evidence count, source, and call-to-action.

const { useState: extS, useMemo: extM } = React;

const SCOPE_META = {
  global:  { glyph: "球", label: "Global only",     color: "var(--ink-2)" },
  either:  { glyph: "両", label: "Pinnable",        color: "var(--success)"   },
  project: { glyph: "場", label: "Project only",    color: "var(--accent)"    },
};

const SOURCE_META = {
  collective: { label: "from collective", color: "var(--accent)"   },
  local:      { label: "yours",           color: "var(--success)"  },
  imported:   { label: "imported",        color: "var(--warning)" },
};

// ─── Kind chip ─────────────────────────────────────────────
function ExtKindChip({ kind, active, count, onClick }) {
  return (
    <button onClick={onClick} style={{
      display: 'inline-flex', alignItems: 'center', gap: 8,
      padding: '4px 12px', borderRadius: 5,
      border: active ? '1px solid var(--ink)' : '1px solid var(--edge)',
      background: active ? 'var(--ink)' : 'transparent',
      color: active ? 'var(--paper)' : 'var(--ink-2)',
      fontSize: 13, cursor: 'pointer', fontFamily: 'var(--font-ui)'
    }}>
      <span className="kanji" style={{ fontSize: 13,
        color: active ? 'var(--paper)' : 'var(--accent)' }}>{kind.kanji}</span>
      <span>{kind.label}</span>
      <span className="mono" style={{ fontSize: 11,
        color: active ? 'var(--paper-3)' : 'var(--ink-4)' }}>{count}</span>
    </button>
  );
}

// ─── Row in the list ───────────────────────────────────────
function ExtListRow({ ext, kind, active, onClick, projectScoped, projectId }) {
  const scope = SCOPE_META[ext.scope];
  const source = SOURCE_META[ext.source];
  const isPinnedHere = projectScoped && ext.pinnedTo.includes(projectId);
  const isInherited  = projectScoped && ext.scope === "global" && ext.installed;

  return (
    <button onClick={onClick} style={{
      display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 12,
      alignItems: 'start', textAlign: 'left', width: '100%',
      padding: '12px 16px', borderBottom: 'var(--hairline)',
      background: active ? 'var(--paper-2)' : 'transparent',
      cursor: 'pointer', borderLeft: active ? '2px solid var(--accent)' : '2px solid transparent'
    }}>
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4,
                     paddingTop: 4 }}>
        <span className="kanji" style={{ fontSize: 17, color: 'var(--accent)', lineHeight: 1 }}>
          {kind.kanji}
        </span>
        {ext.installed && (
          <span style={{ width: 5, height: 5, borderRadius: '50%',
            background: isPinnedHere ? 'var(--accent)' :
                        isInherited  ? 'var(--success)' : 'var(--ink-3)' }}/>
        )}
      </div>

      <div style={{ minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 4 }}>
          <span style={{ fontSize: 13, color: 'var(--ink)', fontWeight: 500 }}>
            {ext.name}
          </span>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
            v{ext.version}
          </span>
          {projectScoped && isPinnedHere && (
            <span style={{ fontSize: 11, color: 'var(--accent)',
                            letterSpacing: '0.12em', textTransform: 'uppercase' }}>
              pinned here
            </span>
          )}
          {projectScoped && isInherited && (
            <span style={{ fontSize: 11, color: 'var(--success)',
                            letterSpacing: '0.12em', textTransform: 'uppercase' }}>
              inherited
            </span>
          )}
        </div>
        <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.5,
                       display: '-webkit-box', WebkitLineClamp: 2,
                       WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
          {ext.desc}
        </div>
        <div style={{ display: 'flex', gap: 12, marginTop: 8,
                       fontSize: 11, color: 'var(--ink-3)' }}>
          <span>{ext.author}</span>
          <span style={{ color: source.color }}>· {source.label}</span>
          {ext.evidence != null && (
            <span className="mono">· {ext.evidence} evidence</span>
          )}
          {ext.stars && <span className="mono">· ★ {ext.stars}</span>}
        </div>
      </div>

      <div style={{ textAlign: 'right', paddingTop: 4 }}>
        {ext.installed ? (
          <span style={{ fontSize: 11, color: 'var(--success)',
                          letterSpacing: '0.12em', textTransform: 'uppercase' }}>
            installed
          </span>
        ) : (
          <span style={{ fontSize: 11, color: 'var(--ink-3)',
                          letterSpacing: '0.12em', textTransform: 'uppercase' }}>
            available
          </span>
        )}
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
          {ext.downloads}
        </div>
      </div>
    </button>
  );
}

// ─── Detail pane ───────────────────────────────────────────
function ExtDetail({ ext, kind, projectScoped, projectId, projectName }) {
  if (!ext) return null;
  const scope = SCOPE_META[ext.scope];
  const source = SOURCE_META[ext.source];
  const isPinnedHere = projectScoped && ext.pinnedTo.includes(projectId);
  const isInherited  = projectScoped && ext.scope === "global" && ext.installed;

  return (
    <div style={{ padding: '24px 32px 32px', overflow: 'auto', height: '100%' }}>
      {/* header */}
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 16,
                     paddingBottom: 24, borderBottom: 'var(--hairline)' }}>
        <div className="kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>
          {kind.kanji}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            {kind.label.replace(/s$/, '')}  ·  v{ext.version}
          </div>
          <h2 className="display" style={{ fontSize: 22, fontWeight: 400, margin: '0 0 4px',
                                            color: 'var(--ink)' }}>
            {ext.name}
          </h2>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6, margin: 0,
                       maxWidth: 640 }}>
            {ext.desc}
          </p>
          <div style={{ display: 'flex', gap: 12, marginTop: 12,
                         fontSize: 11, color: 'var(--ink-3)' }}>
            <span>by <strong style={{ color: 'var(--ink-2)', fontWeight: 500 }}>{ext.author}</strong></span>
            <span style={{ color: source.color }}>{source.label}</span>
            {ext.stars && <span className="mono">★ {ext.stars}</span>}
            {ext.downloads !== "—" && <span className="mono">{ext.downloads} installs</span>}
          </div>
        </div>
      </div>

      {/* properties grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)',
                     gap: 16, padding: '24px 0', borderBottom: 'var(--hairline)' }}>
        <ExtProp label="Scope">
          <span className="kanji" style={{ fontSize: 13, color: scope.color, marginRight: 4 }}>
            {scope.glyph}
          </span>
          <span style={{ color: scope.color, fontSize: 13 }}>{scope.label}</span>
        </ExtProp>
        <ExtProp label="Tags">
          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
            {ext.tags.map(t => (
              <span key={t} style={{ fontSize: 11, color: 'var(--ink-2)',
                background: 'var(--paper-3)', padding: '4px 8px', borderRadius: 3,
                fontFamily: 'var(--font-mono)' }}>{t}</span>
            ))}
          </div>
        </ExtProp>
        <ExtProp label={projectScoped ? "Project status" : "Pinned to"}>
          {projectScoped ? (
            isPinnedHere ? <span style={{ color: 'var(--accent)', fontSize: 13 }}>pinned to {projectName}</span> :
            isInherited  ? <span style={{ color: 'var(--success)', fontSize: 13 }}>inherited from global</span> :
                            <span style={{ color: 'var(--ink-3)', fontSize: 13 }}>not active here</span>
          ) : ext.pinnedTo.length === 0 ? (
            <span style={{ color: 'var(--ink-3)', fontSize: 13 }}>
              {ext.scope === "global" ? "global · always on" : "no projects pinned"}
            </span>
          ) : (
            <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
              {ext.pinnedTo.map(p => (
                <span key={p} style={{ fontSize: 11, color: 'var(--ink-2)',
                  background: 'var(--paper-3)', padding: '4px 8px', borderRadius: 3 }}>{p}</span>
              ))}
            </div>
          )}
        </ExtProp>
      </div>

      {/* evidence */}
      {ext.evidence != null && (
        <div style={{ padding: '16px 0', borderBottom: 'var(--hairline)' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 8 }}>
            Evidence trail
          </div>
          <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6 }}>
            <span className="display" style={{ fontSize: 22, color: 'var(--ink)',
                                                 marginRight: 8 }}>{ext.evidence}</span>
            sessions across the collective have justified this extension's use.
          </div>
        </div>
      )}

      {/* CTA */}
      <div style={{ padding: '24px 0 0', display: 'flex', gap: 8,
                     alignItems: 'center' }}>
        {ext.installed ? (
          <>
            {projectScoped && !isPinnedHere && !isInherited && (
              <button style={btnPrimary}>Pin to {projectName}</button>
            )}
            {projectScoped && isPinnedHere && (
              <button style={btnSecondary}>Unpin from {projectName}</button>
            )}
            {!projectScoped && (
              <>
                <button style={btnSecondary}>Configure</button>
                <button style={btnGhost}>Uninstall</button>
              </>
            )}
          </>
        ) : (
          <>
            <button style={btnPrimary}>Install</button>
            <button style={btnGhost}>Try in playground</button>
          </>
        )}
        <span style={{ flex: 1 }}/>
        {ext.source === "local" && !projectScoped && (
          <button style={btnGhost}>Publish to collective →</button>
        )}
      </div>
    </div>
  );
}

function ExtProp({ label, children }) {
  return (
    <div>
      <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                     textTransform: 'uppercase', marginBottom: 8 }}>
        {label}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap',
                     fontSize: 13, color: 'var(--ink-2)' }}>
        {children}
      </div>
    </div>
  );
}

const btnPrimary = {
  padding: '8px 16px', fontSize: 13, background: 'var(--ink)',
  color: 'var(--paper)', borderRadius: 5, border: 'none',
  cursor: 'pointer', fontFamily: 'var(--font-ui)'
};
const btnSecondary = {
  padding: '8px 16px', fontSize: 13, background: 'transparent',
  color: 'var(--ink)', borderRadius: 5, border: '1px solid var(--ink-3)',
  cursor: 'pointer', fontFamily: 'var(--font-ui)'
};
const btnGhost = {
  padding: '8px 12px', fontSize: 13, background: 'transparent',
  color: 'var(--ink-2)', borderRadius: 5, border: 'none',
  cursor: 'pointer', fontFamily: 'var(--font-ui)'
};

// ─── Main: Extensions browser (Collective view) ────────────
function ExtensionsBrowser({ projectScoped = false, projectId = null, projectName = null }) {
  const E = window.EXT_DATA;
  const [activeKind, setActiveKind] = extS("all");
  const [installedFilter, setInstalledFilter] = extS("all"); // all · installed · available
  const [openId, setOpenId] = extS(null);

  const filtered = extM(() => {
    let list = E.extensions;
    if (projectScoped) {
      // In project view — only globals (inherited) + project-pinned
      list = list.filter(e =>
        (e.scope === "global" && e.installed) ||
        e.pinnedTo.includes(projectId)
      );
    }
    if (activeKind !== "all") list = list.filter(e => e.kind === activeKind);
    if (installedFilter === "installed") list = list.filter(e => e.installed);
    if (installedFilter === "available") list = list.filter(e => !e.installed);
    return list;
  }, [activeKind, installedFilter, projectScoped, projectId]);

  const item = filtered.find(x => x.id === openId) || filtered[0];
  const itemKind = item ? E.kinds.find(k => k.id === item.kind) : null;

  return (
    <div className="sensei" data-screen-label={projectScoped ? "Extensions · Project" : "Extensions · Collective"}
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>

      {/* Hero */}
      <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 24 }}>
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>具</div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            {projectScoped ? `${projectName}  ·  Extensions` : "Observatory · Extensions"}
          </div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--ink)' }}>
            {projectScoped
              ? `What sensei brings to ${projectName}.`
              : "Skills · commands · agents · personas · hooks · plugins."}
          </h1>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                       maxWidth: 720, lineHeight: 1.55 }}>
            {projectScoped
              ? "Globals are always on. Project-pinned ones live in this project's toolkit. Pin or unpin to shape sensei's hands here."
              : "Six kinds of extension. Some run globally; others can be pinned per-project so sensei brings only the right tools to the bench."}
          </p>
        </div>
        <div style={{ paddingLeft: 24, borderLeft: 'var(--hairline)',
                       display: 'flex', gap: 24 }}>
          <ExtMini n={E.extensions.filter(e => e.installed).length} l="installed"/>
          <ExtMini n={E.extensions.filter(e => !e.installed).length} l="available" mono/>
          {!projectScoped && (
            <ExtMini n={E.extensions.filter(e => e.source === "local").length}
                     l="yours" mono accent/>
          )}
          {projectScoped && (
            <ExtMini n={E.extensions.filter(e => e.pinnedTo.includes(projectId)).length}
                     l="pinned here" mono accent/>
          )}
        </div>
      </div>

      {/* Filter bar */}
      <div style={{ padding: '12px 32px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
        <span style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                        textTransform: 'uppercase' }}>kind</span>
        <ExtKindChip kind={{ kanji: "全", label: "All" }}
                     active={activeKind === "all"}
                     count={filtered.length === E.extensions.length ? E.extensions.length :
                       (projectScoped ? E.extensions.filter(e =>
                         (e.scope === "global" && e.installed) || e.pinnedTo.includes(projectId)
                       ).length : E.extensions.length)}
                     onClick={() => setActiveKind("all")}/>
        {E.kinds.map(k => {
          const count = projectScoped
            ? E.extensions.filter(e => e.kind === k.id &&
                ((e.scope === "global" && e.installed) || e.pinnedTo.includes(projectId))).length
            : E.extensions.filter(e => e.kind === k.id).length;
          return (
            <ExtKindChip key={k.id} kind={k}
                         active={activeKind === k.id}
                         count={count}
                         onClick={() => setActiveKind(k.id)}/>
          );
        })}
        <span style={{ flex: 1 }}/>
        <span style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                        textTransform: 'uppercase', marginRight: 4 }}>show</span>
        {["all", "installed", "available"].map(f => (
          <button key={f} onClick={() => setInstalledFilter(f)} style={{
            padding: '4px 8px', fontSize: 11, borderRadius: 4,
            background: installedFilter === f ? 'var(--paper-3)' : 'transparent',
            color: installedFilter === f ? 'var(--ink)' : 'var(--ink-3)',
            border: 'none', cursor: 'pointer', fontFamily: 'var(--font-ui)'
          }}>{f}</button>
        ))}
      </div>

      {/* Two-pane */}
      <div style={{ flex: 1, minHeight: 0, display: 'grid',
                     gridTemplateColumns: '1fr 1.1fr' }}>
        {/* List */}
        <div style={{ borderRight: 'var(--hairline)', overflow: 'auto' }}>
          {filtered.length === 0 ? (
            <div style={{ padding: 32, textAlign: 'center', color: 'var(--ink-3)',
                           fontSize: 13 }}>
              No extensions match these filters.
            </div>
          ) : (
            filtered.map(ext => {
              const k = E.kinds.find(x => x.id === ext.kind);
              return (
                <ExtListRow key={ext.id} ext={ext} kind={k}
                  active={item && item.id === ext.id}
                  onClick={() => setOpenId(ext.id)}
                  projectScoped={projectScoped}
                  projectId={projectId}/>
              );
            })
          )}
        </div>

        {/* Detail */}
        <div style={{ overflow: 'hidden' }}>
          {item ? (
            <ExtDetail ext={item} kind={itemKind}
              projectScoped={projectScoped} projectId={projectId} projectName={projectName}/>
          ) : (
            <div style={{ padding: 32, color: 'var(--ink-3)', fontSize: 13 }}>
              Select an extension.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function ExtMini({ n, l, mono, accent }) {
  return (
    <div style={{ textAlign: 'right' }}>
      <div className={mono ? "mono" : "display"} style={{
        fontSize: mono ? 13 : 22, color: accent ? 'var(--accent)' : 'var(--ink)',
        fontWeight: 400, lineHeight: 1
      }}>{n}</div>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase', marginTop: 4 }}>{l}</div>
    </div>
  );
}

// ─── Convenience wrappers ──────────────────────────────────
function ExtensionsCollective() { return <ExtensionsBrowser/>; }
function ExtensionsProject()    {
  const E = window.EXT_DATA;
  return <ExtensionsBrowser projectScoped={true}
                            projectId={E.exampleProject.id}
                            projectName={E.exampleProject.name}/>;
}

window.ExtensionsBrowser = ExtensionsBrowser;
window.ExtensionsCollective = ExtensionsCollective;
window.ExtensionsProject = ExtensionsProject;
