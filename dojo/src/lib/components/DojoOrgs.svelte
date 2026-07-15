<script lang="ts">
	import { kindToneClass, type DojoOrg } from '$lib/dojo-data';
	import DojoChip from './DojoChip.svelte';

	// user + orgs come from the page's server load (the signed-in Supabase user and
	// their real `dojo.memberships` → tenants) — no longer hardcoded mock data.
	type OrgUser = { name: string; handle: string; initials: string };
	let {
		user,
		orgs,
		onEnter
	}: { user: OrgUser; orgs: DojoOrg[]; onEnter?: (org: DojoOrg) => void } = $props();
</script>

<div class="bg-paper flex h-screen w-full flex-col overflow-hidden">
	<!-- top bar -->
	<div
		class="border-paper-edge bg-paper flex flex-shrink-0 items-center gap-3 border-b"
		style="height: 54px; padding: 0 22px"
	>
		<span class="kanji text-accent" style="font-size: 22px; line-height: 1">結</span>
		<span class="display text-lg" style="letter-spacing: -0.01em">Dōjō</span>
		<span class="mono text-ink-faint" style="font-size: 10.5px">dojo.sensei-hq.com</span>
		<span class="flex-1"></span>
		<span
			class="bg-accent-soft text-accent flex items-center justify-center rounded-full text-xs font-semibold"
			style="width: 28px; height: 28px"
			aria-hidden="true">{user.initials}</span
		>
		<span class="text-ink-soft text-sm">{user.name}</span>
		<button type="button" class="mono text-ink-mute text-xs" style="background: none; border: none; cursor: pointer"
			>sign out</button
		>
	</div>

	<div class="flex-1 overflow-auto" style="padding: 36px 0">
		<div style="max-width: 820px; margin: 0 auto; padding: 0 32px">
			<div class="text-ink-mute text-xs uppercase" style="letter-spacing: 0.2em; margin-bottom: 8px">
				Signed in as {user.handle}
			</div>
			<h1 class="display font-light" style="font-size: 32px; letter-spacing: -0.02em; margin: 0; line-height: 1.1">
				Your organizations
			</h1>
			<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 8px 0 26px; max-width: 560px">
				{#if orgs.length === 0}
					You don't belong to any Dōjōs yet. Join one with an invite code below, or ask an
					administrator to add you.
				{:else}
					You belong to {orgs.length}
					{orgs.length === 1 ? 'Dōjō' : 'Dōjōs'}. Your role in each comes from your membership; pick
					one to open its console.
				{/if}
			</p>

			<!-- org cards -->
			<div class="flex flex-col gap-3">
				{#each orgs as o (o.id)}
					<div
						class="bg-paper-soft grid items-center gap-5 rounded-2xl {o.last
							? 'border-accent'
							: 'border-paper-edge'} border"
						style="grid-template-columns: auto 1fr auto; padding: 16px 20px"
					>
						<div
							class="bg-paper border-paper-edge flex items-center justify-center rounded-xl border"
							style="width: 46px; height: 46px"
						>
							<span class="kanji text-accent" style="font-size: 24px">{o.kanji}</span>
						</div>
						<div style="min-width: 0">
							<div class="flex flex-wrap items-center gap-2">
								<span class="display text-lg">{o.name}</span>
								<DojoChip toneClass={kindToneClass[o.kind]}>{o.kind}</DojoChip>
								{#if o.last}
									<DojoChip toneClass="text-accent">last opened</DojoChip>
								{/if}
							</div>
							<div class="flex flex-wrap items-center gap-3" style="margin-top: 7px">
								<span class="mono text-ink-soft" style="font-size: 11.5px">
									{#if o.host === 'self'}
										<span class="text-ink-faint">self-hosted · </span>{o.url}
									{:else}
										<span class="text-ink-faint">sensei-hq.com/</span>{o.url}
									{/if}
								</span>
								<DojoChip toneClass={o.host === 'self' ? 'text-ink-soft' : 'text-ink-mute'}>
									{o.host === 'self' ? '基 self-hosted' : '雲 SaaS'}
								</DojoChip>
							</div>
						</div>
						<div class="flex items-center" style="gap: 22px">
							<div class="text-right">
								<div class="text-ink text-sm">{o.role}</div>
								<div class="mono text-ink-faint" style="font-size: 10px; margin-top: 2px">{o.from}</div>
							</div>
							<div class="text-right" style="min-width: 64px">
								<div class="mono text-ink-soft" style="font-size: 11.5px">
									{o.members}
									{o.members === 1 ? 'member' : 'members'}
								</div>
								{#if o.pending > 0}
									<div class="mono text-accent" style="font-size: 10.5px; margin-top: 3px">
										{o.pending} to triage
									</div>
								{:else}
									<div class="mono text-ink-faint" style="font-size: 10.5px; margin-top: 3px">up to date</div>
								{/if}
							</div>
							<button
								type="button"
								onclick={() => onEnter?.(o)}
								class="inline-flex items-center gap-2 rounded-lg text-sm font-medium {o.last
									? 'bg-primary text-on-primary'
									: 'bg-paper border-paper-edge text-ink border'}"
								style="padding: 10px 18px; cursor: pointer"
							>
								Enter <span style="font-size: 12px">→</span>
							</button>
						</div>
					</div>
				{/each}
			</div>

			<!-- join another -->
			<div
				class="bg-paper border-ink-faint flex items-center gap-4 rounded-2xl border border-dashed"
				style="margin-top: 18px; padding: 16px 20px"
			>
				<span class="kanji text-ink-mute" style="font-size: 20px">迎</span>
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm">Join another organization</div>
					<div class="text-ink-mute text-xs" style="margin-top: 2px">
						New GitHub orgs you join appear here automatically. Have an invite code for a client or
						non-GitHub org?
					</div>
				</div>
				<input
					type="text"
					placeholder="invite code"
					class="bg-paper border-paper-edge text-ink rounded-lg border text-sm"
					style="width: 170px; box-sizing: border-box; padding: 11px 13px"
				/>
				<button
					type="button"
					class="bg-paper border-paper-edge text-ink rounded-lg border text-sm"
					style="padding: 11px 18px; white-space: nowrap; cursor: pointer">Join</button
				>
			</div>

			<div class="text-ink-mute flex items-start gap-2 text-xs" style="margin-top: 22px; line-height: 1.5; max-width: 640px">
				<span class="kanji text-accent text-sm">鍵</span>
				<span
					>Roles are derived from your GitHub org &amp; repo access, then refined inside each Dōjō.
					Organizations not on GitHub are namespaced
					<b class="text-ink-soft font-semibold">other/&lt;org&gt;</b> and sign in by magic link; self-hosted
					Dōjōs keep their own URL.</span
				>
			</div>
		</div>
	</div>
</div>
