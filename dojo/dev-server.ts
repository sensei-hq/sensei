// Where `vite dev` listens (#185). `localhost` alone bound IPv6 only (`::1`), while
// the daemon's HTTP client resolves `localhost` to IPv4 — so `dojo_sync` against a
// local dōjō could never connect. Bind the address the daemon dials, on the port a
// local membership names; `strictPort` fails loudly instead of drifting to 5174,
// which would be just as unreachable and say less. A browser on `localhost` still
// reaches it.
export const devServer = {
	host: '127.0.0.1',
	port: 5173,
	strictPort: true,
	// packages/* is shared with the desktop app and sits outside this app (#133, #293).
	fs: { allow: ['.', '../packages'] as string[] }
} as const;
