// Observatory · Instruments — the screen's own state: the open tab, the
// playground's call, the replay session list and selection, and the health
// header's first-try rate. The tool catalogue lives on the mcp store; this
// holds only what the screen owns. Every read is the daemon's answer or why
// it failed (#288).
import { failureReason, readOutcome, type ApiResult } from '$lib/api.js';
import type { McpToolCallResult, ObservatoryFtr, SessionData, SessionReplayCall, ToolHealthSource } from '$lib/types.js';
import { loadReplaySessions, type ReplaySession } from './replay-sessions';
import type { InstrumentsTab } from './instruments-view';
import { formFields, resultText, toArguments } from './tool-form';
import { parseServerForm, type ServerBody, type ServerForm } from './server-form';

/** The reads the screen makes — injected so a spec can fail them. */
export interface InstrumentsApi {
  tryCallServerTool: (
    serverId: string,
    tool: string,
    args: Record<string, unknown>,
    workspaceId: string | null,
  ) => Promise<ApiResult<McpToolCallResult>>;
  tryGetSessions: () => Promise<ApiResult<Pick<SessionData, 'sessions'> & Partial<SessionData>>>;
  tryGetObservatoryFtr: () => Promise<ApiResult<ObservatoryFtr>>;
  tryListWatchedWorkspaces: () => Promise<ApiResult<Array<{ id: string; name: string }>>>;
}

type ReplaySummary = { used: number; partial: number; ignored: number; total: number };

/** The part of the mcp store the screen's actions use. */
export interface InstrumentsStore {
  loadSessionReplay: (
    sessionId: string,
    opts: { classify?: boolean },
  ) => Promise<{ replay: { calls: SessionReplayCall[]; summary: ReplaySummary } | null; error: string | null }>;
  registerMcpServer: (server: ServerBody) => Promise<string | null>;
}

const NO_CALLS: ReplaySummary = { used: 0, partial: 0, ignored: 0, total: 0 };

export class InstrumentsState {
  tab = $state<InstrumentsTab>('playground');

  // Playground — the rail and the selected tool's call.
  collapsedGroups = $state<Record<string, boolean>>({});
  toolSearch = $state('');
  selectedToolId = $state<string | null>(null);
  toolParams = $state<Record<string, string>>({});
  toolResult = $state('');
  /** Why the last call failed — never an empty answer in its place. */
  toolError = $state<string | null>(null);
  /** The last call's round trip, as the daemon measured it. */
  durationMs = $state<number | null>(null);
  executing = $state(false);
  /** The workspace a call runs in — its server starts in that workspace's
   *  root, as it would for an assistant working there. None: the daemon's own
   *  folder, which the screen says. */
  runIn = $state<string | null>(null);
  workspaces = $state<Array<{ id: string; name: string }>>([]);
  workspacesError = $state<string | null>(null);
  refreshingServers = $state(false);
  /** The last refused toggle or failed scan, in words; cleared on success. */
  serversError = $state<string | null>(null);
  /** The "add a server" form (#343), and why it was last refused. */
  addingServer = $state(false);
  serverForm = $state<ServerForm>({ name: '', command: '', args: '', env: '' });
  serverFormError = $state<string | null>(null);

  // Replay — the session list, the open session, the selected call.
  replaySessions = $state<ReplaySession[]>([]);
  replayLoading = $state(false);
  replayError = $state<string | null>(null);
  /** Whether the list has been read, successfully or not, so an empty or
   *  failed read is not re-fetched without end. Retry clears it. */
  replayLoaded = $state(false);
  selectedSessionId = $state<string | null>(null);
  sessionCalls = $state<SessionReplayCall[]>([]);
  sessionSummary = $state<ReplaySummary>({ ...NO_CALLS });
  replayDetailError = $state<string | null>(null);
  selectedCall = $state<SessionReplayCall | null>(null);

  // Health — the L1 grid or one source's L2 drill, and the KPI header's rate.
  activeSource = $state<ToolHealthSource | null>(null);
  selectedInsightTool = $state<string | null>(null);
  /** null until read, so the card shows "—" rather than a fabricated 0%. */
  holisticFtr = $state<{ data: ObservatoryFtr | null; error: string | null } | null>(null);

