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
    <button onClick={onClick} style={{ borderRadius: 5,
 border: active ? '1px solid var(--ink)' : '1px solid var(--edge)',
 background: active ? 'var(--ink)' : 'transparent',
 color: active ? 'var(--paper)' : 'var(--ink-2)',
 fontSize: 13, fontFamily: 'var(--font-ui)'
 }} className="gap-2 py-1 px-3 inline-flex items-center cursor-pointer" >
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
    <button onClick={onClick} style={{ gridTemplateColumns: 'auto 1fr auto',
 background: active ? 'var(--paper-2)' : 'transparent', borderLeft: active ? '2px solid var(--accent)' : '2px solid transparent'
 }} className="gap-3 py-3 px-4 grid items-start text-left w-full border-b cursor-pointer" >
      <div className="gap-1 pt-1 flex flex-col items-center" >
        <span className="kanji text-accent" style={{ fontSize: 17, lineHeight: 1 }}>
          {kind.kanji}
        </span>
        {ext.installed && (
          <span className="rounded-full" style={{ width: 5, height: 5,
 background: isPinnedHere ? 'var(--accent)' :
 isInherited ? 'var(--success)' : 'var(--ink-3)' }}/>
        )}
      </div>

      <div className="min-w-0" >
        <div className="gap-2 mb-1 flex items-baseline" >
          <span className="text-ink font-medium" style={{ fontSize: 13 }}>
            {ext.name}
          </span>
          <span className="mono text-ink-4" style={{ fontSize: 11 }}>
            v{ext.version}
          </span>
          {projectScoped && isPinnedHere && (
            <span className="text-accent uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
              pinned here
            </span>
          )}
          {projectScoped && isInherited && (
            <span className="text-success uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
              inherited
            </span>
          )}
        </div>
        <div className="text-ink-2 overflow-hidden" style={{ fontSize: 13, lineHeight: 1.5,
 display: '-webkit-box', WebkitLineClamp: 2,
 WebkitBoxOrient: 'vertical' }}>
          {ext.desc}
        </div>
        <div style={{
 fontSize: 11 }} className="gap-3 mt-2 flex text-ink-3" >
          <span>{ext.author}</span>
          <span style={{ color: source.color }}>· {source.label}</span>
          {ext.evidence != null && (
            <span className="mono">· {ext.evidence} evidence</span>
          )}
          {ext.stars && <span className="mono">· ★ {ext.stars}</span>}
        </div>
      </div>

      <div className="pt-1 text-right" >
        {ext.installed ? (
          <span className="text-success uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
            installed
          </span>
        ) : (
          <span className="text-ink-3 uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
            available
          </span>
        )}
        <div className="mono mt-1 text-ink-4" style={{ fontSize: 11 }}>
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
    <div className="pt-6 pb-8 px-8 overflow-auto h-full" >
      {/* header */}
      <div className="gap-4 pb-6 flex items-start border-b" >
        <div className="kanji text-accent" style={{ fontSize: 56, lineHeight: 1 }}>
          {kind.kanji}
        </div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            {kind.label.replace(/s$/, '')}  ·  v{ext.version}
          </div>
          <h2 className="display mt-0 mb-1 font-normal text-ink" style={{
 fontSize: 22 }}>
            {ext.name}
          </h2>
          <p style={{
 fontSize: 13, lineHeight: 1.6,
 maxWidth: 640
 }} className="m-0 text-ink-2" >
            {ext.desc}
          </p>
          <div style={{
 fontSize: 11 }} className="gap-3 mt-3 flex text-ink-3" >
            <span>by <strong className="text-ink-2 font-medium" >{ext.author}</strong></span>
            <span style={{ color: source.color }}>{source.label}</span>
            {ext.stars && <span className="mono">★ {ext.stars}</span>}
            {ext.downloads !== "—" && <span className="mono">{ext.downloads} installs</span>}
          </div>
        </div>
      </div>

      {/* properties grid */}
      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-4 py-6 px-0 grid border-b" >
        <ExtProp label="Scope">
          <span className="kanji mr-1" style={{ fontSize: 13, color: scope.color }}>
            {scope.glyph}
          </span>
          <span style={{ color: scope.color, fontSize: 13 }}>{scope.label}</span>
        </ExtProp>
        <ExtProp label="Tags">
          <div className="gap-1 flex flex-wrap" >
            {ext.tags.map(t => (
              <span key={t} style={{
 fontSize: 11, borderRadius: 3,
 fontFamily: 'var(--font-mono)'
 }} className="py-1 px-2 text-ink-2 bg-paper-3" >{t}</span>
            ))}
          </div>
        </ExtProp>
        <ExtProp label={projectScoped ? "Project status" : "Pinned to"}>
          {projectScoped ? (
            isPinnedHere ? <span className="text-accent" style={{ fontSize: 13 }}>pinned to {projectName}</span> :
            isInherited  ? <span className="text-success" style={{ fontSize: 13 }}>inherited from global</span> :
                            <span className="text-ink-3" style={{ fontSize: 13 }}>not active here</span>
          ) : ext.pinnedTo.length === 0 ? (
            <span className="text-ink-3" style={{ fontSize: 13 }}>
              {ext.scope === "global" ? "global · always on" : "no projects pinned"}
            </span>
          ) : (
            <div className="gap-1 flex flex-wrap" >
              {ext.pinnedTo.map(p => (
                <span key={p} style={{
 fontSize: 11, borderRadius: 3
 }} className="py-1 px-2 text-ink-2 bg-paper-3" >{p}</span>
              ))}
            </div>
          )}
        </ExtProp>
      </div>

      {/* evidence */}
      {ext.evidence != null && (
        <div className="py-4 px-0 border-b" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 text-ink-3 uppercase" >
            Evidence trail
          </div>
          <div className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.6 }}>
            <span className="display mr-2 text-ink" style={{
 fontSize: 22 }}>{ext.evidence}</span>
            sessions across the collective have justified this extension's use.
          </div>
        </div>
      )}

      {/* CTA */}
      <div className="gap-2 pt-6 pb-0 flex items-center" >
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
        <span className="flex-1" />
        {ext.source === "local" && !projectScoped && (
          <button style={btnGhost} className="mb-2" >Publish to collective →</button>
        )}
      </div>
    </div>
  );
}

