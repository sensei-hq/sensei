/**
 * WizardState — singleton managing all setup wizard data.
 *
 * Hydrated from daemon via layout load. Stages read/write slices.
 * commitStage() persists to daemon on advance.
 *
 * Stage metadata + live status + active flag all live on `stages` (one
 * WizardStage[] array). The rail and the page header consume the same
 * objects; status persists via commitStage; active flips on route change.
 */

import { senseiApi, failureReason, daemonFailure, type ApiResult, type SenseiApi } from './api.js';
import { EventManager } from './events.js';
import { roleAssignments } from './setup/role-meta.js';
import { assignChainRoles } from './setup/chain-roles.svelte.js';
import { DEFAULT_PREFERENCES } from './setup/preferences.js';
import { appState } from './appstate.svelte.js';
import { hasTauri } from './bootstrap.js';
import { healthState } from './health-state.svelte.js';
import { scanState } from './scan-state.svelte.js';
import { loadWizardData } from './setup/loaders.js';
import { STORAGE_KEYS } from './storage-keys.js';
import { guessUserName } from './setup/user-name.js';
import { STAGES, type WizardStage } from '../routes/(config)/stages.js';
import type {
  DaemonAssistantFamily, DaemonWatchRoot, DaemonWorkspace,
  DaemonLibEntry, DaemonMcpEntry, PreferencesData,
  ScanBaseline, WizardLoadData,
  DaemonChain, SenseiRole,
} from './setup/contracts.js';
import type { AssistantPartEvent, AssistantPartStatus } from './types.js';

// ── Slice interfaces ────────────────────────────────────────

/** Per-family configuration progress during commit. Kept for compat with
 *  legacy callers; the live status truth now flows through `partStatus`
 *  and the per-card UI derives its header label from that. */
export type AssistantConfigureState = 'idle' | 'configuring' | 'removing' | 'failed' | 'skipped';

/** True when every installed variant of a family is currently configured. */
export function familyIsConfigured(family: DaemonAssistantFamily): boolean {
  const installed = family.variants.filter(v => v.installed);
  return installed.length > 0 && installed.every(v => v.configured);
}

export interface AssistantsSlice {
  assistants: DaemonAssistantFamily[];
  /** Live per-part status, keyed by family id then part id. The daemon
   *  emits one StateEvent per (family × part × transition) over SSE; the
   *  reducer (`applyAssistantEvent`) writes into this map. The
   *  AssistantCard reads it for chip state. */
  partStatus: Record<string, Record<string, AssistantPartStatus>>;
  /** Last-seen error message per family.part. Cleared on the next
   *  `configuring` transition so a retry resets the consolidated error. */
  partErrors: Record<string, Record<string, string>>;
}

export interface RootsSlice {
  roots: DaemonWatchRoot[];
  newPath: string;
}

export interface ScanSlice {
  baseline: ScanBaseline | null;
  started: boolean;
  done: boolean;
}

export interface WorkspacesSlice {
  workspaces: DaemonWorkspace[];
  /** Per-workspace confirmation flag — true when the user has reviewed/approved the workspace. */
  confirmed: Record<string, boolean>;
}

export interface LibrariesSlice {
  libs: DaemonLibEntry[];
}

export interface InstrumentsSlice {
  mcps: DaemonMcpEntry[];
}

export type RouterSaveState = 'idle' | 'saving' | 'done' | 'failed';

export interface RouterEntry {
  id: string;
  name: string;
  providers: string[];
  capabilities: string[];
  needsKey: boolean;
  /** `null` when the key is set but could not be read (#285). */
  configured: boolean | null;
  keyError: string | null;
  draftKey: string;          // never sent until commit
  saveState: RouterSaveState;
  saveError: string;
}

/** How a router's key reads on its card (#285). */
export function routerKeyStatus(r: RouterEntry): { label: string; tone: 'success' | 'mute' | 'warning' } {
  if (!r.needsKey) return { label: 'no key needed', tone: 'mute' };
  if (r.configured === null) {
    return { label: `key unreadable: ${r.keyError ?? 'reason not given'}`, tone: 'warning' };
  }
  return r.configured
    ? { label: 'configured ✓', tone: 'success' }
    : { label: 'not configured', tone: 'mute' };
}

/**
 * Save a provider's drafted key to the Keychain (through the daemon) and update
 * the row's status — the one key save, for the wizard's commit step and
 * Settings · Providers (audit 2026-10-28, item 13: Settings carried a copy).
 * An empty draft is not a save.
 */
export async function saveRouterKey(
  router: RouterEntry,
  api: Pick<SenseiApi, 'setGatewayRouterKey'>,
): Promise<void> {
  const key = router.draftKey.trim();
  if (key.length === 0) return;
  router.saveState = 'saving';
  router.saveError = '';
  const result = await api.setGatewayRouterKey(router.id, key);
  if (result.ok) {
    router.configured = result.data.configured;
    router.keyError = null;
    router.draftKey = '';
    router.saveState = 'done';
  } else {
    router.saveState = 'failed';
    router.saveError = result.error.message;
  }
}

export interface InferenceSlice {
  routers: RouterEntry[];
}

/** Model Assignments slice. `chains` is the full active-chain list
 *  (utility chains included so the UI can render "consensus-* is used
 *  internally" hints); `roleAssignments` snapshots the daemon's
 *  role→chainId map at hydrate time so commit can diff to a minimal
 *  set of PUTs on submit. `pendingAssignments` is the user's working
 *  edit — same shape as `roleAssignments` but keyed by user intent. */
