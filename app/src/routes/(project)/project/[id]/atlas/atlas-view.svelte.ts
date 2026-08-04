// Observatory · Atlas — the screen's reactive view state.
//
// Holds only user-driven selection: which granularity level is showing and
// which node is focused. All graph shaping lives in atlas-graph.svelte.ts;
// this class is the small mutable surface the header controls and canvas bind
// to. Switching level clears the selection (a symbol id means nothing in the
// community view, and vice versa).

import type { AtlasLevel } from './atlas-graph.svelte.js';

export class AtlasView {
  level = $state<AtlasLevel>('communities');
  /** Focused node id (community id or symbol id), or null for the overview. */
  selectedId = $state<string | null>(null);

  constructor(initial: AtlasLevel = 'communities') {
    this.level = initial;
  }

  /** Switch granularity. A no-op when already there; otherwise the previous
   *  selection is dropped since ids don't carry across levels. */
  setLevel(level: AtlasLevel): void {
    if (level === this.level) return;
    this.level = level;
    this.selectedId = null;
  }

  /** Click a node: focus it, or unfocus if it was already selected. */
  select(id: string): void {
    this.selectedId = this.selectedId === id ? null : id;
  }

  clear(): void {
    this.selectedId = null;
  }
}
