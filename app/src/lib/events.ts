// The SSE event manager lives in @sensei/state (#421): the relay source there
// subscribes through it, and the scan screens here still import it from this path.
export { EventManager, type EventHandler } from '@sensei/state';