export interface AssignmentsSlice {
  chains: DaemonChain[];
  roleAssignments: Record<SenseiRole, string | null>;
  pendingAssignments: Record<SenseiRole, string | null>;
}

// ── Commit handlers ─────────────────────────────────────────

/** Why an assistant configure/remove did not fully land — the daemon's
 *  refusal, or the errors it reported — or null when it did (#304: a refusal
 *  used to arrive as an empty result, which read as success). */
function assistantWriteFailure(res: ApiResult<{ errors: string[] }>): string | null {
  if (!res.ok) return failureReason(res.error);
  return res.data.errors.length > 0 ? res.data.errors.join('; ') : null;
}

type CommitFn = (ws: WizardState, api: ReturnType<typeof senseiApi>) => Promise<void>;

const COMMIT_HANDLERS: Record<string, CommitFn> = {
  welcome:     async () => {},
  preferences: async (ws) => {
    // Daemon write + appState cache update in one call so the next reader
    // (e.g. greeting on the welcome page) sees the new user_name without
    // a fresh appState.load().
    await appState.setConfigs({
      'setup.preferences': JSON.stringify(ws.preferences),
      'user_name': ws.preferences.displayName,
    });
  },
  assistants:  async (ws, api) => {
    // Reconcile user intent (switch state) with daemon truth (variant.configured)
    // family-by-family. The HTTP call returns when configure() has fully
    // finished server-side; the SSE stream delivers per-part chip
    // transitions in flight (configuring → done | error) so the card
    // animates while the request is open. On HTTP error or daemon-reported
    // errors[] we fall back to writing the part status directly here —
    // covers the "subscriber wasn't ready" / "daemon crashed mid-run" cases.
    //
    // Daemon endpoints take *variant* ids (claude-code, claude-desktop), so each
    // family flows through with the list of its installed variants. On success,
    // we mutate variant.configured locally to keep the UI in sync without
    // refetching — daemon is canonical on next hydrate.
    const failed: string[] = [];

    for (const family of ws.assistants.assistants) {
      const variantIds = family.variants.filter(v => v.installed).map(v => v.id);
      if (variantIds.length === 0) continue;
      const wasConfigured = familyIsConfigured(family);

      if (family.selected && !wasConfigured) {
        const failure = assistantWriteFailure(await api.tryConfigureAssistants(variantIds));
        if (failure) {
          ws.markFamilyError(family.id, failure);
          failed.push(family.id);
        } else {
          for (const v of family.variants) if (v.installed) v.configured = true;
          ws.markFamilyDone(family.id);
        }
      } else if (!family.selected && wasConfigured) {
        const failure = assistantWriteFailure(await api.tryRemoveAssistants(variantIds));
        if (failure) {
          ws.markFamilyError(family.id, failure);
          failed.push(family.id);
        } else {
          for (const v of family.variants) v.configured = false;
          ws.markFamilyIdle(family.id);
        }
      }
      // else: switch already matches daemon state, nothing to do
    }

    if (failed.length > 0) {
      throw new Error(`Failed to update: ${failed.join(', ')}`);
    }
  },
  roots:       async () => {
    // Roots are persisted to the DB at "Add" time on the roots page; this
    // stage's commit is now a no-op. Scan triggering moved to the Scan
    // stage (scanState.start) so SSE is subscribed before any work emits.
    // Without that ordering, the wave of events from scan_root would land
    // on a broadcast channel with no subscriber and the UI would show
    // "stuck at queued" for everything that finished pre-mount.
  },
  scan:        async () => {},
  workspaces:    async (ws, api) => {
    // Send only confirmed workspaces. The daemon's update_solution endpoint
    // persists name/description; per-folder role mutations go through the
    // dedicated /api/folders/{id} endpoint so a role change doesn't have
    // to round-trip through workspace metadata.
    // A save the daemon refused fails the stage, naming what was not saved;
    // the edits stay on screen for another try (#304).
    for (const p of ws.workspaces.workspaces) {
      if (ws.workspaces.confirmed[p.id] === false) continue;
      const saved = await api.tryUpdateWorkspace(p.id, { name: p.name, description: p.description });
      if (!saved.ok) throw new Error(`Workspace "${p.name}" was not saved: ${failureReason(saved.error)}`);
      for (const folder of p.folders) {
        const role = await api.tryUpdateFolder(folder.id, { role: folder.role });
        if (!role.ok) throw new Error(`The role of "${folder.name}" was not saved: ${failureReason(role.error)}`);
      }
    }
  },
  libraries:   async (ws) => {
    // Persist the wrapped/disabled split to `setup.libraries`. The daemon
    // reads this when deciding which libs to index/wrap. Both lists are
    // stored explicitly so that adding a new lib after this commit defaults
    // to enabled (it's neither on `wrapped` nor `disabled` yet — see the
    // loader's mapLibraries).
    const wrapped: string[] = [];
    const disabled: string[] = [];
    for (const lib of ws.libraries.libs) {
      (lib.enabled ? wrapped : disabled).push(lib.name);
    }
    await appState.setConfigs({ 'setup.libraries': JSON.stringify({ wrapped, disabled }) });
  },
  instruments: async (ws) => {
    // Persist the user's MCP selection to `setup.instruments`. Once a daemon
    // registry endpoint lands, the same key can drive install/uninstall.
    const selected = ws.instruments.mcps.filter(m => m.selected).map(m => m.id);
    const deselected = ws.instruments.mcps.filter(m => !m.selected).map(m => m.id);
    await appState.setConfigs({ 'setup.instruments': JSON.stringify({ selected, deselected }) });
  },
  inference: async (ws, api) => {
    // Persist any non-empty drafted keys. Per-card status updates so
    // the UI shows progress. Failures are non-fatal — user can retry
    // later from settings — but we surface them via saveError.
    for (const router of ws.inference.routers) {
      if (router.needsKey) await saveRouterKey(router, api);
    }
  },
  assignments: async (ws, api) => {
    // Diff pending vs. daemon truth so only the roles that changed are written.
    const roles: SenseiRole[] = ['inference', 'consolidation', 'embedding', 'voice'];
    const current = ws.assignments.roleAssignments;
    const pending = ws.assignments.pendingAssignments;

    const changes = roles
      .filter(r => (pending[r] ?? null) !== (current[r] ?? null))
      .map(r => ({ role: r, next: pending[r] ?? null, prev: current[r] ?? null }));

    // The one role write (clear old holders, then set new ones). A refusal
    // fails the commit (#288) — it used to be logged and the stage marked done.
    const refused = await assignChainRoles(api, changes);
    // Refresh state so `roleAssignments` reflects the daemon's truth — after a
    // refusal too, so the screen shows what actually stuck.
    await ws.refreshAssignments();
    if (refused.length) throw new Error(`The daemon refused: ${refused.join('; ')}`);
  },
  done:        async (ws) => { await ws.setCompleted(); },
};

