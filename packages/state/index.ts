// @sensei/state — domain state shared by the desktop app and the dōjō (#414).
export type { StateEvent, StreamFrame, StreamFrameError } from './events.js';
export {
  sseEvents,
  realtimeEvents,
  type StateStream,
  type EventStream,
  type ChannelStatus,
  type ChannelSubscriber,
} from './source.js';
export { ReactiveStageContext } from './stage.svelte.js';
export { EventManager, type EventHandler } from './event-manager.js';
export { failureFrom, failureReason, type ApiError, type ApiResult } from './result.js';

// The relay domain (#421): the wire shapes and the stream event's shape-check.
export * from './relay/types.js';
export { relayClient, relayEvents, type RelayApi } from './relay/client.js';
export {
  RelayState,
  type RelayReads,
  type RelaySource,
  type RelayEvents,
  type RelaySprintDraft,
  type RelayLaneView,
  type RelayStateOptions,
} from './relay/relay-state.svelte.js';
export {
  DecisionsState,
  ItemsState,
  LanesState,
  MessagesState,
  RunsState,
  TasksState,
  type RelayItemRow,
  type RelayRunRow,
  type RelayTaskRow,
} from './relay/entity-states.svelte.js';
export { relaySseSource, relayTaskSseSource, type RelayEventStream } from './relay/sources/sse.js';
export {
  relayRealtimeSource,
  relayRealtimeTaskSource,
  cannedRealtime,
  type CannedWrite,
  type RelayChannelStatus,
  type RelayRealtimeTransport,
} from './relay/sources/realtime.js';
export {
  fromPostgresChange,
  RELAY_TABLES,
  type PostgresChangePayload,
  type RelayTableEntry,
  type RelayTableMap,
} from './relay/sources/postgres-change.js';
export {
  realtimeChannel,
  type RealtimeChannelLike,
  type RealtimeChannelSpec,
  type RealtimeClientLike,
} from './relay/sources/channel.js';
export {
  toStats,
  firstPassLine,
  isIdle,
  isWorking,
  minutesSince,
  type RelayStats,
} from './relay/relay-selectors.js';
export {
  toDecisionForm,
  toAnswerValues,
  textCap,
  textMisses,
  toPatternMatcher,
  withoutTextRules,
  withArrayLayout,
  type DecisionFormView,
  type TextMiss,
} from './relay/decision-form.js';
export {
  RelayTaskState,
  type RelayTaskSource,
  type RelayTaskStateOptions,
  type TaskReads,
} from './relay/task-state.svelte.js';
export {
  nameOf,
  nameOfCode,
  crewGroups,
  crewRepoLabel,
  crewSizeContext,
  crewTail,
  currentStepKey,
  hhmm,
  laneState,
  providerLabel,
  roundsLabel,
  since,
  taskHref,
  taskStateLabel,
  ticketLabel,
  todayLine,
  answerGist,
} from './relay/format.js';
export {
  assistantLabel,
  toCompletedRows,
  toNeedsYou,
  toWaitRows,
  type NowRowOptions,
  type RelayNeedsYouView,
  type RelayWaitRowView,
  type RelayWaitNudge,
  type RelayCompletedRowView,
  type RelayLaneRowView,
  type RelayTone,
} from './relay/now-selectors.js';
export {
  toAgentGroups,
  toRunGroups,
  type RelayAgentGroupView,
  type RelayCrewAgentsView,
  type RelayCrewRunsView,
  type RelayRunGroupView,
  type RelayRunTaskRowView,
} from './relay/run-selectors.js';
export { toQueueRows, type RelayQueueRowView } from './relay/queue-selectors.js';
export {
  editorTag,
  toAddMember,
  toMemberGroups,
  toRunCard,
  toPastRuns,
  toArchivedRun,
  runHref,
  type RelayAddMemberView,
  type RelayCatalogueInput,
  type RelayCatalogueItemView,
  type RelayCatalogueView,
  type RelayMemberGroupView,
  type RelayMemberView,
  type RelayPresence,
  type RelayRunCardView,
  type RelayPastRunView,
  type RelayArchivedRunView,
} from './relay/crew-selectors.js';
export {
  toTaskView,
  type RelayHandoffView,
  type RelayRunDetailsView,
  type RelayTaskHeaderView,
  type RelayTaskPartsView,
} from './relay/task-selectors.js';

// The scan domain (#496): the wire shapes, the daemon client, and the state.
export type {
  FolderStatus,
  WorkspaceStatus,
  ActivityLevel,
  ScanWorkspaceFolder,
  ScanFolderEvent,
  ScanWorkspace,
  ActivityEvent,
  ScanDaemonWorkspace,
  ScanDaemonRoot,
  QueueCounts,
  IndexQueueStatus,
} from './scan/types.js';
export { parseScanStreamEvent } from './scan/types.js';
export { scanClient, scanEvents, type ScanApi } from './scan/client.js';
export { scanSseSource, type ScanEventStream } from './scan/sources/sse.js';
export {
  ScanWorkspaceState,
  ScanActivityState,
  ScanState,
  commonParent,
  pollStep,
  triggerScans,
  miniSummary,
  type ScanSource,
  type ScanRunStatus,
} from './scan/scan-state.svelte.js';

// The ops domain (operations center): the wire shapes, the daemon client,
// the per-domain sources, and the states.
export type {
  OpsTaskView,
  OpsBackoffView,
  OpsKindDepth,
  OpsQueueListing,
  OpsFailedTask,
  OpsAbandonedFeed,
  OpsIndexError,
  OpsScheduledTask,
  OpsSchedulePatch,
  OpsScheduleLog,
  OpsSyncStateRow,
  OpsSyncStateFeed,
  OpsWave,
  OpsTaskDetail,
  OpsQueuedTaskDetail,
  OpsBackfillResult,
  OpsAlertAck,
  OpsWaveRun,
} from './ops/types.js';
export { opsClient, opsEvents, parseOpsFrame, type OpsApi } from './ops/client.js';
export {
  alertsSource,
  jobsSseSource,
  schedulesSseSource,
  syncSseSource,
  type OpsEventStream,
} from './ops/sources/sse.js';
export { WaveState, waveProgress, type WaveRow } from './ops/wave-state.svelte.js';
export {
  classifyAlert,
  sortAlerts,
  type AlertClass,
  type AlertKind,
  type AlertLevel,
  type AlertSeverity,
  type OpsAlert,
} from './ops/alerts.js';
export { AlertsState, type AlertsDeps, type AlertsSource } from './ops/alerts-state.svelte.js';
export {
  JobsState,
  classifyJobFrame,
  JOBS_POLL_MS,
  type JobsSource,
} from './ops/jobs-state.svelte.js';
export {
  SchedulesState,
  pausedSchedules,
  failingSchedules,
  SCHEDULES_POLL_MS,
  type SchedulesSource,
} from './ops/schedules-state.svelte.js';
export {
  SyncState,
  classifySyncStage,
  SYNC_POLL_MS,
  type SyncSource,
  type SyncStageActivity,
} from './ops/sync-state.svelte.js';
