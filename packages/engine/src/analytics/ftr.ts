export interface FtrSignals {
  snapshotCount: number;
  toolErrorRate: number;     // 0.0–1.0
  completedCleanly: boolean;
  hasDescription: boolean;
}

export interface FtrResult {
  score: number;             // 0.000–1.000
  signals: FtrSignals;
}

export function computeFtr(signals: FtrSignals): number {
  let score = 1.0;

  // Snapshot penalty: -0.05 per snapshot beyond the first, capped at -0.30
  const extraSnapshots = Math.max(0, signals.snapshotCount - 1);
  score -= Math.min(extraSnapshots * 0.05, 0.30);

  // Tool error rate penalty
  if (signals.toolErrorRate >= 0.20) {
    score -= 0.20;
  } else if (signals.toolErrorRate >= 0.10) {
    score -= 0.10;
  }

  // Session completion penalty
  if (!signals.completedCleanly) {
    score -= 0.30;
  }

  // No description cap (applied after all penalties)
  if (!signals.hasDescription) {
    score = Math.min(score, 0.70);
  }

  // Clamp to [0.0, 1.0] and round to 3 decimal places
  return Math.round(Math.max(0, Math.min(1, score)) * 1000) / 1000;
}