// ── WizardState ─────────────────────────────────────────────

function cloneStages(): WizardStage[] {
  return STAGES.map(s => ({ ...s }));
}

/** Seed for `setupComplete` $state — reads localStorage[`setupComplete`]
 *  as the cold-start fallback so the boot gate gets a stable answer
 *  before any async load(). Updated by load() (from daemon truth) and
 *  setCompleted() (when the user finishes the wizard). */
function initialSetupComplete(): boolean {
  // SvelteKit static prerender (adapter-static) shims `localStorage` as
  // an object whose methods are absent — so `typeof localStorage` is
  // 'object' but `localStorage.getItem` is undefined. Probe getItem
  // directly instead of trusting the global presence.
  const ls = (globalThis as { localStorage?: Storage }).localStorage;
  if (!ls || typeof ls.getItem !== 'function') return false;
  try { return ls.getItem(STORAGE_KEYS.setupComplete) === '1'; }
  catch (e) {
    console.warn('[wizardState] localStorage read failed at cold start; defaulting to false', e);
    return false;
  }
}

/** A family as the card and the setup summary render it — see `assistantView`. */
export interface AssistantView {
  id: string;
  name: string;
  /** Some variant is installed. */
  found: boolean;
  /** The switch. */
  enabled: boolean;
  parts: { id: string; label: string; status: AssistantPartStatus }[];
  error: string | null;
  busy: boolean;
  showError: boolean;
  state: 'idle' | 'configuring' | 'failed' | 'done';
  configured: boolean;
  header: { text: string; tone: string; icon: 'spinner' | 'check' | 'cross' | null };
}

/** Pure: one family's view from its live part statuses and errors — what
 *  `WizardState.assistantView` returns, testable without the store. */
export function assistantViewOf(
  family: DaemonAssistantFamily,
  partStatus: Record<string, AssistantPartStatus> = {},
  partErrors: Record<string, string | undefined> = {},
): AssistantView {
  const parts = family.parts.map((p) => ({
    id: p.id,
    label: p.label,
    status: (partStatus[p.id] ?? 'idle') as AssistantPartStatus
  }));
  const found = family.variants.some((v) => v.installed);
  const enabled = family.selected;
  const statuses = parts.map((p) => p.status);
  const configuring = statuses.includes('configuring');
  const failed = statuses.includes('error');
  const allDone = parts.length > 0 && statuses.every((st) => st === 'done');
  const live = statuses.some((st) => st !== 'idle');
  const configured = enabled && (live ? allDone : familyIsConfigured(family));
  const errs = Object.values(partErrors).filter(Boolean) as string[];
  const error = errs.length ? Array.from(new Set(errs)).join('; ') : null;
  const state: AssistantView['state'] = configuring
    ? 'configuring'
    : !enabled
      ? 'idle'
      : failed
        ? 'failed'
        : configured
          ? 'done'
          : 'idle';
  const header: AssistantView['header'] = !found
    ? { text: 'not found', tone: 'muted-italic', icon: null }
    : state === 'configuring'
      ? { text: 'configuring…', tone: 'accent', icon: 'spinner' }
      : !enabled
        ? { text: 'off', tone: 'muted', icon: null }
        : state === 'failed'
          ? { text: 'failed', tone: 'danger', icon: 'cross' }
          : state === 'done'
            ? { text: 'configured', tone: 'success', icon: 'check' }
            : { text: '', tone: 'muted', icon: null };
  return {
    id: family.id,
    name: family.name,
    found,
    enabled,
    parts,
    error,
    busy: configuring,
    showError: enabled && !!error && !configuring,
    state,
    configured,
    header
  };
}

