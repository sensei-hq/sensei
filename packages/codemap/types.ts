// The Code map's wire shapes — what the daemon's
// /api/workspaces/{id}/diagrams/codemap* endpoints answer, and what the dōjō's
// reads must answer too. One shape, so one set of components draws both.

export type GroupBy = 'layers' | 'folders';
export type Direction = 'down' | 'skip' | 'up' | 'level';
export type Role = 'code' | 'test' | 'docs' | 'config' | 'generated';
/** How a module's (or row's) complexity reads — the daemon's one rule (#156):
 *  over a fifth of its functions above cyclomatic 10 is `danger`, over a tenth
 *  `warning`; `null` when nothing was measured. */
export type Tier = 'danger' | 'warning' | 'neutral';

/** Files by role — so tests and docs never read as code size. */
export type RoleCounts = Record<Role, number>;

export interface Block {
  id: string;
  label: string;
  group: string;
  /** The package its modules share; null for a fold spanning several. */
  package: string | null;
  files: number;
  symbols: number;
  roles: RoleCounts;
  /** A hub: its edges are counted here, not drawn. */
  hub: boolean;
  weight_in: number;
  weight_out: number;
  /** For a "+N more" block, the modules folded into it. */
  folded: string[];
  /** Functions and how many exceed cyclomatic 10 (local qlty); null: not measured. */
  complexity?: { functions: number; complex: number; /** The daemon's reading of it (#156): the one rule the lens and the metrics share. */ tier: Tier | null } | null;
}

export interface Band {
  source: string;
  target: string;
  weight: number;
  kinds: Record<string, number>;
  /** The layering's verdict; null for a band to or from a "+N more" block. */
  direction: Direction | null;
}

/** What the map is focused on (user, 2026-11-03): one module, or a Group-by
 *  group of them — every lens narrows to it and its direct neighbours. */
export interface Focus {
  kind: 'module' | 'segment';
  id: string;
}

export interface Level0 {
  level: 'module';
  groupBy: GroupBy;
  /** The focus this map was narrowed to: its modules and their neighbours. */
  focus?: (Focus & { members: string[]; neighbours: string[] }) | null;
  blocks: Block[];
  bands: Band[];
  budget: { drawn: number; modules: number; bands: number; edges: number };
  cycles: unknown[];
  coverage: { unplaced: number };
  /** The workspace's referenced libraries: referenced, never members. */
  libraries: { count: number; top: string[] };
}

export interface ModuleFile {
  id: string;
  path: string;
  symbols: number;
}

export interface Column {
  role: Role;
  files: ModuleFile[];
  more: number;
  more_symbols: number;
}

export interface Neighbour {
  module: string;
  weight: number;
  direction: Direction;
}

export interface Level1 {
  level: 'file';
  module: string;
  columns: Column[];
  dependsOn: Neighbour[];
  usedBy: Neighbour[];
}

export interface SymbolRef {
  id: string;
  name: string;
  file: string | null;
  weight: number;
}

export interface Level2 {
  level: 'symbol';
  file: string;
  path: string;
  role: Role;
  symbols: { id: string; name: string; kind: string; line: number | null }[];
  callers: SymbolRef[];
  callersTotal: number;
  callees: SymbolRef[];
  calleesTotal: number;
}

/** One component on the Zones lens (Martin's metrics): instability
 *  Ce ÷ (Ca + Ce) against abstractness, its distance from the main sequence,
 *  and the zone it falls in — null when it cannot be placed. */
export interface ZonePoint {
  component: string;
  types: number;
  abstractTypes: number;
  ca: number;
  ce: number;
  abstractness: number;
  instability: number;
  /** None when the component cannot be placed. */
  distance: number | null;
  zone: 'main' | 'pain' | 'useless' | 'risk' | null;
}

export interface Zones {
  points: ZonePoint[];
  /** None when nothing is placed: a mean over no point is no reading. */
  meanDistance: number | null;
  coverage: { points: number; unplaceable: number };
}

/** Two modules that change together with no static dependency (Hidden coupling). */
export interface Coupling {
  a: string;
  b: string;
  together: number;
  /** How often the rarer of the two changes with the other (0–1). */
  strength: number;
}

export interface CoChange {
  hidden: Coupling[];
  /** Pairs that changed together often enough to count, coupled or not. */
  pairs: number;
  /** Commits read — none means no history, not no coupling. */
  commits: number;
  minTogether: number;
}

/** A read: the answer, or why there is none — never an empty stand-in. */
export type Read<T> = { ok: true; data: T } | { ok: false; error: string };

/** Where a Code map reads from: the daemon in the app, the dōjō's own API in
 *  the dōjō. Each host implements it; the components never fetch. */
/** One row of the Complexity tree (user, 2026-11-02): a module, a folder in it,
 *  or a file — each carrying what is under it. */
export interface ComplexityNode {
  id: string;
  label: string;
  kind: 'module' | 'folder' | 'file';
  functions: number;
  /** Functions over cyclomatic 10. */
  complex: number;
  /** The highest cyclomatic complexity of any function under this row. */
  maxCyclo: number;
  /** The daemon's reading of the row (#156): the one rule the lens and the metrics share. */
  tier: Tier | null;
  files: number;
  children: ComplexityNode[];
}

export interface ComplexityTree {
  /** The tier is the daemon's; a total the app sums (under a focus) has none. */
  total: { functions: number; complex: number; maxCyclo: number; files: number; tier: Tier | null };
  modules: ComplexityNode[];
  /** The workspace's latest maintainability reading, graded by the host; null
   *  where none is computed. */
  maintainability: { value: number; grade: string | null; label: string; date: string } | null;
}

export interface CodeMapPort {
  /** L0, narrowed to `focus` and its neighbours when one is given. */
  level0(groupBy: GroupBy, focus: Focus | null): Promise<Read<Level0>>;
  module(id: string): Promise<Read<Level1>>;
  file(id: string): Promise<Read<Level2>>;
  zones(): Promise<Read<Zones>>;
  cochange(): Promise<Read<CoChange>>;
  complexity(): Promise<Read<ComplexityTree>>;
}
