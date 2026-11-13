// @sensei/state — domain state shared by the desktop app and the dōjō (#414).
export type { StateEvent } from './events.js';
export { ReactiveStageContext } from './stage.svelte.js';

// The relay domain (#421): the wire shapes and the stream event's shape-check.
export * from './relay/types.js';
