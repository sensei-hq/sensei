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

/** A read: the answer, or why there is none — never an empty stand-in. */
export type Read<T> = { ok: true; data: T } | { ok: false; error: string };

/** Where a Code map reads from: the daemon in the app, the dōjō's own API in
 *  the dōjō. Each host implements it; the components never fetch. */
export interface CodeMapPort {
  level0(groupBy: GroupBy): Promise<Read<Level0>>;
  module(id: string): Promise<Read<Level1>>;
  file(id: string): Promise<Read<Level2>>;
}