export class WizardState {
  /**
   * Canonical "is the wizard finished?" flag. Owned here (rather than on
   * appState) because completion is a wizard concern. appState surfaces
   * this through its `setupComplete` / `setupOk` facade.
   *
   * Lifecycle:
   *   - Initial value seeded from localStorage so the boot gate has a stable
   *     answer on cold start.
   *   - `load()` reconciles against daemon's `config['setup_complete']`
   *     on every invocation, and syncs localStorage to whatever the
   *     daemon reports. The module-level $effect.root below re-fires
   *     load() whenever healthState flips to ok (typically right after
   *     a resolve pass that may have recreated the DB), so a dropped DB
   *     can't leave the cache stuck at '1'.
   *   - `setCompleted()` flips it to true on the final wizard stage and
   *     writes '1' to localStorage.
   */
  setupComplete = $state<boolean>(initialSetupComplete());

  // Single source of truth for stage metadata + persisted status + transient active.
  // The rail iterates this array; the page header indexes it.
  stages = $state<WizardStage[]>(cloneStages());

  constructor() {
    // Reactive bridge from healthState. When health flips to ok (typically
    // right after a resolve pass that may have recreated the DB and its
    // config table), re-run load() so `setupComplete` + the localStorage
    // cache reconcile against fresh daemon truth — a dropped DB can't
    // leave the cache stuck at '1'.
    //
    // `$effect` is the right primitive here, NOT `$derived`: $derived is
    // synchronous and would store a Promise instead of the boolean.
    // The effect only re-fires when `healthState.isOk` actually changes,
    // so steady-state-ok doesn't re-trigger load.
    //
    // $effect.root creates a non-component tracking scope so this works
    // for a module-level singleton.
    $effect.root(() => {
      $effect(() => {
        // load() catches its own errors and returns false on failure —
        // it can't reject, so no .catch() is needed here. `void` makes
        // the fire-and-forget intent explicit (and satisfies the rule
        // against returning a Promise from an $effect callback, which
        // Svelte would mistake for a cleanup function).
        if (healthState.isOk) void this.load();
      });
    });
  }

  preferences = $state<PreferencesData>({ ...DEFAULT_PREFERENCES });
  assistants  = $state<AssistantsSlice>({ assistants: [], partStatus: {}, partErrors: {} });
  roots       = $state<RootsSlice>({ roots: [], newPath: '' });
  scan        = $state<ScanSlice>({ baseline: null, started: false, done: false });
  workspaces    = $state<WorkspacesSlice>({ workspaces: [], confirmed: {} });
  libraries   = $state<LibrariesSlice>({ libs: [] });
  instruments = $state<InstrumentsSlice>({ mcps: [] });
  inference   = $state<InferenceSlice>({ routers: [] });
  assignments = $state<AssignmentsSlice>({
    chains: [],
    roleAssignments:    { inference: null, consolidation: null, embedding: null, voice: null },
    pendingAssignments: { inference: null, consolidation: null, embedding: null, voice: null },
  });

  // ── Derived ──

  get firstPendingStage(): string {
    const pending = this.stages.find(s => s.status !== 'done');
    return pending ? pending.id : 'done';
  }

  get allDone(): boolean {
    return this.stages.every(s => s.status === 'done');
  }

  /**
   * Mirror of healthState.isOk — true when the user has finished the setup
   * wizard. Read by the boot gate (routes/+layout.ts) to decide whether the user should be in
   * the setup flow or the observatory. Reads our own `setupComplete`
   * field, which is the canonical owner of this state (see field docs).
   */
  get isOk(): boolean {
    return this.setupComplete;
  }

  isStageComplete(id: string): boolean {
    return this.stages.find(s => s.id === id)?.status === 'done';
  }

  /** Mark a stage active, clearing any previous active stage.
   *  Rebuilds the array so Svelte 5 derived signals re-evaluate —
   *  in-place mutation of plain-object items is not guaranteed to
   *  propagate through $derived chains. */
  setActive(id: string): void {
    // Idempotency guard — without this, the layout's $effect re-fires every
    // tick because the array rebuild creates new object references that the
    // $derived `stage = stages[currentIdx]` sees as changed, re-triggering
    // the effect that called us. Svelte then aborts with effect_depth_exceeded.
    const current = this.stages.find(s => s.active);
    if (current?.id === id) return;
    this.stages = this.stages.map(s => ({ ...s, active: s.id === id }));
  }

  canAdvance(stageId: string): boolean {
    switch (stageId) {
      case 'preferences': return this.preferences.displayName.trim().length > 0;
      case 'roots':       return this.roots.roots.length > 0;
      // Scan completion now lives on scanState (the live runtime singleton)
      // because the scan can finish while the user is on another stage; the
      // wizardState slice is just the persisted baseline snapshot.
      case 'scan':        return scanState.completed;
      default:            return true;
    }
  }

  // ── Assistant per-part state ────────────────────────────────────

  /**
   * One family as the card and the setup summary show it (audit 2026-10-28,
   * item 12: the card decided its state twice with different precedence, and
   * the summary counted "configured" by a third rule). In-flight work wins —
   * a removal clears the switch first, and still reads configuring. Configured
   * is the live parts once any has moved, else the daemon's word.
   */
  assistantView(family: DaemonAssistantFamily): AssistantView {
    return assistantViewOf(family, this.assistants.partStatus[family.id], this.assistants.partErrors[family.id]);
  }

