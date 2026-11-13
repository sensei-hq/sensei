// @sensei/state — domain state shared by the desktop app and the dōjō (#414).
export type { StateEvent } from './events.js';
export { ReactiveStageContext } from './stage.svelte.js';
export { EventManager, type EventHandler } from './event-manager.js';
export { failureReason, type ApiError, type ApiResult } from './result.js';

// The relay domain (#421): the wire shapes and the stream event's shape-check.
export * from './relay/types.js';
export { relayClient, relayEvents, type RelayApi } from './relay/client.js';
