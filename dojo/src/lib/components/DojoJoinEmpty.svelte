<script lang="ts">
	import { resolve } from '$app/paths';

	// The shared "join or create a Dōjō" empty state (DJ1). Every org-scoped
	// console screen (members, identities, policies, triage, engagements,
	// incidents, health, audit, relay) renders this when the caller has no
	// membership (`noMembership` from the guarded load) instead of a fabricated
	// tenant's data. Presentational: it takes the screen's name so the copy reads
	// naturally, and links back to the org picker where the caller creates or
	// joins a Dōjō. Voice: calm, lowercase "sensei", no emoji, sentence case.
	let { what = 'this' }: { what?: string } = $props();
</script>

<div class="flex flex-1 items-center justify-center" style="padding: 24px">
	<div
		class="bg-paper-soft border-paper-edge flex flex-col items-center rounded-xl border text-center"
		style="padding: 32px 24px; max-width: 460px"
	>
		<span class="kanji text-accent text-3xl" style="line-height: 1">結</span>
		<h2 class="display text-ink text-lg font-normal" style="letter-spacing: -0.01em; margin: 16px 0 0">
			{what} lives inside a Dōjō
		</h2>
		<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 8px 0 0">
			a Dōjō is a shared mind — where a team's sessions consolidate into knowledge everyone draws on.
			join or create one to see {what} for your team. you can keep working solo as long as you like.
		</p>
		<a
			href={resolve('/orgs')}
			class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-sm font-medium no-underline"
			style="margin-top: 20px; padding: 8px 16px"
		>
			<span class="kanji text-xs">迎</span>
			join or create a Dōjō
		</a>
	</div>
</div>
