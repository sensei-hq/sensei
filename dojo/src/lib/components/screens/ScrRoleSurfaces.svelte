<script lang="ts">
	import {
		SectionHead,
		Btn,
		Chip,
		Icon,
		Avatar,
		RoleTag,
		KanjiToken
	} from '$lib/components/kit';
	import type { KitMember, KitRolePolicy, KitChatTurn } from '$lib/components/kit/types';
	import { ROLE_SURFACE_TABS, resolveTab } from '$lib/role-surfaces-view';

	// The admin Members & Roles / Policies / Audit surface (mockup
	// ScrRoleSurfaces) — ONE screen the org nav shares between two ids: the
	// `members` item opens the Members tab, the `audit` item opens the Audit tab.
	// The Members tab is a table of git-derived roles + dōjō overrides (RoleTag);
	// Policies is the additive role-policy grid; Audit is a compact read of the
	// governance log. Presentational: the page supplies the tab (from the
	// section), the members, the role policies and the audit log (kit fixtures
	// this chunk).
	let {
		orgName,
		tab = 'members',
		members = [],
		policies = [],
		audit = [],
		me = 'You',
		onSetRole
	}: {
		orgName: string;
		tab?: string;
		members?: KitMember[];
		policies?: KitRolePolicy[];
		audit?: KitChatTurn[];
		me?: string;
		/** Change a member's role (admin). Called with the row + the chosen role;
		 *  absent = read-only (fixture render). The row must carry a `userId`. */
		onSetRole?: (member: KitMember, role: string) => void;
	} = $props();

	/** The additive role ladder the set-role picker offers (matches
	 *  `dojo.member_role`). */
	const ROLE_OPTIONS = ['contributor', 'maintainer', 'lead', 'admin'];

	// The active tab. The nav shares this one screen between the `members` and
	// `audit` ids, so a members↔audit navigation changes the `tab` prop from the
	// URL (the source of truth) WITHOUT re-mounting (same {#if} branch). An
	// in-screen tab click is an ephemeral override; it wins only while the prop it
	// was made against still holds, so navigating re-follows the URL. A `$derived`
	// (not an effect) settles which wins — no prop→state copy.
	let override = $state<{ against: string; id: string } | null>(null);
	const active = $derived(
		override && override.against === tab ? override.id : resolveTab(tab).id
	);
	const current = $derived(resolveTab(active));

	function select(id: string) {
		override = { against: tab, id: resolveTab(id).id };
	}
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead
		kanji="規" eyebrow={orgName + ' · ' + current.eyebrow} title={current.title}
		description="Roles are additive and derived from git. developer → maintainer → lead → admin. A role only ever adds capability."
	>
		{#snippet right()}
			<Btn size="sm" icon="add-circle">{current.cta}</Btn>
		{/snippet}
	</SectionHead>

	<!-- The three admin surfaces — a segmented tab strip. -->
	<div class="flex flex-wrap gap-2" role="tablist">
		{#each ROLE_SURFACE_TABS as t (t.id)}
			{@const on = active === t.id}
			<button
				type="button"
				role="tab"
				aria-selected={on}
				onclick={() => select(t.id)}
				class="inline-flex cursor-pointer items-center gap-2 rounded border text-sm {on
					? 'bg-paper-soft border-paper-edge text-ink'
					: 'text-ink-mute border-transparent bg-transparent'}"
				style="padding: 8px 12px"
			>
				<Icon name={t.icon} size={15} toneClass={on ? 'text-accent' : 'text-ink-mute'} />{t.label}
			</button>
		{/each}
	</div>

	{#if active === 'members'}
		<!-- Members & Roles — the git-derived role + dōjō-override table. -->
		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
			{#each members as m (m.name)}
				<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
					<Avatar name={m.name} size={30} />
					<div class="flex-1" style="min-width: 0">
						<div class="flex items-center gap-2">
							<span class="text-ink text-sm font-medium">{m.name}</span>
							{#if m.you}<Chip toneClass="text-ink-mute">you</Chip>{/if}
						</div>
						<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
							git: {m.git} · {m.scopes}
						</div>
					</div>
					<span class="text-ink-mute text-xs">{m.active}</span>
					{#if onSetRole && m.userId && !m.you}
						<!-- Change-role picker (admin): a native select keeps the kit lean
						     and stays keyboard/a11y-correct. The viewer's own row stays a
						     read-only tag so an admin can't accidentally demote themselves. -->
						<label class="sr-only" for={'role-' + m.userId}>Role for {m.name}</label>
						<select
							id={'role-' + m.userId}
							class="bg-paper-soft border-paper-edge text-ink rounded border text-xs"
							style="padding: 4px 8px"
							value={m.role}
							onchange={(e) => onSetRole?.(m, (e.currentTarget as HTMLSelectElement).value)}
						>
							{#each ROLE_OPTIONS as r (r)}
								<option value={r}>{r}</option>
							{/each}
						</select>
					{:else}
						<RoleTag role={m.role} />
					{/if}
					<Icon name="alt-arrow-right" size={16} toneClass="text-ink-faint" />
				</div>
			{/each}
		</div>
	{:else if active === 'policies'}
		<!-- Policies — the additive role-policy grid. -->
		<div class="flex flex-col gap-3">
			{#each policies as r (r.id)}
				<div
					class="bg-paper-soft border-paper-edge flex items-center gap-4 rounded-lg border"
					style="padding: 16px"
				>
					<KanjiToken char={r.kanji} size="xl" toneClass="text-accent" w={30} />
					<div class="flex-1" style="min-width: 0">
						<div class="text-ink text-sm font-medium">{r.label}</div>
						<div class="text-ink-mute text-xs">{r.note}</div>
					</div>
					<Btn size="sm" variant="ghost" icon="tuning-2">Edit policy</Btn>
				</div>
			{/each}
		</div>
	{:else}
		<!-- Audit — a compact read of the governance log. -->
		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
			{#each audit as x, i (i)}
				<div class="border-paper-edge flex items-center gap-3 border-b" style="padding: 12px 16px">
					<Icon name="clipboard-list" size={17} toneClass="text-ink-mute" />
					<span class="text-ink flex-1 text-sm">
						{x.who === 'sensei' ? 'sensei' : me} · {x.text.slice(0, 60)}…
					</span>
					<span class="mono text-ink-faint text-xs">{x.when}</span>
				</div>
			{/each}
		</div>
	{/if}
</div>
