// Load seam (layer 3 of ui-state-pattern) — returns UI-typed sessions and hands
// them to the state. Mock-first so the components build to fidelity before the
// user-wide read exists. Later, swap this body to a real fetch + transform
// (relay_sessions + relay_inbox + relay_segments → RelaySession[], scoped to the
// user's memberships across all dōjōs) — component + state stay untouched.
import type { RelaySession } from './types';
import { relayInboxMock } from './relay-inbox.mock';

export function loadRelayInbox(): RelaySession[] {
	return relayInboxMock();
}
