import type { StreamFrame } from '../events.js';

// WaveState tracks one wave row per terminal-guarded wave (transcript,
// metric, sync): the totals frame opens the row, the terminal closes it with
// settled counts. Mid-wave settled is unknown by design — only the terminal
// names it — so the row never invents one.

export interface WaveRow {
  wave: string;
  key: string;
  total: number;
  settled: number;
  done: boolean;
}

function asCount(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null;
}

/** Percentage-style progress for a wave row; `share` is null until a total exists. */
export function waveProgress(row: WaveRow): { settled: number; total: number; share: number | null } {
  return {
    settled: row.settled,
    total: row.total,
    share: row.total > 0 ? row.settled / row.total : null,
  };
}

export class WaveState {
  private readonly waves = new Set<string>();
  items = $state<WaveRow[]>([]);

  constructor(waves: string[]) {
    for (const wave of waves) this.waves.add(wave);
  }

  /** Applies a totals or terminal frame for one of this tracker's waves. Returns false for foreign frames. */
  apply(frame: StreamFrame): boolean {
    if (!this.waves.has(frame.entity)) return false;
    if (!frame.done && frame.action !== 'add') return false;
    if (frame.done && frame.action !== 'update') return false;
    const total = asCount(frame.data.total);
    if (total === null) return false;
    const settled = frame.done ? (asCount(frame.data.settled) ?? 0) : 0;
    const row: WaveRow = { wave: frame.entity, key: frame.key, total, settled, done: frame.done === true };
    const at = this.items.findIndex((r) => r.wave === row.wave && r.key === row.key);
    if (at >= 0) this.items[at] = row;
    else this.items.push(row);
    return true;
  }
}
