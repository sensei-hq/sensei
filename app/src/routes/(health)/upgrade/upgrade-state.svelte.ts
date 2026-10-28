// The post-restart upgrade, as the Upgrade page shows it (audit 2026-10-28,
// item 13: the page held this machine itself — and marked a run that could not
// start as done with nothing failed, and faked progress in a browser).

export type StepStatus = 'pending' | 'running' | 'done' | 'failed';
export type UpgradePhase = 'running' | 'ok' | 'partial' | 'unavailable';

export const STEPS = [
  { id: 'prereqs', label: 'Prerequisite upgrades', note: 'health resolvers' },
  { id: 'db_deploy', label: 'Database schema deploy', note: 'dbd deploy' },
] as const;

export interface UpgradeEvent {
  step: string;
  status: string;
  error?: string;
}

/** The app's side of the upgrade, injected — `null` outside the Tauri app. */
export interface UpgradeDeps {
  listen: (handler: (event: UpgradeEvent) => void) => Promise<() => void>;
  invoke: () => Promise<unknown>;
  /** Clear the upgrade-pending flag and go on to Health. */
  finish: () => void;
}

/** How long a finished run stays on screen before continuing. */
const SETTLE_MS = 1200;

const DOT: Record<StepStatus, 'ok' | 'busy' | 'fail' | 'idle'> = { done: 'ok', running: 'busy', failed: 'fail', pending: 'idle' };
const STATUS_CLASS: Record<StepStatus, string> = {
  done: 'text-success',
  running: 'text-accent',
  failed: 'text-accent',
  pending: 'text-ink-mute',
};

export class UpgradeRun {
  statuses = $state<Record<string, StepStatus>>(Object.fromEntries(STEPS.map((s) => [s.id, 'pending'])));
  errors = $state<Record<string, string>>({});
  complete = $state(false);
  /** Why the run could not start or finish, when it could not. */
  error = $state<string | null>(null);
  unavailable = $state(false);

  get phase(): UpgradePhase {
    if (this.unavailable) return 'unavailable';
    if (!this.complete) return 'running';
    return this.error || Object.values(this.statuses).includes('failed') ? 'partial' : 'ok';
  }

  get rows() {
    return STEPS.map((s) => {
      const status = this.statuses[s.id] ?? 'pending';
      return {
        ...s,
        status,
        dot: DOT[status],
        statusClass: STATUS_CLASS[status],
        error: status === 'failed' ? (this.errors[s.id] ?? null) : null,
        dim: status === 'pending',
      };
    });
  }

  get progress(): string {
    const settled = this.rows.filter((r) => r.status === 'done' || r.status === 'failed').length;
    return `${(settled / STEPS.length) * 100}%`;
  }

  get title(): { lead: string; tail: string; tailClass: string } {
    switch (this.phase) {
      case 'ok':
        return { lead: 'Up to date.', tail: 'Resuming.', tailClass: 'text-success' };
      case 'partial':
        return { lead: 'Partial upgrade.', tail: 'Continuing.', tailClass: 'text-accent' };
      case 'unavailable':
        return { lead: 'Upgrade', tail: 'not available here.', tailClass: 'text-ink-mute' };
      default:
        return { lead: 'Upgrading', tail: 'sensei.', tailClass: 'text-accent' };
    }
  }

  get blurb(): string {
    switch (this.phase) {
      case 'ok':
        return 'Homebrew and the database schema are up to date. Running health checks now.';
      case 'partial':
        return 'Some steps had issues. Health checks will run — if anything is missing you can fix it there.';
      case 'unavailable':
        return 'Upgrade steps run only in the sensei app.';
      default:
        return 'Applying the update: upgrading packages via Homebrew and deploying the latest schema. No input needed.';
    }
  }

  get cardText(): string {
    switch (this.phase) {
      case 'ok':
        return 'All upgrade steps completed.';
      case 'partial':
        return this.error ? `The upgrade could not run: ${this.error}` : 'Completed with warnings. Continuing to health check.';
      case 'unavailable':
        return 'Nothing to run outside the app.';
      default: {
        const running = STEPS.find((s) => this.statuses[s.id] === 'running');
        const settled = this.rows.filter((r) => r.status === 'done' || r.status === 'failed').length;
        return `${running?.label ?? 'Preparing'}… (${Math.min(settled + 1, STEPS.length)}/${STEPS.length})`;
      }
    }
  }

  get indicator(): 'check' | 'warn' | 'spinner' | 'none' {
    return { ok: 'check', partial: 'warn', running: 'spinner', unavailable: 'none' }[this.phase] as never;
  }

  apply(event: UpgradeEvent): void {
    if (event.step === 'complete') {
      this.complete = true;
      return;
    }
    this.statuses = { ...this.statuses, [event.step]: event.status as StepStatus };
    if (event.status === 'failed' && event.error) this.errors = { ...this.errors, [event.step]: event.error };
  }

  /** Run the upgrade; on completion, settle a moment and continue. Outside the
   *  app (`deps` null) it says it cannot run — it never simulates progress. */
  async start(deps: UpgradeDeps | null): Promise<void> {
    if (!deps) {
      this.unavailable = true;
      return;
    }
    let unlisten: (() => void) | null = null;
    unlisten = await deps.listen((event) => {
      this.apply(event);
      if (this.complete) this.#settle(deps, unlisten);
    });
    try {
      await deps.invoke();
    } catch (e) {
      // A run that could not start is a failure — never "up to date".
      this.error = e instanceof Error ? e.message : String(e);
      this.complete = true;
      this.#settle(deps, unlisten);
    }
  }

  #settled = false;
  #settle(deps: UpgradeDeps, unlisten: (() => void) | null): void {
    if (this.#settled) return;
    this.#settled = true;
    setTimeout(() => {
      unlisten?.();
      deps.finish();
    }, SETTLE_MS);
  }
}
