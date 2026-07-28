import type { LayoutLoad } from './$types';

// The inbox rail is fed by the client-side three-layer state (relay-inbox-state +
// loadRelayInbox), not a server load — so this layout does no data fetch. Kept as a
// no-op load. When the real user-wide read lands it goes in loadRelayInbox's body
// (client-side, membership-scoped), NOT here (a rune singleton must not be populated
// during SSR). See docs/spec/dojo-screens/inbox.md.
export const load: LayoutLoad = () => ({});
