// The Code map's state (M17): which level is open and how it was reached, the
// grouping, the roles shown, the lens and the selection. Host-agnostic — every
// read goes through the CodeMapPort the host passes in.
import type { CoChange, CodeMapPort, Column, GroupBy, Level0, Level1, Level2, Read, Role, Zones } from './types';

export type Level = 'modules' | 'files' | 'symbols';
export type Lens = 'structure' | 'cycles' | 'matrix' | 'zones' | 'coupling';
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
  /** Generated code is off by default, so it cannot dominate sizes (M17). */
  roles = $state<Role[]>(['code', 'test', 'docs', 'config']);
  crumbs = $state<Crumb[]>([{ level: 'modules', id: null, label: 'workspace' }]);
  l0 = $state<Level0 | null>(null);
  l1 = $state<Level1 | null>(null);
  l2 = $state<Level2 | null>(null);
  zones = $state<Zones | null>(null);
  cochange = $state<CoChange | null>(null);
  loading = $state(false);
  /** Why the last read failed; cleared by the next that succeeds. */
  error = $state<string | null>(null);
  #seq = 0;

  /** L1's columns, without the roles turned off. */
  get columns(): Column[] {
    return (this.l1?.columns ?? []).filter((c) => this.roles.includes(c.role));
  }

  async #read<T>(read: () => Promise<Read<T>>): Promise<T | null> {
    const mine = ++this.#seq;
    this.loading = true;
    const r = await read();
    if (mine !== this.#seq) return null; // overtaken by a later request
    this.loading = false;
    this.error = r.ok ? null : r.error;
    return r.ok ? r.data : null;
  }

  async load(port: CodeMapPort): Promise<void> {
    const data = await this.#read(() => port.level0(this.groupBy));
    if (data) this.l0 = data;
  }

  /** Re-band under another grouping; the level, selection and lens stay. */
  async setGroupBy(port: CodeMapPort, groupBy: GroupBy): Promise<void> {
    this.groupBy = groupBy;
    await this.load(port);
  }

  /** Switch lens; Zones and Hidden coupling read their data the first time
   *  they open. */
  async showLens(port: CodeMapPort, lens: Lens): Promise<void> {
    this.lens = lens;
    if (lens === 'zones' && !this.zones) {
      const data = await this.#read(() => port.zones());
      if (data) this.zones = data;
    }
    if (lens === 'coupling' && !this.cochange) {
      const data = await this.#read(() => port.cochange());
      if (data) this.cochange = data;
    }
  }

  select(id: string | null): void {
    this.selected = id;
  }

  toggleRole(role: Role): void {
    this.roles = this.roles.includes(role) ? this.roles.filter((r) => r !== role) : [...this.roles, role];
  }

  async openModule(port: CodeMapPort, id: string): Promise<void> {
    const data = await this.#read(() => port.module(id));
    if (!data) return;
    this.l1 = data;
    this.l2 = null;
    this.level = 'files';
    this.selected = null;
    this.crumbs = [this.crumbs[0], { level: 'files', id, label: id }];
  }

  async openFile(port: CodeMapPort, id: string, path: string): Promise<void> {
    const data = await this.#read(() => port.file(id));
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