  constructor(tab: InstrumentsTab) {
    this.tab = tab;
  }

  /** Focus a tool with its prefilled inputs; an earlier answer or failure goes. */
  pickTool(id: string, params: Record<string, string>): void {
    this.selectedToolId = id;
    this.toolParams = params;
    this.toolResult = '';
    this.toolError = null;
    this.durationMs = null;
  }

  /** The workspaces a call can run in — those under a watched root, by name;
   *  the active one is chosen when it is among them. */
  async loadWorkspaces(api: InstrumentsApi, active: string | null): Promise<void> {
    const read = await api.tryListWatchedWorkspaces();
    if (!read.ok) {
      this.workspaces = [];
      this.workspacesError = `Could not list the workspaces: ${failureReason(read.error)}`;
      return;
    }
    this.workspacesError = null;
    this.workspaces = read.data.map(({ id, name }) => ({ id, name })).sort((a, b) => a.name.localeCompare(b.name));
    this.runIn = this.workspaces.some((w) => w.id === active) ? active : null;
  }

  /** Run a tool through its server with the form's values, typed by its
   *  schema, in the chosen workspace, and show its answer — or why there is
   *  none. */
  async execute(api: InstrumentsApi, tool: { serverId: string; name: string; schema: unknown }): Promise<void> {
    this.toolResult = '';
    this.toolError = null;
    this.durationMs = null;
    const args = toArguments(formFields(tool.schema), this.toolParams);
    if (!args.ok) {
      this.toolError = args.error;
      return;
    }
    this.executing = true;
    const read = await api.tryCallServerTool(tool.serverId, tool.name, args.args, this.runIn);
    if (read.ok) {
      this.toolResult = resultText(read.data.result);
      this.durationMs = read.data.duration_ms;
      if (read.data.isError) this.toolError = 'The tool reported an error';
    } else {
      this.toolError = `The call failed: ${failureReason(read.error)}`;
    }
    this.executing = false;
  }

  /** Register the form's server; the form closes only once the daemon took it. */
  async addServer(store: Pick<InstrumentsStore, 'registerMcpServer'>): Promise<void> {
    const parsed = parseServerForm(this.serverForm);
    if (!parsed.ok) {
      this.serverFormError = parsed.error;
      return;
    }
    const refused = await store.registerMcpServer(parsed.server);
    this.serverFormError = refused;
    if (refused) return;
    this.addingServer = false;
    this.serverForm = { name: '', command: '', args: '', env: '' };
  }

  /** Read the session list the first time Replay opens. */
  async loadReplaySessions(api: InstrumentsApi): Promise<void> {
    if (this.replayLoaded || this.replayLoading) return;
    this.replayLoading = true;
    const read = await loadReplaySessions(api);
    this.replaySessions = read.sessions;
    this.replayError = read.error;
    this.replayLoaded = true;
    this.replayLoading = false;
  }

  async retryReplaySessions(api: InstrumentsApi): Promise<void> {
    this.replayLoaded = false;
    await this.loadReplaySessions(api);
  }

  /** Open a session: its calls with their verdicts (classified on first open). */
  async selectReplaySession(store: InstrumentsStore, sessionId: string): Promise<void> {
    this.selectedSessionId = sessionId;
    this.selectedCall = null;
    this.sessionCalls = [];
    this.sessionSummary = { ...NO_CALLS };
    this.replayLoading = true;
    const read = await store.loadSessionReplay(sessionId, { classify: true });
    this.replayDetailError = read.error;
    if (read.replay) {
      this.sessionCalls = read.replay.calls;
      this.sessionSummary = read.replay.summary;
    }
    this.replayLoading = false;
  }

  /** Read the 14-day first-try rate once, for the Health header. */
  async loadHolisticFtr(api: InstrumentsApi): Promise<void> {
    if (this.holisticFtr != null) return;
    this.holisticFtr = readOutcome(await api.tryGetObservatoryFtr());
  }
}
