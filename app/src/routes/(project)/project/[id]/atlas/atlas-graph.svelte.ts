// Observatory · Atlas — pure graph shaping + derivation.
//
// The daemon ships a rich code graph but no ready-to-render view. This module
// turns the four raw wire shapes into the two things the canvas draws:
//   • the collapsed community overview  (from GET /api/graph/communities/info)
//   • the symbol call graph, capped     (from GET /api/graph/nodes)
// plus the header stats (call-flow) and the solution roll-up (solution_graph).
//
// Everything here is a pure function of its inputs so it can be unit-tested
// without a daemon or a mounted component. Colours resolve to named design
// tokens (`var(--accent)` …) — communities and symbols are both coloured by
// KIND, because the communities endpoint carries no per-node membership.

import type {
  GraphSymbolNode,
  GraphCallEdge,
  CommunityInfo,
} from '$lib/types.js';

/** Render at most this many symbols — the sensei repo alone has ~6.6k nodes,
 *  far past what a force layout can lay out or a webview can paint. We keep the
 *  highest-degree symbols and surface the truncation honestly ("showing N of M"). */
export const SYMBOL_CAP = 200;

/** Above this node count the atlas auto-opens on the collapsed community
 *  overview instead of the symbol graph (spec: solution-architecture done-gate). */
export const COLLAPSE_THRESHOLD = 500;

// ── Colours — kind → named token ────────────────────────────────────────────
// Mirrors the mockup's `atlasFill`: functions read accent, types read ink,
// files/modules are muted (with a ring for contrast), docs warn.
const KIND_COLOR: Record<string, string> = {
  function: 'var(--accent)',
  method: 'var(--accent)',
  class: 'var(--ink)',
  struct: 'var(--ink)',
  enum: 'var(--ink)',
  interface: 'var(--ink)',
  type: 'var(--ink)',
  component: 'var(--success)',
  hook: 'var(--info)',
  doc: 'var(--warning)',
  const: 'var(--ink-soft)',
  extension: 'var(--accent-soft)',
  file: 'var(--paper-mute)',
  module: 'var(--paper-mute)',
};

/** The fill token for a node/community kind. Unknown kinds fall back to a
 *  muted ink so nothing renders invisibly. */
export function kindColor(kind: string): string {
  return KIND_COLOR[kind] ?? 'var(--ink-mute)';
}

/** Muted fills (files/modules) are low-contrast on paper, so the canvas draws
 *  them with an ink hairline ring. */
export function kindNeedsRing(kind: string): boolean {
  return kind === 'file' || kind === 'module';
}

// ── Communities ─────────────────────────────────────────────────────────────

export interface AtlasCommunity {
  id: string;
  /** The kind that leads the community label, e.g. "function". */
  kind: string;
  /** The directory the label points at, e.g. "app/src-tauri/src/commands". */
  path: string;
  /** The full wire label. */
  label: string;
  nodeCount: number;
}

/** Split a community label `"{kind} ({path})"` into its parts. Labels that
 *  don't match keep the whole string as the kind with an empty path. */
export function parseCommunityLabel(label: string): { kind: string; path: string } {
  const m = label.match(/^\s*(.+?)\s*\((.*)\)\s*$/);
  if (m) return { kind: m[1].trim(), path: m[2].trim() };
  return { kind: label.trim(), path: '' };
}

/** Shape the community-info wire into atlas communities, largest first. */
export function buildCommunities(raw: CommunityInfo[]): AtlasCommunity[] {
  return raw
    .map((c) => {
      const { kind, path } = parseCommunityLabel(c.label);
      return { id: c.id, kind, path, label: c.label, nodeCount: c.node_count };
    })
    .sort((a, b) => b.nodeCount - a.nodeCount);
}

// ── Symbols ─────────────────────────────────────────────────────────────────

export interface AtlasSymbol {
  id: string;
  name: string;
  kind: string;
  file: string;
  /** Internal degree — calls to/from other in-scope symbols. */
  degree: number;
}

export interface AtlasLink {
  source: string;
  target: string;
}

export interface SymbolGraph {
  symbols: AtlasSymbol[];
  links: AtlasLink[];
  /** Total symbols in scope before the cap. */
  totalSymbols: number;
  /** Total edges whose endpoints both resolve in scope (before capping). */
  totalInternalEdges: number;
}

