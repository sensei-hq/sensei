<script lang="ts">
	// Placeholder for the guarded console group. The sentry rule
	// `{ path: '/console', roles: '*' }` in kavach.config.js gates this behind an
	// authenticated session (unauthenticated → redirected to /signin). The real
	// admin / maintainer / client-lead console screens land here in R9–R11.
	let { data } = $props();
</script>

<svelte:head>
	<title>Console · Dōjō</title>
</svelte:head>

<div class="bg-paper flex min-h-screen flex-col items-center justify-center" style="padding: 40px">
	<div class="text-center" style="max-width: 480px">
		<span class="kanji text-accent" style="font-size: 40px">道</span>
		<h1 class="display font-light" style="font-size: 28px; letter-spacing: -0.02em; margin: 16px 0 0">
			Console
		</h1>
		<p class="text-ink-mute text-sm" style="line-height: 1.55; margin: 8px 0 0">
			You're inside the guarded console group. The admin, maintainer and client-lead consoles land
			here in a later chunk.
		</p>
		{#if data?.user}
			<p class="mono text-ink-faint text-xs" style="margin-top: 16px">
				signed in as {data.user.email}
			</p>
		{/if}
	</div>
</div>
