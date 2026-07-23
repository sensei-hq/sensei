<script lang="ts">
	import { resolve } from '$app/paths';
	import { personalGreeting, soloIdentity, LIBRARY_HREF, type PersonalUser } from '$lib/personal-home-view';

	// The solo home (DJ1, mockup dojo-saas.jsx `DojoOrgsEmpty`). The landing a
	// signed-in user with NO Dōjō membership sees. Joining a Dōjō is optional,
	// never a gate — the user works fully solo. Blocks, top to bottom:
	//   · greeting — you're already working, no Dōjō needed
	//   · needs you — honest-empty placeholder (the cloud Dōjō can't see local
	//     approvals/gates; the desktop app surfaces those on this machine)
	//   · your projects — honest-empty (the cloud Worker can't see local FS
	//     projects — no fabricated rows; the honest copy says where they live)
	//   · your own rules · optional — links to the constitution library
	//   · create or join a Dōjō · optional — clearly secondary
	// Presentational only: `user` in, no fetch, no live tenant. Mobile-first —
	// grids collapse to one column below md:. Voice: calm, lowercase "sensei".
	let { user }: { user?: PersonalUser } = $props();

	const identity = $derived(soloIdentity(user));
	const greeting = $derived(personalGreeting(user));
</script>

<svelte:head>
	<title>Your work · Dōjō</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-y-auto">
	<div class="mx-auto w-full" style="max-width: 860px; padding: 32px 16px">
		<!-- greeting — you're already working, no Dōjō needed -->
		<div class="flex items-center gap-3">
			<span
				class="bg-accent-soft text-accent flex flex-shrink-0 items-center justify-center rounded-full text-xs font-semibold"
				style="width: 32px; height: 32px"
				aria-hidden="true">{identity.initials}</span
			>
			<div class="text-ink-mute text-xs uppercase" style="letter-spacing: 0.18em">{greeting}</div>
		</div>
		<h1 class="display text-ink text-2xl font-light" style="letter-spacing: -0.02em; margin: 12px 0 0; line-height: 1.1">
			your work, here.
		</h1>
		<p class="text-ink-soft text-sm" style="line-height: 1.6; margin: 8px 0 0; max-width: 560px">
			sensei watches your projects on your own machine — you don't need a Dōjō to work. a Dōjō is
			optional, for when you want to share what you learn with a team.
		</p>

		<!-- needs you — honest empty (surfaced on the desktop app, on this machine) -->
		<div class="flex items-center gap-2" style="margin-top: 32px; margin-bottom: 12px">
			<span class="kanji text-accent text-sm">要</span>
			<span class="text-ink-mute font-semibold uppercase text-xs" style="letter-spacing: 0.14em">needs you</span>
		</div>
		<div
			class="bg-paper-soft border-paper-edge text-ink-mute rounded-xl border text-sm"
			style="padding: 16px 16px; line-height: 1.55"
		>
			nothing here is waiting on you right now. approvals and decisions from your running tasks
			surface in the sensei desktop app, on this machine.
		</div>

		<!-- your projects — honest empty (the cloud Dōjō can't see local projects) -->
		<div class="flex items-center gap-2" style="margin-top: 24px; margin-bottom: 12px">
			<span class="kanji text-ink-mute text-sm">場</span>
			<span class="text-ink-mute font-semibold uppercase text-xs" style="letter-spacing: 0.14em">your projects</span>
			<span class="flex-1"></span>
			<span class="mono text-ink-faint text-xs">local · this machine</span>
		</div>
		<div
			class="bg-paper-soft border-paper-edge text-ink-mute rounded-xl border text-sm"
			style="padding: 16px 16px; line-height: 1.55"
		>
			your projects live on your own machine, watched by sensei locally. open the sensei desktop
			app to see what's running — this web Dōjō only shows work you choose to share with a team.
		</div>

		<!-- your own rules · optional — solo governance, no Dōjō needed -->
		<div class="flex items-center gap-2" style="margin-top: 24px; margin-bottom: 12px">
			<span class="kanji text-ink-mute text-sm">己</span>
			<span class="text-ink-mute font-semibold uppercase text-xs" style="letter-spacing: 0.14em"
				>your own rules · optional</span
			>
		</div>
		<div
			class="bg-paper flex flex-col items-start gap-3 rounded-xl border border-dashed md:flex-row md:items-center border-ink-faint"
			style="padding: 16px 16px"
		>
			<span class="kanji text-accent text-xl flex-shrink-0">典</span>
			<div class="flex-1" style="min-width: 0">
				<div class="text-ink text-sm">seed your personal constitution</div>
				<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
					even solo, you can give sensei a constitution for your own projects — pull proven rules
					from the library. no Dōjō required.
				</div>
			</div>
			<a
				href={LIBRARY_HREF}
				class="border-paper-edge text-ink inline-flex flex-shrink-0 items-center gap-2 rounded-lg border text-sm no-underline"
				style="padding: 8px 16px"
			>
				open library
			</a>
		</div>

		<!-- create or join a Dōjō · optional — clearly secondary -->
		<div class="flex items-center gap-2" style="margin-top: 24px; margin-bottom: 12px">
			<span class="kanji text-ink-mute text-sm">結</span>
			<span class="text-ink-mute font-semibold uppercase text-xs" style="letter-spacing: 0.14em"
				>create or join a Dōjō · optional</span
			>
		</div>
		<p class="text-ink-mute text-sm" style="line-height: 1.55; margin: 0 0 12px; max-width: 560px">
			a Dōjō is a shared mind for a team. you can keep working solo as long as you like — step into
			one only when there's someone to share with.
		</p>
		<div class="grid gap-3 md:grid-cols-2">
			<div
				class="bg-paper flex items-center gap-3 rounded-xl border border-dashed border-ink-faint"
				style="padding: 16px 16px"
			>
				<span class="kanji text-accent text-xl flex-shrink-0">開</span>
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm">create a Dōjō</div>
					<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
						start one for your team. you become its first steward.
					</div>
				</div>
				<a
					href={resolve('/orgs')}
					class="border-paper-edge text-ink inline-flex flex-shrink-0 items-center rounded-lg border text-sm no-underline"
					style="padding: 8px 16px">create</a
				>
			</div>
			<div
				class="bg-paper flex items-center gap-3 rounded-xl border border-dashed border-ink-faint"
				style="padding: 16px 16px"
			>
				<span class="kanji text-ink-mute text-xl flex-shrink-0">迎</span>
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm">join a Dōjō</div>
					<div class="text-ink-mute text-xs" style="margin-top: 4px; line-height: 1.5">
						github orgs appear on their own. have an invite code?
					</div>
				</div>
				<a
					href={resolve('/orgs')}
					class="border-paper-edge text-ink inline-flex flex-shrink-0 items-center rounded-lg border text-sm no-underline"
					style="padding: 8px 16px">join</a
				>
			</div>
		</div>

		<div
			class="text-ink-mute flex items-start gap-2 text-xs"
			style="margin-top: 24px; line-height: 1.5; max-width: 620px"
		>
			<span class="kanji text-accent text-sm flex-shrink-0">基</span>
			<span
				>everything stays on your machine until you choose to share it. when you join or create a
				Dōjō, you pick exactly what leaves.</span
			>
		</div>
	</div>
</div>
