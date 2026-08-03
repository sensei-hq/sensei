<script lang="ts">
	import { kindToneClass, type DojoOrg } from '$lib/dojo-data';
	import { Chip } from '$lib/components/kit';

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
		class="border-paper-edge bg-paper flex h-[54px] flex-shrink-0 items-center gap-3 border-b px-4 md:px-6"
	>
		<span class="kanji text-accent text-xl" style="line-height: 1">結</span>
		<span class="display text-lg" style="letter-spacing: -0.01em">Dōjō</span>
		<span class="mono text-ink-faint hidden text-xs sm:inline">dojo.sensei-hq.com</span>
		<span class="flex-1"></span>
		<span
			class="bg-accent-soft text-accent flex h-[28px] w-[28px] items-center justify-center rounded-full text-xs font-semibold"
			aria-hidden="true">{user.initials}</span
		>
		<span class="text-ink-soft hidden text-sm sm:inline">{user.name}</span>
		<button type="button" class="mono text-ink-mute cursor-pointer border-none bg-transparent text-xs"
			>sign out</button
		>
	</div>

	<div class="flex-1 overflow-auto py-8">
		<div class="mx-auto max-w-[820px] px-4 md:px-8">
			<div class="text-ink-mute mb-2 text-xs uppercase" style="letter-spacing: 0.2em">
				Signed in as {user.handle}
			</div>
			<h1 class="display m-0 font-light text-2xl" style="letter-spacing: -0.02em; line-height: 1.1">
				Your organizations
			</h1>
			<p class="text-ink-soft mb-6 mt-2 max-w-[560px] text-sm" style="line-height: 1.55">
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
					<!--
						Org card. Phone (<md): 2-col grid (icon | body) with the action block
						wrapping to a full-width row below (col-span-2). md:+: the original
						3-col split (icon | body | actions).
					-->
					<div
						class="bg-paper-soft grid grid-cols-[auto_1fr] items-center gap-4 rounded-2xl p-4 md:grid-cols-[auto_1fr_auto] md:gap-5 {o.last
							? 'border-accent'
							: 'border-paper-edge'} border"
					>
						<div
							class="bg-paper border-paper-edge flex h-[46px] w-[46px] items-center justify-center rounded-xl border"
						>
							<span class="kanji text-accent text-xl">{o.kanji}</span>
						</div>
						<div class="min-w-0">
							<div class="flex flex-wrap items-center gap-2">
								<span class="display text-lg">{o.name}</span>
								<Chip toneClass={kindToneClass[o.kind]}>{o.kind}</Chip>
								{#if o.last}
									<Chip toneClass="text-accent">last opened</Chip>
								{/if}
							</div>
							<div class="mt-2 flex flex-wrap items-center gap-3">
								<span class="mono text-ink-soft text-xs">
									{#if o.host === 'self'}
										<span class="text-ink-faint">self-hosted · </span>{o.url}
									{:else}
										<span class="text-ink-faint">sensei-hq.com/</span>{o.url}
									{/if}
								</span>
								<Chip toneClass={o.host === 'self' ? 'text-ink-soft' : 'text-ink-mute'}>
									{o.host === 'self' ? '基 self-hosted' : '雲 SaaS'}
								</Chip>
							</div>
						</div>
						<div
							class="border-t-paper-edge col-span-2 mt-2 flex flex-wrap items-center justify-end gap-5 border-t pt-3 md:col-span-1 md:mt-0 md:border-t-0 md:pt-0"
						>
							<div class="text-right">
								<div class="text-ink text-sm">{o.role}</div>
								<div class="mono text-ink-faint mt-1 text-xs">{o.from}</div>
							</div>
							<div class="min-w-[64px] text-right">
								{#if o.members != null}
									<div class="mono text-ink-soft text-xs">
										{o.members}
										{o.members === 1 ? 'member' : 'members'}
									</div>
								{/if}
								{#if o.pending != null}
									{#if o.pending > 0}
										<div class="mono text-accent mt-1 text-xs">
											{o.pending} to triage
										</div>
									{:else}
										<div class="mono text-ink-faint mt-1 text-xs">up to date</div>
									{/if}
								{/if}
							</div>
							<button
								type="button"
								onclick={() => onEnter?.(o)}
								class="inline-flex cursor-pointer items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium {o.last
									? 'bg-primary text-on-primary'
									: 'bg-paper border-paper-edge text-ink border'}"
							>
								Enter <span class="text-xs">→</span>
							</button>
						</div>
					</div>
				{/each}
			</div>

			<!-- join another -->
			<div
				class="bg-paper border-ink-faint mt-5 flex flex-col gap-4 rounded-2xl border border-dashed p-4 md:flex-row md:items-center"
			>
				<span class="kanji text-ink-mute text-xl">迎</span>
				<div class="min-w-0 flex-1">
					<div class="text-ink text-sm">Join another organization</div>
					<div class="text-ink-mute mt-1 text-xs">
						New GitHub orgs you join appear here automatically. Have an invite code for a client or
						non-GitHub org?
					</div>
				</div>
				<div class="flex gap-2">
					<input
						type="text"
						placeholder="invite code"
						class="bg-paper border-paper-edge text-ink box-border w-full rounded-lg border px-3 py-3 text-sm md:w-[170px]"
					/>
					<button
						type="button"
						class="bg-paper border-paper-edge text-ink cursor-pointer whitespace-nowrap rounded-lg border px-4 py-3 text-sm"
						>Join</button
					>
				</div>
			</div>

			<div class="text-ink-mute mt-6 flex max-w-[640px] items-start gap-2 text-xs" style="line-height: 1.5">
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
