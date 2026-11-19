// @sensei/state — domain state shared by the desktop app and the dōjō (#414).
export type { StateEvent } from './events.js';
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
  textMisses,
  toPatternMatcher,
  withoutTextRules,
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