function ExtProp({ label, children }) {
  return (
    <div>
      <div className="text-ink-4 uppercase" style={{
 fontSize: 11, letterSpacing: '0.16em' }}>
        {label}
      </div>
      <div className="flex items-center flex-wrap text-ink-2" style={{
 fontSize: 13 }}>
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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label={projectScoped ? "Extensions · Project" : "Extensions · Collective"}
 >

      {/* Hero */}
      <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>具</div>
        <div className="flex-1 min-w-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            {projectScoped ? `${projectName}  ·  Extensions` : "Observatory · Extensions"}
          </div>
          <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
            {projectScoped
              ? `What sensei brings to ${projectName}.`
              : "Skills · commands · agents · personas · hooks · plugins."}
          </h1>
          <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
            {projectScoped
              ? "Globals are always on. Project-pinned ones live in this project's toolkit. Pin or unpin to shape sensei's hands here."
              : "Six kinds of extension. Some run globally; others can be pinned per-project so sensei brings only the right tools to the bench."}
          </p>
        </div>
        <div className="gap-6 pl-6 border-l flex" >
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
      <div className="py-3 px-8 gap-2 border-b flex items-center flex-wrap" >
        <span className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>kind</span>
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
        <span className="flex-1" />
        <span style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mr-1 text-ink-4 uppercase" >show</span>
        {["all", "installed", "available"].map(f => (
          <button key={f} onClick={() => setInstalledFilter(f)} style={{
 fontSize: 11, borderRadius: 4,
 background: installedFilter === f ? 'var(--paper-3)' : 'transparent',
 color: installedFilter === f ? 'var(--ink)' : 'var(--ink-3)', fontFamily: 'var(--font-ui)'
 }} className="py-1 px-2 border-0 cursor-pointer" >{f}</button>
        ))}
      </div>

      {/* Two-pane */}
      <div className="flex-1 min-h-0 grid" style={{
 gridTemplateColumns: '1fr 1.1fr' }}>
        {/* List */}
        <div className="border-r overflow-auto" >
          {filtered.length === 0 ? (
            <div style={{
 fontSize: 13
 }} className="p-8 text-center text-ink-3" >
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
        <div className="overflow-hidden" >
          {item ? (
            <ExtDetail ext={item} kind={itemKind}
              projectScoped={projectScoped} projectId={projectId} projectName={projectName}/>
          ) : (
            <div style={{ fontSize: 13 }} className="p-8 text-ink-3" >
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
    <div className="text-right" >
      <div className={mono ? "mono" : "display"} style={{
        fontSize: mono ? 13 : 22, color: accent ? 'var(--accent)' : 'var(--ink)',
        fontWeight: 400, lineHeight: 1
      }}>{n}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 text-ink-4 uppercase" >{l}</div>
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