  /** What setup configured, as the Done page says it (audit 2026-10-28, item 15:
   *  the page assembled these in its template). */
  get doneSummary(): { kanji: string; label: string; sub: string }[] {
    const n = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`;
    const assistants = this.configuredAssistants;
    const roots = this.roots.roots.length;
    const workspaces = this.workspaces.workspaces.length;
    const folders = this.workspaces.workspaces.reduce((t, p) => t + p.folders.length, 0);
    const libs = this.libraries.libs.filter((l) => l.enabled).length;
    const selected = this.instruments.mcps.filter((m) => m.selected).length;
    const installed = this.instruments.mcps.filter((m) => m.installed).length;
    return [
      {
        kanji: '連',
        label: assistants === 0 ? 'No assistants' : n(assistants, 'assistant', 'assistants'),
        sub: assistants === 0 ? 'install Claude/Cursor/etc. to enable sessions' : 'configured with sensei',
      },
      {
        kanji: '庵',
        label: roots === 0 ? 'No roots' : n(roots, 'root', 'roots'),
        sub: folders > 0
          ? `${n(folders, 'folder', 'folders')} across ${n(workspaces, 'workspace', 'workspaces')}`
          : 'no folders scanned yet',
      },
      {
        kanji: '書',
        label: libs === 0 ? 'No libraries' : n(libs, 'library', 'libraries'),
        sub: libs === 0 ? 'nothing to wrap yet' : 'docs & code indexed by sensei',
      },
      {
        kanji: '器',
        label: selected === 0 ? 'No instruments' : n(selected, 'instrument', 'instruments'),
        sub: installed > 0 ? `${installed} already installed in your assistants` : 'recommended for your stack',
      },
    ];
  }

  /** How many families are configured — the same rule the cards show. */
  get configuredAssistants(): number {
    return this.assistants.assistants.filter((f) => this.assistantView(f).configured).length;
  }

  /** Follow the daemon's per-part events (entity "assistant" on the scan
   *  stream) so the cards' chips settle live. Returns the unsubscribe. */
  watchAssistantEvents(port: number): () => void {
    const events = new EventManager<{ action: string; entity: string; data: AssistantPartEvent }>(
      `http://127.0.0.1:${port}/api/scan/events`,
      (data) => JSON.parse(data)
    );
    const unsubscribe = events.subscribe((evt) => {
      if (evt.entity === 'assistant') this.applyAssistantEvent(evt.data);
    });
    return () => {
      unsubscribe();
      events.destroy();
    };
  }

  /**
   * Apply one assistant-part SSE event from the daemon. Single entry point
   * for live status: the assistants page subscribes to /api/scan/events
   * and forwards every `entity: "assistant"` event here. Ignores events
   * for unknown families so a stale subscription after navigation away
   * from the page can't poison the slice.
   *
   * Mutates the slice's `partStatus` / `partErrors` maps with new object
   * references at each level — Svelte 5 derived signals that walk the
   * map need a fresh reference to know they've changed.
   */
  applyAssistantEvent(event: AssistantPartEvent): void {
    if (!this.assistants.assistants.some(f => f.id === event.family)) return;

    const status = { ...this.assistants.partStatus };
    status[event.family] = { ...(status[event.family] ?? {}), [event.part]: event.status };

    const errors = { ...this.assistants.partErrors };
    if (event.status === 'error' && event.error) {
      errors[event.family] = { ...(errors[event.family] ?? {}), [event.part]: event.error };
    } else if (errors[event.family]?.[event.part]) {
      // Clear stale error on the next transition for the same part.
      const fam = { ...errors[event.family] };
      delete fam[event.part];
      errors[event.family] = fam;
    }

    this.assistants = { ...this.assistants, partStatus: status, partErrors: errors };
  }

  /** Mark every part of a family as `configuring`, clearing prior errors.
   *  Called by `toggleAssistant` so the chip strip animates immediately
   *  even if the SSE subscription handshake hasn't returned yet. */
  markFamilyConfiguring(familyId: string): void {
    const family = this.assistants.assistants.find(f => f.id === familyId);
    if (!family) return;
    for (const part of family.parts) {
      this.applyAssistantEvent({ family: familyId, part: part.id, status: 'configuring' });
    }
  }

  /** Mark every part of a family as `done` — used as the success
   *  fallback when the HTTP response arrives before any SSE event. */
  markFamilyDone(familyId: string): void {
    const family = this.assistants.assistants.find(f => f.id === familyId);
    if (!family) return;
    for (const part of family.parts) {
      this.applyAssistantEvent({ family: familyId, part: part.id, status: 'done' });
    }
  }

  /** Mark every part of a family as `error` with the same consolidated
   *  message. The card surfaces one error block under the chip strip. */
  markFamilyError(familyId: string, message: string): void {
    const family = this.assistants.assistants.find(f => f.id === familyId);
    if (!family) return;
    for (const part of family.parts) {
      this.applyAssistantEvent({ family: familyId, part: part.id, status: 'error', error: message });
    }
  }

  /** Reset every part of a family back to `idle` — used after a
   *  successful remove() so the chips return to the empty ring. */
  markFamilyIdle(familyId: string): void {
    const family = this.assistants.assistants.find(f => f.id === familyId);
    if (!family) return;
    for (const part of family.parts) {
      this.applyAssistantEvent({ family: familyId, part: part.id, status: 'idle' });
    }
  }

  /**
   * Flip the per-family switch and trigger configure or remove on the
   * daemon when the user's intent diverges from daemon truth. Optimistically
   * spins the chip strip via `markFamilyConfiguring` so the card animates
   * the moment the user clicks — actual progress refines via SSE.
   *
   * Short-circuits when:
   *   - the family has no installed variants (nothing the daemon can act on)
   *   - the user toggled OFF an already-unconfigured family (asst.remove()
   *     would return false and the daemon's emit_parts would surface an
   *     "remove failed" error chip for a no-op)
   *   - the user toggled ON an already-fully-configured family
   *
   * Throws on HTTP error so the caller can surface a retry. Reverts the
   * switch and surfaces the consolidated error block on failure.
   */
  async toggleAssistant(familyId: string): Promise<void> {
    const family = this.assistants.assistants.find(f => f.id === familyId);
    if (!family) return;
    const variantIds = family.variants.filter(v => v.installed).map(v => v.id);
    if (variantIds.length === 0) return;

    const willEnable = !family.selected;
    const wasConfigured = familyIsConfigured(family);

    // No-op short-circuit: flip the switch only — no daemon round-trip.
    if (willEnable === wasConfigured) {
      family.selected = willEnable;
      this.markFamilyIdle(familyId);
      return;
    }

    // Mark configuring BEFORE flipping selected. Svelte batches in the
    // same tick so there's no flash, but ordering matters when the chip
    // strip is being driven off `partStatus` — the spinner appears as
    // soon as the user clicks.
    this.markFamilyConfiguring(familyId);
    family.selected = willEnable;

    const api = senseiApi(appState.port);
    const failure = assistantWriteFailure(
      willEnable ? await api.tryConfigureAssistants(variantIds) : await api.tryRemoveAssistants(variantIds),
    );
    if (failure) {
      family.selected = !willEnable;
      this.markFamilyError(familyId, failure);
      throw new Error(failure);
    }
    if (willEnable) {
      for (const v of family.variants) if (v.installed) v.configured = true;
      this.markFamilyDone(familyId);
    } else {
      for (const v of family.variants) v.configured = false;
      this.markFamilyIdle(familyId);
    }
  }

  /** Retry configure for a family that currently has at least one part
   *  in `error`. Equivalent to forcing the switch on. */
  async retryAssistant(familyId: string): Promise<void> {
    const family = this.assistants.assistants.find(f => f.id === familyId);
    if (!family) return;
    if (!family.selected) family.selected = true;
    const variantIds = family.variants.filter(v => v.installed).map(v => v.id);
    if (variantIds.length === 0) return;

    const api = senseiApi(appState.port);
    this.markFamilyConfiguring(familyId);
    const failure = assistantWriteFailure(await api.tryConfigureAssistants(variantIds));
    if (failure) {
      this.markFamilyError(familyId, failure);
      return;
    }
    for (const v of family.variants) if (v.installed) v.configured = true;
    this.markFamilyDone(familyId);
  }

  // ── Lifecycle ──

  /**
   * Mark the wizard as completed end-to-end. Writes setup_complete to the
   * daemon and updates the appState cache + localStorage sync-gate so the
   * boot gate sees the new state immediately on the next navigation.
   *
   * Lives here (not on appState) because completion is a wizard concern —
   * appState just exposes the resulting flag via .setupOk for callers that
   * want a status read without coupling to the wizard module.
   */
  async setCompleted(): Promise<void> {
    // setConfigs throws on daemon failure — the done-stage commit handler
    // re-raises so the wizard layout can show the error instead of
    // navigating to the observatory.
    await appState.setConfigs({ setup_complete: '1' });
    this.setupComplete = true;
    const ls = (globalThis as { localStorage?: Storage }).localStorage;
    if (ls && typeof ls.setItem === 'function') {
      ls.setItem(STORAGE_KEYS.setupComplete, '1');
    }
  }

  /** True once load() has resolved at least once. The (config) layout uses
   *  this as the gate before rendering — keeps wizard pages from flashing
   *  with empty state during the cold mount. */
  loaded = $state(false);

  /**
   * Fetch wizard data from the daemon and apply it. Single entry point —
   * components / layouts call this; nobody else should reach into the
   * daemon for wizard concerns. Mirrors `healthState.init()` and
   * `appState.load()` so all three globals share the same idempotent
   * load() + state-driven UI pattern.
   *
   * Returns false on daemon unreachable so the layout can surface a 503
   * instead of mounting with stale or empty state.
   */
  async load(): Promise<boolean> {
    try {
      const data = await loadWizardData(appState.port);
      await this.hydrate(data);
      this.loaded = true;
      return true;
    } catch {
      // Don't clear existing state — a transient daemon outage shouldn't
      // wipe the cache the user is currently looking at (mirrors the
      // contract appState.load() now follows after the comment/code
      // contradiction was resolved).
      return false;
    }
  }

  /**
   * Re-read the watch roots from the daemon, for a screen outside setup (#247).
   *
   * Only the setup layout calls `load()`, so the settings Roots screen used to
   * start empty and stay empty — and the remove action with it. Returns `null`
   * on success, else the failure in words; ON FAILURE THE CACHED LIST IS KEPT,
   * because an emptied list would read as "you have no roots".
   */
  async reloadRoots(api: {
    tryGetScanRoots: () => Promise<ApiResult<DaemonWatchRoot[]>>;
  }): Promise<string | null> {
    const res = await api.tryGetScanRoots();
    if (!res.ok) return daemonFailure(res.error);
    this.roots = { ...this.roots, roots: res.data };
    return null;
  }

  /**
   * Add the typed path as a watch root. Returns `null` when it was added (or
   * there was nothing new to add), else the failure in words; ON FAILURE THE
   * TYPED PATH AND THE LIST ARE KEPT, so the user can correct and retry (#304).
   */
  async addRoot(api: {
    tryAddWatchRoot: (path: string) => Promise<ApiResult<{ ok: boolean; id: string; path: string }>>;
  }): Promise<string | null> {
    const path = this.roots.newPath.trim();
    if (!path || this.roots.roots.some((r) => r.path === path)) return null;
    const res = await api.tryAddWatchRoot(path);
    if (!res.ok) return daemonFailure(res.error);
    const added: DaemonWatchRoot = {
      id: res.data.id,
      path: res.data.path,
      name: res.data.path.split('/').pop() ?? res.data.path,
      status: 'scanning',
      excluded: [],
      repos_found: 0,
      scanned: false,
      modified_at: new Date().toISOString(),
    };
    this.roots = { roots: [...this.roots.roots, added], newPath: '' };
    return null;
  }

  async hydrate(data: WizardLoadData): Promise<void> {
    // Wizard init resets the live scan runtime — a fresh wizard session
    // starts with an empty SSE stream + Begin button. The runtime survives
    // stage navigation within the session; only re-entering the wizard
    // (which is what triggers hydrate) clears it.
    scanState.reset();

    // Reconcile the canonical `setupComplete` flag against daemon truth
    // and propagate to localStorage. Single source of truth: this method
    // and `setCompleted()` are the only writers — anywhere else that
    // touches `localStorage[setupComplete]` is a layering violation.
    //
    // Guard the global access — vitest's node env can leave a partial
    // `localStorage` global on globalThis without setItem/removeItem
    // wired up; `typeof localStorage !== 'undefined'` alone isn't enough.
    this.setupComplete = data.setupComplete;
    const ls = (globalThis as { localStorage?: Storage }).localStorage;
    if (ls && typeof ls.setItem === 'function' && typeof ls.removeItem === 'function') {
      if (data.setupComplete) ls.setItem(STORAGE_KEYS.setupComplete, '1');
      else                    ls.removeItem(STORAGE_KEYS.setupComplete);
    }

    // Reset stages from the canonical static defs, then layer persisted status.
    this.stages = cloneStages();
    for (const s of this.stages) {
      if (data.completion[s.id] === 'done') s.status = 'done';
    }

    this.preferences = { ...data.preferences };

    // Prefill displayName from system username if empty
    if (!this.preferences.displayName) {
      const user = await guessUserName();
      if (user) this.preferences.displayName = user;
    }

    // Default selection: if the family is already configured on the daemon,
    // start with it selected (so the switch reflects daemon truth). If it's
    // installed but not configured, also start selected so the user's first
    // pass through configures it. Uninstalled families start unselected.
    //
    // Seed `partStatus` from variant.configured: every part of an already-
    // configured family lands in `done`, anything else starts `idle`. SSE
    // events from a later configure() run override these as transitions happen.
    const partStatus: Record<string, Record<string, AssistantPartStatus>> = {};
    for (const a of data.assistantFamilies) {
      const configured = familyIsConfigured(a);
      partStatus[a.id] = Object.fromEntries(
        a.parts.map(p => [p.id, configured ? 'done' : 'idle' as AssistantPartStatus])
      );
    }
    this.assistants = {
      assistants: data.assistantFamilies.map(a => ({
        ...a,
        selected: a.selected ?? (a.variants.some(v => v.installed)),
      })),
      partStatus,
      partErrors: {},
    };

    this.roots = { roots: [...data.roots], newPath: '' };

    const scannedRoots = data.roots.filter(r => r.scanned);
    this.scan = {
      baseline: {
        rootCount: scannedRoots.length,
        repoCount: data.roots.reduce((sum, r) => sum + r.repos_found, 0),
        fileCount: 0,
        scannedRootIds: scannedRoots.map(r => r.id),
      },
      started: false,
      done: false,
    };

    // Each loaded workspace starts as confirmed — the user has the option to
    // unconfirm via the Workspaces stage, in which case it is excluded from commit.
    this.workspaces = {
      workspaces: data.workspaces.map(p => ({ ...p, folders: p.folders ?? [] })),
      confirmed: Object.fromEntries(data.workspaces.map(p => [p.id, true])),
    };
    this.libraries = { libs: [...data.libraries.libs] };
    this.instruments = { mcps: [...data.mcps] };
    // A failed router read leaves the list as it was (#286); the Providers
    // screen re-reads on mount and shows the error.
    if (data.routers) this.inference = {
      routers: data.routers.map(r => ({
        id: r.id,
        name: r.name,
        providers: r.providers,
        capabilities: r.capabilities,
        needsKey: r.needs_key,
        configured: r.configured,
        keyError: r.key_error ?? null,
        draftKey: '',
        saveState: 'idle' as RouterSaveState,
        saveError: '',
      })),
    };
  }

  /**
   * Re-fetch instruments (MCPs) from daemon, preserving the user's local
   * selection for entries that survive the refresh. The daemon recomputes
   * `recommended` per request based on the detected stack, so we re-honour
   * its truth except where the user explicitly toggled.
   */
  async refreshInstruments(): Promise<void> {
    const read = await senseiApi(appState.port).tryListInstruments();
    // A failed read throws: the section shows why (#288), not "no instruments".
    if (!read.ok) throw new Error(failureReason(read.error));
    const fresh = read.data;
    const previous = new Map(this.instruments.mcps.map(m => [m.id, m.selected]));
    this.instruments = {
      mcps: fresh.mcps.map(m => ({
        ...m,
        selected: previous.get(m.id) ?? m.selected,
      })),
    };
  }

  /**
   * Re-fetch libraries from daemon, preserving the user's local enable/disable
   * intent for libs that survive the refresh. Daemon discovers libs during
   * scan, so the initial wizard hydrate may run before any libs exist.
   *
   * Filter libs to those touching at least one repo scanned in this wizard run
   * to avoid showing stale libs from the persistent DB.
   */
  async refreshLibraries(): Promise<void> {
    const api = senseiApi(appState.port);
    // The folders are read with the libraries: the session's copy is a snapshot, and
    // folders indexed since would hide their libraries until a restart.
    const [read, folders] = await Promise.all([api.tryGetLibs(), api.tryListWorkspaces()]);
    // A failed read throws: the section shows why (#288), not "no libraries".
    if (!read.ok) throw new Error(failureReason(read.error));
    if (!folders.ok) throw new Error(failureReason(folders.error));
    const fresh = read.data;

    // Repo (folder) names of the registered workspaces. Filter libs to those
    // touching at least one of these so we don't show the entire persistent DB.
    const scannedRepoNames = new Set<string>();
    for (const p of folders.data) {
      for (const f of p.folders ?? []) scannedRepoNames.add(f.name);
    }

    const previous = new Map(this.libraries.libs.map(l => [l.name, l.enabled]));
    this.libraries = {
      libs: fresh.libs
        .filter(l => (l.repos ?? []).some((r: string) => scannedRepoNames.has(r)))
        .map(l => ({
          id: l.id || l.name,
          name: l.name,
          ecosystem: l.ecosystem ?? '',
          version: l.version ?? null,
          description: l.description ?? null,
          pageCount: l.pageCount ?? 0,
          repos: l.repos ?? [],
          repoCount: l.repoCount ?? (l.repos?.length ?? 0),
          enabled: previous.get(l.name) ?? true,
        })),
    };
  }

  /**
   * Re-fetch gateway routers from daemon, preserving per-card draft state
   * (draftKey, saveState, saveError) for entries that survive the refresh.
   */
  /**
   * Hydrate the assignments slice from the daemon's chain list. Called
   * by the Model Assignments page's `+page.ts` load so the wizard state
   * is always in sync with `gateway.fallback_chains` at page-open time.
   * Preserves the user's `pendingAssignments` when possible — a refresh
   * mid-edit shouldn't wipe uncommitted picks.
   */
  async refreshAssignments(): Promise<void> {
    const read = await senseiApi(appState.port).tryListGatewayChains();
    // A failed read throws (#288): every role read as unassigned.
    if (!read.ok) throw new Error(failureReason(read.error));
    const chains = read.data.chains;
    // Daemon truth: role → chainId for the four supported roles.
    const daemon = roleAssignments(chains);
    this.assignments = {
      chains,
      roleAssignments:    { ...daemon },
      // Preserve any prior pending pick that's still valid; fall back
      // to the daemon's truth otherwise so the UI has something to show.
      pendingAssignments: {
        inference:     this.assignments.pendingAssignments.inference    ?? daemon.inference,
        consolidation: this.assignments.pendingAssignments.consolidation ?? daemon.consolidation,
        embedding:     this.assignments.pendingAssignments.embedding    ?? daemon.embedding,
        voice:         this.assignments.pendingAssignments.voice        ?? daemon.voice,
      },
    };
  }

  async refreshInferenceRouters(): Promise<void> {
    const api = senseiApi(appState.port);
    const read = await api.tryListGatewayRouters();
    // A failed read is an error the Providers screen shows (#286), not "no routers".
    if (!read.ok) throw new Error(failureReason(read.error));
    const fresh = read.data;
    const previous = new Map(this.inference.routers.map(r => [r.id, r]));
    this.inference = {
      routers: fresh.routers.map(r => {
        const prev = previous.get(r.id);
        return {
          id: r.id,
          name: r.name,
          providers: r.providers,
          capabilities: r.capabilities,
          needsKey: r.needs_key,
          configured: r.configured,
        keyError: r.key_error ?? null,
          draftKey: prev?.draftKey ?? '',
          saveState: prev?.saveState ?? 'idle',
          saveError: prev?.saveError ?? '',
        };
      }),
    };
  }

  /**
   * Clear the stored API key for a gateway router and refresh the list.
   */
  async clearInferenceRouterKey(id: string): Promise<void> {
    const api = senseiApi(appState.port);
    await api.clearGatewayRouterKey(id);
    await this.refreshInferenceRouters();
  }

  /**
   * Commit the named stage's data to the daemon and mark the stage done.
   *
   * Throws on failure — the layout's commit handler catches and surfaces
   * the error message via `commitError`. Previously this method swallowed
   * errors for non-`done` stages and returned `false`, which left the
   * user with a stuck Continue button and no feedback. The aggregate
   * error message is additive to any per-card UI (e.g. assistants /
   * inference set their own per-row failure state in their handlers).
   *
   * Returns `void` on success — handler-less stages (welcome, roots,
   * scan) succeed trivially.
   */
  async commitStage(stageId: string): Promise<void> {
    const api = senseiApi(appState.port);
    const handler = COMMIT_HANDLERS[stageId];
    if (handler) {
      await handler(this, api);
      // Marker write goes through appState so the in-memory config picks
      // up the `setup.X: done` keys — that matters for the boot gate's setup
      // gate and any other reader of appState.config that runs before the
      // next load().
      await appState.setConfigs({ [`setup.${stageId}`]: 'done' });
    }
    const stage = this.stages.find(s => s.id === stageId);
    if (stage) stage.status = 'done';
  }
}


export const wizardState = new WizardState();
