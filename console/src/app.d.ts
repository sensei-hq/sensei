// See https://svelte.dev/docs/kit/types#app.d.ts

import type { Session } from '@supabase/supabase-js';

declare global {
	namespace App {
		interface Locals {
			// Populated by the kavach handle in hooks.server.ts from the session
			// cookie. Null when unauthenticated.
			session: Session | null;
		}
	}

	// @rokkit/states ships JS source; its `types` field points at an unpublished
	// dist/, so it resolves untyped here. Shim until rokkit ships declarations.
	// (Declared inside `global` to stay ambient alongside the App namespace.)
}

// @rokkit/states resolves untyped — shim until rokkit ships declarations.
declare module '@rokkit/states';

export {};
