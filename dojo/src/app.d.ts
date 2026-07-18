// See https://svelte.dev/docs/kit/types#app.d.ts

import type { Session } from '@supabase/supabase-js';

declare global {
	namespace App {
		interface Locals {
			// Populated by the kavach handle in hooks.server.ts from the session
			// cookie. Null when unauthenticated.
			session: Session | null;
		}

		// The Cloudflare Workers platform (adapter-cloudflare). `context.waitUntil`
		// lets a route run background work (the relay P4.4 push send) without
		// blocking / failing the response — used at the relay/inbox trigger site.
		// Undefined outside the Worker (local `bun run dev` / vitest), so callers
		// guard on it. Only the fields we use are declared.
		interface Platform {
			context?: { waitUntil(promise: Promise<unknown>): void };
		}
	}

	// @rokkit/states ships JS source; its `types` field points at an unpublished
	// dist/, so it resolves untyped here. Shim until rokkit ships declarations.
	// (Declared inside `global` to stay ambient alongside the App namespace.)
}

// @rokkit/states resolves untyped — shim until rokkit ships declarations.
declare module '@rokkit/states';

export {};
