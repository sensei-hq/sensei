<script lang="ts">
	// Layout for the guarded `(console)` route group. Route protection itself is
	// enforced server-side by the kavach handle (hooks.server.ts) via the
	// `/console` rule in kavach.config.js — this layout is just the shared shell
	// the R9–R11 console screens will render into.
	let { children } = $props();
</script>

{@render children()}