/**
 * Reduce the raw node/edge dump to a renderable symbol graph: keep only edges
 * whose target resolves in scope (drops ~2/3 that call stdlib/out-of-scope
 * symbols), rank symbols by internal degree, keep the top `cap`, and return the
 * links among those kept. Every returned link's endpoints are in `symbols`.
 */
export function buildSymbolGraph(
  nodes: GraphSymbolNode[],
  edges: GraphCallEdge[],
  cap: number = SYMBOL_CAP,
): SymbolGraph {
  const byId = new Map(nodes.map((n) => [n.id, n]));

  const internal = edges.filter(
    (e) => e.target_id != null && byId.has(e.source_id) && byId.has(e.target_id),
  );

  const degree = new Map<string, number>();
  for (const e of internal) {
    const t = e.target_id as string;
    degree.set(e.source_id, (degree.get(e.source_id) ?? 0) + 1);
    degree.set(t, (degree.get(t) ?? 0) + 1);
  }

  const ranked = nodes
    .map((n) => ({ node: n, degree: degree.get(n.id) ?? 0 }))
    .sort((a, b) => b.degree - a.degree || a.node.name.localeCompare(b.node.name));

  const kept = ranked.slice(0, cap);
  const keptIds = new Set(kept.map((r) => r.node.id));

  const symbols: AtlasSymbol[] = kept.map((r) => ({
    id: r.node.id,
    name: r.node.name,
    kind: r.node.kind,
    file: r.node.file_path,
    degree: r.degree,
  }));

  const links: AtlasLink[] = internal
    .filter((e) => keptIds.has(e.source_id) && keptIds.has(e.target_id as string))
    .map((e) => ({ source: e.source_id, target: e.target_id as string }));

  return {
    symbols,
    links,
    totalSymbols: nodes.length,
    totalInternalEdges: internal.length,
  };
}

/** Ids adjacent to `id` in `links`, plus `id` itself. Empty set when `id` is
 *  null — the canvas then dims nothing. */
export function neighbourIds(links: AtlasLink[], id: string | null): Set<string> {
  const out = new Set<string>();
  if (!id) return out;
  out.add(id);
  for (const l of links) {
    if (l.source === id) out.add(l.target);
    if (l.target === id) out.add(l.source);
  }
  return out;
}

// ── Geometry ────────────────────────────────────────────────────────────────

/** Area-proportional radius (sqrt scale) clamped to `[min, max]`. A zero or
 *  negative `maxValue` returns `min` so nothing collapses to a point. */
export function scaleRadius(
  value: number,
  maxValue: number,
  min: number,
  max: number,
): number {
  if (maxValue <= 0) return min;
  const t = Math.sqrt(Math.max(0, value) / maxValue);
  return min + (max - min) * Math.min(1, t);
}

// ── Legend ──────────────────────────────────────────────────────────────────

export interface LegendItem {
  kind: string;
  color: string;
  ring: boolean;
}

const KIND_ORDER = [
  'function',
  'method',
  'class',
  'struct',
  'enum',
  'interface',
  'type',
  'component',
  'hook',
  'const',
  'extension',
  'file',
  'module',
  'doc',
];

/** One legend entry per kind actually present, in a stable canonical order. */
export function legendItems(kinds: Iterable<string>): LegendItem[] {
  const present = new Set(kinds);
  const ordered = [
    ...KIND_ORDER.filter((k) => present.has(k)),
    ...[...present].filter((k) => !KIND_ORDER.includes(k)).sort(),
  ];
  return ordered.map((kind) => ({
    kind,
    color: kindColor(kind),
    ring: kindNeedsRing(kind),
  }));
}

// ── Page assembly ───────────────────────────────────────────────────────────

export interface AtlasScope {
  id: string;
  name: string;
}

export interface AtlasStats {
  modules: number;
  exports: number;
  calls: number;
}

export interface AtlasSolution {
  repos: number;
  nodes: number;
  edges: number;
}

/** A renderable symbol view (the top-`cap` symbols + their internal links). The
 *  page carries two: the full graph and the production-only ("hide tests") graph,
 *  each capped INDEPENDENTLY so hiding tests pulls the next production symbols into
 *  the top-`cap` rather than just blanking the test ones. */
