// @sensei/state — domain state shared by the desktop app and the dōjō (#414).
export type { StateEvent } from './events.js';
export { ReactiveStageContext } from './stage.svelte.js';
export { EventManager, type EventHandler } from './event-manager.js';
export { failureReason, type ApiError, type ApiResult } from './result.js';

// The relay domain (#421): the wire shapes and the stream event's shape-check.
export * from './relay/types.js';
export { relayClient, relayEvents, type RelayApi } from './relay/client.js';
export {
  RelayState,
  type RelayReads,
  type RelaySource,
  type RelaySprintDraft,
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
export { relaySseSource, type RelayEventStream } from './relay/sources/sse.js';
export {
  relayRealtimeSource,
  cannedRealtime,
  type CannedWrite,
  type RelayChannelStatus,
  type RelayRealtimeTransport,
} from './relay/sources/realtime.js';
export {
  toStats,
  firstPassLine,
  isIdle,
  isWorking,
  minutesSince,
  type RelayStats,
} from './relay/relay-selectors.js';
export { RelayTaskState, type TaskReads } from './relay/task-state.svelte.js';
export {
  codeOf,
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
} from './relay/format.js';
export {
  assistantLabel,
  toCompletedRows,
  toLaneGroups,
  toNeedsYou,
  toWaitRows,
  type NowRowOptions,
  type RelayNeedsYouView,
  type RelayWaitRowView,
  type RelayCompletedRowView,
  type RelayLaneGroupView,
  type RelayLaneRowView,
  type RelayTone,
} from './relay/now-selectors.js';
export { toQueueRows, type RelayQueueRowView } from './relay/queue-selectors.js';
export {
  toAddMember,
  toMemberGroups,
  toRunCard,
  type RelayAddMemberView,
  type RelayMemberGroupView,
  type RelayMemberView,
  type RelayPresence,
  type RelayRunCardView,
} from './relay/crew-selectors.js';
