<script lang="ts">
	import { resolve } from '$app/paths';

	// Left nav (mockup DojoNav): grouped destinations with a kanji glyph. In R9
	// only Overview + Triage are wired; the rest render as "soon" (non-interactive)
	// per the mockup's DOJO_BUILT gating. `active` is the current section id.
	let { active, tenantKey }: { active: string; tenantKey: string } = $props();

	// `to` marks a built (linked) destination in R9; absent → a "soon" placeholder.
	interface NavItem {
		id: string;
		kanji: string;
		label: string;
		to?: 'overview' | 'triage';
		badge?: number;
	}
	interface NavGroup {
		group: string;
		items: NavItem[];
	}

	// Only Overview + Triage are wired in R9; the rest stay "soon". The href is
	// built inline in the template with resolve().
	const groups = $derived<NavGroup[]>([
		{
			group: 'Govern',
			items: [
				{ id: 'overview', kanji: '全', label: 'Overview', to: 'overview' },
				{ id: 'triage', kanji: '門', label: 'Triage', to: 'triage' },
				{ id: 'knowledge', kanji: '蔵', label: 'Knowledge' }
			]
		},
		{
			group: 'Org',
			items: [
				{ id: 'members', kanji: '任', label: 'Members & roles' },
				{ id: 'scopes', kanji: '規', label: 'Scopes & policies' },
				{ id: 'clients', kanji: '守', label: 'Clients' }
			]
		},
		{
			group: 'Trust',
			items: [
				{ id: 'monitor', kanji: '観', label: 'Monitor' },
				{ id: 'audit', kanji: '録', label: 'Audit trail' }
			]
		}
	]);
</script>

<aside
	class="border-paper-edge bg-paper-soft flex flex-shrink-0 flex-col overflow-auto border-r"
	style="width: 218px; padding: 16px 12px"
>
	{#each groups as grp (grp.group)}
		<div style="margin-bottom: 14px">
			<div
				class="text-ink-faint font-semibold uppercase"
				style="font-size: 9.5px; letter-spacing: 0.14em; padding: 0 8px; margin-bottom: 6px"
			>
				{grp.group}
			</div>
			<div class="flex flex-col gap-1">
				{#each grp.items as it (it.id)}
					{@const on = active === it.id}
					{#if it.to}
						<a
							href={resolve(it.to === 'triage' ? '/(console)/console/triage' : '/(console)/console')}
							aria-current={on ? 'page' : undefined}
							class="grid w-full items-center gap-2 rounded-lg no-underline {on
								? 'bg-paper border-paper-edge text-ink border'
								: 'text-ink-soft border border-transparent'}"
							style="grid-template-columns: auto 1fr auto; text-align: left; padding: 8px 9px; font-size: 13px"
						>
							<span
								class="kanji text-center {on ? 'text-accent' : 'text-ink-mute'}"
								style="font-size: 13px; width: 15px">{it.kanji}</span
							>
							<span>{it.label}</span>
							{#if it.badge != null}
								<span
									class="mono bg-accent text-on-primary rounded-full font-semibold"
									style="font-size: 10px; padding: 0 6px; line-height: 16px">{it.badge}</span
								>
							{:else}
								<span></span>
							{/if}
						</a>
					{:else}
						<div
							title="Designed in a later pass"
							class="text-ink-faint grid w-full items-center gap-2 rounded-lg border border-transparent"
							style="grid-template-columns: auto 1fr auto; text-align: left; padding: 8px 9px; font-size: 13px; opacity: 0.6"
						>
							<span class="kanji text-ink-mute text-center" style="font-size: 13px; width: 15px">{it.kanji}</span>
							<span>{it.label}</span>
							<span class="text-ink-faint" style="font-size: 8.5px; letter-spacing: 0.06em">soon</span>
						</div>
					{/if}
				{/each}
			</div>
		</div>
	{/each}

	<div class="flex-1"></div>
	<div
		class="border-paper-edge text-ink-mute grid w-full items-center gap-2 border-t"
		style="grid-template-columns: auto 1fr; text-align: left; padding: 12px 9px 0; font-size: 13px; opacity: 0.6"
	>
		<span class="kanji text-ink-mute text-center" style="font-size: 13px; width: 15px">調</span>
		<span>Settings · SSO</span>
	</div>
</aside>
