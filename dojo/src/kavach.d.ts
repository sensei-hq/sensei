// Ambient declarations for the virtual modules that @kavach/vite generates from
// kavach.config.js at build time ($kavach/auth, $kavach/providers). Kept in a
// pure ambient file (no top-level import/export) so `declare module` stays
// global and svelte-check resolves the specifiers.

declare module '$kavach/auth' {
	import type { AuthAdapter } from 'kavach';

	// The kavach instance created in the generated module (auth-supabase.js).
	// Only the members this app uses are typed.
	export const kavach: {
		handle: (input: { event: unknown; resolve: unknown }) => Response | Promise<Response>;
	};
	// The supabase auth adapter, passed to createKavach in the client layout.
	export const adapter: AuthAdapter;
	// The kavach logger instance.
	export const logger: unknown;
}

declare module '$kavach/providers' {
	export const providers: Array<{ name: string; mode?: string; label: string }>;
}
