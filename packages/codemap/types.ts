// The Code map's wire shapes — what the daemon's
// /api/workspaces/{id}/diagrams/codemap* endpoints answer, and what the dōjō's
// reads must answer too. One shape, so one set of components draws both.

export type GroupBy = 'layers' | 'folders';
export type Direction = 'down' | 'skip' | 'up' | 'level';
export type Role = 'code' | 'test' | 'docs' | 'config' | 'generated';

export interface Block {
  id: string;
  label: string;
  group: string;
  files: number;
  symbols: number;
  /** A hub: its edges are counted here, not drawn. */
  hub: boolean;
  weight_in: number;
  weight_out: number;
  /** For a "+N more" block, the modules folded into it. */
  folded: string[];
}

export interface Band {
  source: string;
  target: string;
  weight: number;
  kinds: Record<string, number>;
  /** The layering's verdict; null for a band to or from a "+N more" block. */
  direction: Direction | null;
}

export interface Level0 {
  level: 'module';
  groupBy: GroupBy;
  blocks: Block[];
  bands: Band[];
  budget: { drawn: number; modules: number; bands: number; edges: number };
  cycles: unknown[];
  coverage: { unplaced: number };
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
  distance: number;
  zone: 'main' | 'pain' | 'useless' | 'risk' | null;
}

export interface Zones {
  points: ZonePoint[];
  meanDistance: number;
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
export interface CodeMapPort {
  level0(groupBy: GroupBy): Promise<Read<Level0>>;
  module(id: string): Promise<Read<Level1>>;
  file(id: string): Promise<Read<Level2>>;
  zones(): Promise<Read<Zones>>;
  cochange(): Promise<Read<CoChange>>;
}
