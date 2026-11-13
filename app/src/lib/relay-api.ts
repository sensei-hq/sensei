// Relay · the desktop's client contract for the daemon's relay API (D90, slice 1 item 4).
//
// The shapes came first (phase A), so the screen's tests can mock the client; the
// runtime factory below is phase B's. The screen reaches the daemon through it alone.
//
// Paths: the daemon's relay routes are L3's to publish (ruling Q9). Until they are,
// these are the ones the screen asks for — `relay/now`, `relay/queue`, `relay/tasks/
// {crew}/{task}`, and the three writes — and a change on the daemon's side is a change
// here, in one file.
//
// Shapes follow `docs/prd/modules/03-delivery/21-relay/lanes-data.md` (the views
// `relay.now` and `relay.task_history`, and the action kinds) and the interaction
// contract `coord/mockups/relay-lanes/CONTRACT.md` §3.
//
// Repository required, workspace optional (lanes-data, user 2026-11-05): every task names its
// `repo_key`; `workspace_id` is a tag and a filter, never a gate. Reads are scoped by the user and
// the crew. The main window reads every crew; the workspace window reads the same views filtered
// to its workspace.
//
// D83: every self-reported value is nullable. `null` means "not reported" and the
// screen says so; it is never coerced to 0. Times are ISO instants; the screen
// formats them.

import { apiBase } from './api.js';
import { relayClient, relayEvents, parseRelayStreamEvent, type EventManager, type RelayApi, type RelayStreamEvent } from '@sensei/state';

// The wire shapes live in @sensei/state (#421); every importer of this client still finds them here.
export type * from '@sensei/state';
export { parseRelayStreamEvent };

// ─── The client ─────────────────────────────────────────────────────────────
// The client lives in @sensei/state (#421) by base URL; the app builds it from the daemon's port.

/** What the screens build once from the daemon's port. */
export type RelayApiFactory = (port: number) => RelayApi;

/** The relay client: every call the daemon's answer or an `ApiResult` error, the
 *  daemon's own refusal text when it sends one (a 409's reason is the useful part). */
export const relayApi: RelayApiFactory = (port) => relayClient(apiBase(port));

/** The relay socket, owned by the route that subscribes (never by shared state). */
export const relayStream = (port: number): EventManager<RelayStreamEvent> => relayEvents(apiBase(port));
