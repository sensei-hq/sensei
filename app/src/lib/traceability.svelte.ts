// Doc drift — the rules every traceability screen shares (audit 2026-10-28,
// item 10: the workspace screen drew an unknown drift status as a green dot, the
// observatory drew it muted), and the workspace screen's scan action.
import { failureReason, type ApiResult } from '$lib/api.js';
import type { ChipClasses } from '$lib/dojo-artifacts.js';

/** Semantic tone a drift status maps to. `ink` = an unexpected status we still
 *  surface rather than drop (never a status colour). */
export type DriftTone = 'danger' | 'warning' | 'ink';


/** Drift status → semantic tone. `broken` (the referenced symbol no longer
 *  exists) is the harder failure and reads danger; `drifted` (signature changed)
 *  reads warning; anything else degrades to muted ink. */
export function driftTone(status: string): DriftTone {
  if (status === 'broken') return 'danger';
  if (status === 'drifted') return 'warning';
  return 'ink';
}

/** Named-token chip classes for a drift status. */
export function driftChipClasses(status: string): Pick<ChipClasses, 'bg' | 'text'> {
  switch (driftTone(status)) {
    case 'danger':
      return { bg: 'bg-danger-soft', text: 'text-danger' };
    case 'warning':
      return { bg: 'bg-warning-soft', text: 'text-warning' };
    default:
      return { bg: 'bg-paper-mute', text: 'text-ink-soft' };
  }
}

/** The StatusDot status for a drift status — the same tone, so an unknown
 *  status is neutral, never "ok". */
export function driftDotStatus(status: string): 'fail' | 'warn' | 'idle' {
  const tone = driftTone(status);
  return tone === 'danger' ? 'fail' : tone === 'warning' ? 'warn' : 'idle';
}

export interface DriftScanSummary {
  scannedDocs: number;
  newBroken: number;
  resolved: number;
}

/** The workspace screen's on-demand drift scan: whether one is running, what it
 *  found, or why it failed (#288 — a failed scan used to end with nothing shown). */
export class DriftScan {
  scanning = $state(false);
  summary = $state<DriftScanSummary | null>(null);
  error = $state<string | null>(null);

  /** The scan button's text. */
  get buttonLabel(): string {
    return this.scanning ? 'Scanning…' : 'Scan now';
  }

  /** What the last scan found, as a sentence; null before one or after a failure. */
  get summaryText(): string | null {
    const s = this.summary;
    return s ? `Scanned ${s.scannedDocs} docs · added ${s.newBroken} broken · resolved ${s.resolved}.` : null;
  }

  /** Why the last scan failed, as a sentence; null when it did not. */
  get errorText(): string | null {
    return this.error ? `The scan failed: ${this.error}` : null;
  }

  #scan: (workspaceId: string) => Promise<ApiResult<DriftScanSummary>>;
  #reload: () => Promise<void>;

  constructor(
    scan: (workspaceId: string) => Promise<ApiResult<DriftScanSummary>>,
    reload: () => Promise<void>,
  ) {
    this.#scan = scan;
    this.#reload = reload;
  }

  async run(workspaceId: string): Promise<void> {
    if (!workspaceId || this.scanning) return;
    this.scanning = true;
    this.summary = null;
    this.error = null;
    try {
      const res = await this.#scan(workspaceId);
      if (res.ok) {
        this.summary = res.data;
        await this.#reload();
      } else {
        this.error = failureReason(res.error);
      }
    } finally {
      this.scanning = false;
    }
  }
}
