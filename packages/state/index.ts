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