export interface AtlasSymbolView {
  symbols: AtlasSymbol[];
  links: AtlasLink[];
  totalSymbols: number;
  totalInternalEdges: number;
}

export interface AtlasPageData {
  repoId: string;
  scopes: AtlasScope[];
  communities: AtlasCommunity[];
  /** Sum of community node counts — the clustered population. */
  communityNodeTotal: number;
  symbols: AtlasSymbol[];
  links: AtlasLink[];
  totalSymbols: number;
  totalInternalEdges: number;
  /** Production-only view (test-file nodes excluded BEFORE the degree cap), for
   *  the Atlas "hide tests" toggle. */
  code: AtlasSymbolView;
  cap: number;
  stats: AtlasStats;
  solution: AtlasSolution;
}

export interface BuildAtlasInput {
  repoId: string;
  /** The selectable scopes — THIS project's repo roots (graph is keyed by repo
   *  name). The project window scopes to its own repos, not every project. */
  scopes: AtlasScope[];
  communities: CommunityInfo[];
  callFlow: { moduleCount: number; exportCount: number; callCount: number };
  graph: { nodes: GraphSymbolNode[]; edges: GraphCallEdge[] };
  solution: { repos: number; nodes: number; edges: number } | null;
  cap?: number;
}

/** Assemble the full page payload from the four raw responses. Pure — the
 *  loader calls this so the component never touches wire shapes. */
export function buildAtlasPage(input: BuildAtlasInput): AtlasPageData {
  const cap = input.cap ?? SYMBOL_CAP;
  const communities = buildCommunities(input.communities);
  const sym = buildSymbolGraph(input.graph.nodes, input.graph.edges, cap);
  // Production-only view: drop test-file nodes BEFORE ranking/capping so the top
  // `cap` are the highest-degree PRODUCTION symbols (not the full top-cap with
  // tests merely blanked out — which matters for test-heavy repos).
  const codeNodes = input.graph.nodes.filter((n) => !n.is_test);
  const codeSym = buildSymbolGraph(codeNodes, input.graph.edges, cap);

  return {
    repoId: input.repoId,
    scopes: [...input.scopes].sort((a, b) => a.name.localeCompare(b.name)),
    communities,
    communityNodeTotal: communities.reduce((s, c) => s + c.nodeCount, 0),
    symbols: sym.symbols,
    links: sym.links,
    totalSymbols: sym.totalSymbols,
    totalInternalEdges: sym.totalInternalEdges,
    code: {
      symbols: codeSym.symbols,
      links: codeSym.links,
      totalSymbols: codeSym.totalSymbols,
      totalInternalEdges: codeSym.totalInternalEdges,
    },
    cap,
    stats: {
      modules: input.callFlow.moduleCount,
      exports: input.callFlow.exportCount,
      calls: input.callFlow.callCount,
    },
    solution: input.solution ?? { repos: 0, nodes: 0, edges: 0 },
  };
}

// ── View-model shapes (shared by the components) ────────────────────────────

/** A node ready for the force canvas: label + geometry + resolved fill. */
export interface RenderNode {
  id: string;
  label: string;
  kind: string;
  /** Radius in viewBox units. */
  r: number;
  /** Fill token, e.g. `var(--accent)`. */
  color: string;
  /** Draw an ink hairline ring (low-contrast muted fills). */
  ring: boolean;
}

export interface InspectorCommunity {
  label: string;
  kind: string;
  path: string;
  nodeCount: number;
  /** Share of the clustered population, 0–100 (already rounded). */
  sharePct: number;
}

export interface InspectorSymbol {
  name: string;
  kind: string;
  file: string;
  degree: number;
}

export interface InspectorOverview {
  /** What "node" means in the current view: communities or symbols. */
  nodes: number;
  relations: number;
  communities: number;
  /** Rendered / total, when the symbol view is capped. */
  shown?: number;
  total?: number;
}

export type AtlasLevel = 'communities' | 'symbols';

/** The level the atlas should open on for a given scale (spec: >500 nodes
 *  auto-collapses to the community overview). */
export function initialLevel(totalSymbols: number, communityCount: number): AtlasLevel {
  if (totalSymbols > 0 && totalSymbols <= COLLAPSE_THRESHOLD) return 'symbols';
  if (communityCount > 0) return 'communities';
  return totalSymbols > 0 ? 'symbols' : 'communities';
}
