// The Code map's state (M17): which level is open and how it was reached, the
// grouping, the roles shown, the lens and the selection. Host-agnostic — every
// read goes through the CodeMapPort the host passes in.
import type { EdgeMode } from './view';
import type { ZoneFilter } from './lenses';
import type { CoChange, CodeMapPort, Column, ComplexityTree, Focus, GroupBy, Level0, Level1, Level2, Read, Role, Zones } from './types';

export type Level = 'modules' | 'files' | 'symbols';
export type Lens = 'structure' | 'wheel' | 'cycles' | 'matrix' | 'zones' | 'coupling' | 'complexity';
export const ROLES: Role[] = ['code', 'test', 'docs', 'config', 'generated'];

export interface Crumb {
  level: Level;
  id: string | null;
  label: string;
}

export class CodeMapState {
  level = $state<Level>('modules');
  groupBy = $state<GroupBy>('layers');
  lens = $state<Lens>('structure');
  selected = $state<string | null>(null);
  /** What every lens is narrowed to (user, 2026-11-03); the same on each, until cleared. */
  focus = $state<Focus | null>(null);
  /** Generated code is off by default, so it cannot dominate sizes (M17). */
  roles = $state<Role[]>(['code', 'test', 'docs', 'config']);
  crumbs = $state<Crumb[]>([{ level: 'modules', id: null, label: 'workspace' }]);
  /** Bands drawn: all, or violations & cycles only. */
  edgeMode = $state<EdgeMode>('all');
  /** Bands lighter than this are not drawn. */
  minWeight = $state(1);
  /** Structure: tint every band by direction, not only upward. */
  byDirection = $state(false);
  /** Each lens's zoom over fitting it to the column: 1 is fit (rokkit's range).
   *  A zoom is the diagram's own — zooming one never carries to the next. */
  #zooms = $state<Partial<Record<Lens, number>>>({});
  get zoom(): number {
    return this.#zooms[this.lens] ?? 1;
  }
  set zoom(z: number) {
    this.#zooms[this.lens] = z;
  }
  /** Zones: the zone explored; the others stay drawn, dimmed. Pain and
   *  uselessness by default — the zones that need work. */
  zoneFilter = $state<ZoneFilter>('issues');
  l0 = $state<Level0 | null>(null);
  /** The latest map read with no focus: every module, for finding one to focus. */
  whole = $state<Level0 | null>(null);
  l1 = $state<Level1 | null>(null);
  l2 = $state<Level2 | null>(null);
  zones = $state<Zones | null>(null);
  cochange = $state<CoChange | null>(null);
  /** The Complexity tree, read the first time the lens opens. */
  complexity = $state<ComplexityTree | null>(null);
  /** Reads in flight. */
  #pending = $state(0);
  get loading(): boolean {
    return this.#pending > 0;
  }
  /** Why the last read failed; cleared by the next that succeeds. */
  error = $state<string | null>(null);
  /** Each kind of read's latest request: only a later read of the SAME kind
   *  overtakes one — a module opening must not drop the coupling being read. */
  #seq: Record<string, number> = {};

  /** L1's columns, without the roles turned off. */
  get columns(): Column[] {
    return (this.l1?.columns ?? []).filter((c) => this.roles.includes(c.role));
  }

  async #read<T>(kind: string, read: () => Promise<Read<T>>): Promise<T | null> {
    const mine = (this.#seq[kind] = (this.#seq[kind] ?? 0) + 1);
    this.#pending++;
    try {
      const r = await read();
      if (mine !== this.#seq[kind]) return null; // overtaken by a later read of the same kind
      this.error = r.ok ? null : r.error;
      return r.ok ? r.data : null;
    } finally {
      this.#pending--;
    }
  }

  async load(port: CodeMapPort): Promise<void> {
    const focus = this.focus;
    const data = await this.#read('level0', () => port.level0(this.groupBy, focus));
    if (!data) return;
    this.l0 = data;
    if (!focus) this.whole = data;
  }

  /** Re-band under another grouping; the level, selection, lens and a module
   *  focus stay. A segment is a group of the old grouping, so its focus goes. */
  async setGroupBy(port: CodeMapPort, groupBy: GroupBy): Promise<void> {
    this.groupBy = groupBy;
    if (this.focus?.kind === 'segment') this.focus = null;
    await this.load(port);
  }

  /** Focus every lens on a module or a segment (user, 2026-11-03): the map is
   *  re-read narrowed to it and its neighbours, from the modules level. */
  async setFocus(port: CodeMapPort, focus: Focus): Promise<void> {
    this.focus = focus;
    if (this.level !== 'modules') this.crumbTo(0);
    await this.load(port);
  }

  /** A pick from the module finder: a module, or a group header (`group:<name>`)
   *  as a segment. */
  async focusChoice(port: CodeMapPort, value: string): Promise<void> {
    const group = value.startsWith('group:') ? value.slice('group:'.length) : null;
    await this.setFocus(port, group === null ? { kind: 'module', id: value } : { kind: 'segment', id: group });
  }

  async clearFocus(port: CodeMapPort): Promise<void> {
    this.focus = null;
    await this.load(port);
  }

  /** Switch lens; Zones, Hidden coupling and Complexity read their data the
   *  first time they open. Every lens draws at the modules level, so an open
   *  module's files are left; the focus stays — it is the same on every lens. */
  async showLens(port: CodeMapPort, lens: Lens): Promise<void> {
    this.lens = lens;
    if (this.level !== 'modules') this.crumbTo(0);
    if (lens === 'zones' && !this.zones) {
      const data = await this.#read('zones', () => port.zones());
      if (data) this.zones = data;
    }
    if (lens === 'coupling' && !this.cochange) {
      const data = await this.#read('cochange', () => port.cochange());
      if (data) this.cochange = data;
    }
    if (lens === 'complexity' && !this.complexity) {
      const data = await this.#read('complexity', () => port.complexity());
      if (data) this.complexity = data;
    }
  }

  select(id: string | null): void {
    this.selected = id;
  }

  toggleRole(role: Role): void {
    this.roles = this.roles.includes(role) ? this.roles.filter((r) => r !== role) : [...this.roles, role];
  }

  async openModule(port: CodeMapPort, id: string): Promise<void> {
    const data = await this.#read('module', () => port.module(id));
    if (!data) return;
    this.l1 = data;
    this.l2 = null;
    this.level = 'files';
    this.selected = null;
    this.crumbs = [this.crumbs[0], { level: 'files', id, label: id }];
  }

  async openFile(port: CodeMapPort, id: string, path: string): Promise<void> {
    const data = await this.#read('file', () => port.file(id));
    if (!data) return;
    this.l2 = data;
    this.level = 'symbols';
    this.selected = null;
    this.crumbs = [...this.crumbs.slice(0, 2), { level: 'symbols', id, label: path }];
  }

  /** Back up the breadcrumb to `index`. */
  crumbTo(index: number): void {
    const crumb = this.crumbs[index];
    if (!crumb) return;
    this.crumbs = this.crumbs.slice(0, index + 1);
    this.level = crumb.level;
    if (crumb.level !== 'symbols') this.l2 = null;
    if (crumb.level === 'modules') this.l1 = null;
    this.selected = null;
  }
}
