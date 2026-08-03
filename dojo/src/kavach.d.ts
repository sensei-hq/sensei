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
	export const providers: Array<{
		name: string;
		mode?: 'otp' | 'oauth' | 'password';
		label: string;
		scopes?: string[];
	}>;
}

// @kavach/ui ships .svelte components without .d.ts; declare the ones this app
// uses so svelte-check resolves them. AuthProvider renders one configured
// provider (oauth / magic-link / password) and owns its signIn (applying the
// provider's scopes).
declare module '@kavach/ui' {
	export const AuthProvider: import('svelte').Component<{
		name: string;
		mode?: 'otp' | 'oauth' | 'password';
		label?: string;
		scopes?: string[];
		class?: string;
		onerror?: (error: { message?: string } | undefined) => void;
		onsuccess?: (data: unknown) => void;
	}>;
}
