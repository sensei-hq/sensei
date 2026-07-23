<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import CheckerChip from '$lib/components/CheckerChip.svelte';
	import { PV_PROJECTS, scopeRank, type ProjectKind, type ScopeId } from '$lib/preview-data';
	import type { RuleStatus } from '$lib/preview-view';
	import { createPreviewStore } from '$lib/preview-state.svelte';

	// The effective-constitution preview (mockup dojo-preview.jsx `DojoRulePreview`).
	// Answers "before I start, what rules govern THIS project?" — it resolves the
	// whole ladder (Company → Client → Personal → Project → Stack) for one project
	// and shows the composed constitution with conflicts already settled.
	// Presentational — the selected project + classification override live in the
	// injected `store` (createPreviewStore, $state); this component only reads
	// getters and calls mutators (select, reclassify, reset). Mobile-first: the
	// ladder and the conflicts stack on phone, two columns at md:+.
	let {
		store = createPreviewStore()
	}: { store?: ReturnType<typeof createPreviewStore> } = $props();

	// Kind → label + tone class, the picker + banner rendering rule (a lookup, not
	// data derivation). Client is accented (the engagement case); company/personal
	// read muted.
	const KIND: Record<ProjectKind, { label: string; toneClass: string }> = {
		company: { label: '社 company', toneClass: 'text-ink-soft' },
		client: { label: '客 client', toneClass: 'text-accent' },
		personal: { label: '己 personal', toneClass: 'text-ink-mute' }
	};

	// Per-rule status → the tag rendering rule (glyph + copy + tone). The three
	// resolved states from preview-view: a hard lock (★), a plain rule (negotiable),
	// and a rule that lost its topic (overridden ↑).
	const STATUS: Record<RuleStatus, { label: string; toneClass: string; strike: boolean }> = {
		'non-negotiable': { label: '★ non-negotiable', toneClass: 'text-warning', strike: false },
		negotiable: { label: 'negotiable', toneClass: 'text-ink-faint', strike: false },
		overridden: { label: 'overridden ↑', toneClass: 'text-warning', strike: true }
	};

	// Scope → short name, for the "overridden ↑ (Company)" annotation on a rule
	// that lost its topic. Keyed by ScopeId (five rungs), distinct from KIND (the
	// three project classifications).
	const SCOPE_NAME: Record<ScopeId, string> = {
		company: 'company',
		client: 'client',
		personal: 'personal',
		project: 'project',
		stack: 'stack'
	};

	// The rung indent (broad → specific), mirroring the mockup's `marginLeft: i*10`.
	// Driven by scope rank so it survives reclassification. Kept as a geometry-only
	// inline style (the design system doesn't model a per-depth indent scale).
	function indent(scope: ScopeId): number {
		return scopeRank(scope) * 10;
	}
</script>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="序"
		eyebrow="Project · effective constitution"
		title="What governs this project"
		sub="The composed constitution for a project — every rule that resolves onto it, down the ladder, with conflicts already settled. See exactly what governs your work before the first commit."
	>
		{#snippet right()}
			<div class="flex flex-col items-end gap-2">
				<DojoChip toneClass="text-ink-mute">場 {store.project.repo}</DojoChip>
				<DojoChip toneClass="text-ink-soft"
					>{store.constitution.totalRules} rules · {store.constitution.lockedCount} locked</DojoChip
				>
			</div>
		{/snippet}
	</ConsoleHead>

	<!-- project picker -->
	<div
		class="border-paper-edge bg-paper-soft flex flex-shrink-0 flex-wrap gap-2 overflow-x-auto border-b"
		style="padding: 12px 16px"
		role="tablist"
		aria-label="Sample projects"
	>
		{#each PV_PROJECTS as p (p.id)}
			{@const on = p.id === store.baseProject.id}
			<button
				type="button"
				role="tab"
				aria-selected={on}
				onclick={() => store.select(p.id)}
				class="inline-flex items-center gap-2 rounded-lg {on
					? 'bg-paper border-accent border'
					: 'border-paper-edge border'}"
				style="padding: 6px 12px; cursor: pointer; font-family: inherit; background: {on
					? ''
					: 'transparent'}"
			>
				<span class="kanji text-base {KIND[p.kind].toneClass}">{p.kanji}</span>
				<span class="mono text-sm {on ? 'text-ink' : 'text-ink-soft'}">{p.name}</span>
				<DojoChip toneClass={KIND[p.kind].toneClass}>{KIND[p.kind].label}</DojoChip>
			</button>
		{/each}
	</div>

	<!-- why this classification + reclassify override -->
	<div
		class="border-paper-edge flex flex-shrink-0 flex-col gap-2 border-b {store.effectiveKind ===
		'client'
			? 'bg-accent-soft'
			: 'bg-paper'}"
		style="padding: 12px 16px"
	>
		<div class="flex items-start gap-2">
			<span
				class="kanji text-base flex-shrink-0 {store.effectiveKind === 'client'
					? 'text-accent'
					: 'text-ink-mute'}">問</span
			>
			<span
				class="text-xs {store.effectiveKind === 'client' ? 'text-ink-soft' : 'text-ink-mute'}"
				style="line-height: 1.5"
			>
				<b class="mono text-ink font-semibold">{store.project.repo}</b> — {store.baseProject.why}
			</span>
		</div>
		<div class="flex flex-wrap items-center gap-2" style="padding-left: 24px">
			<span class="text-ink-mute text-xs">Classified</span>
			<DojoChip toneClass={KIND[store.effectiveKind].toneClass}
				>{KIND[store.effectiveKind].label}</DojoChip
			>
			{#if store.isOverridden}
				<span class="mono text-warning text-xs">overridden</span>
			{/if}
			{#if store.baseProject.dojo}
				<span class="text-ink-faint text-xs">· not right?</span>
				{#each ['client', 'company'] as const as kind (kind)}
					{@const active = store.effectiveKind === kind}
					<button
						type="button"
						onclick={() => store.reclassify(kind)}
						aria-pressed={active}
						class="rounded text-xs {active
							? 'border-accent text-ink border'
							: 'border-paper-edge text-ink-soft border'}"
						style="padding: 2px 8px; cursor: pointer; font-family: inherit; background: {active
							? ''
							: 'transparent'}"
					>
						{kind === 'client' ? 'Client' : 'Company'}
					</button>
				{/each}
				{#if store.isOverridden}
					<button
						type="button"
						onclick={() => store.resetClassification()}
						class="text-ink-faint text-xs underline"
						style="background: none; border: none; cursor: pointer; font-family: inherit">reset</button
					>
				{/if}
			{:else}
				<span class="text-ink-faint text-xs"
					>· bind a Dōjō to govern this as company or client work</span
				>
			{/if}
		</div>
	</div>

	<!-- ladder + conflicts: stacked on phone, two columns at md:+ -->
	<div class="flex min-h-0 flex-1 flex-col overflow-y-auto md:grid md:overflow-hidden"
		style="grid-template-columns: minmax(0, 0.92fr) minmax(0, 1.08fr)">
		<!-- left · the ladder -->
		<div
			class="border-paper-edge min-w-0 overflow-y-auto border-b md:border-b-0 md:border-r"
			style="padding: 16px"
		>
			<div class="flex items-center gap-2" style="margin-bottom: 12px">
				<span
					class="text-ink-mute text-xs font-semibold uppercase"
					style="letter-spacing: 0.14em">The ladder</span
				>
				<span class="mono text-ink-faint text-xs">broad → specific</span>
			</div>
			<div class="flex flex-col gap-2">
				{#each store.constitution.ladder as rung (rung.id)}
					<div
						class="bg-paper-soft border-paper-edge rounded-lg border"
						style="padding: 12px 16px; margin-left: {indent(rung.scope)}px"
					>
						<!-- rung header -->
						<div class="flex flex-wrap items-center gap-2" style="margin-bottom: 8px">
							<span class="kanji text-lg text-ink-mute">{rung.kanji}</span>
							<span class="text-ink text-sm font-semibold">{rung.name}</span>
							<span class="mono text-ink-soft text-xs">{rung.label}</span>
							{#if rung.free}
								<DojoChip toneClass="text-success">free</DojoChip>
							{/if}
							{#if rung.scope === 'personal'}
								<DojoChip toneClass="text-ink-mute">yours · editable</DojoChip>
							{/if}
							<span class="flex-1"></span>
							<span class="mono text-ink-faint text-xs">{rung.caption}</span>
						</div>
						<!-- wired checkers (stack rungs) -->
						{#if rung.checkers.length > 0}
							<div class="flex flex-wrap items-center gap-1" style="margin-bottom: 8px">
								<span class="text-ink-mute text-xs">caught by</span>
								{#each rung.checkers as checker (checker)}
									<CheckerChip name={checker} />
								{/each}
								<span class="text-ink-faint text-xs">→ surfaced in your review lane</span>
							</div>
						{/if}
						<!-- rules -->
						<div class="bg-paper border-paper-edge rounded border" style="padding: 0 12px">
							{#each rung.rules as rule, i (rule.topic + i)}
								{@const meta = STATUS[rule.status]}
								<div
									class="grid items-center gap-3 {i < rung.rules.length - 1
										? 'border-paper-edge border-b'
										: ''}"
									style="grid-template-columns: 1fr auto; padding: 8px 0"
								>
									<div class="flex items-center gap-2" style="min-width: 0">
										<span
											class="text-sm {meta.strike ? 'text-ink-faint line-through' : 'text-ink'}"
											style="line-height: 1.4">{rule.text}</span
										>
										{#if rule.src === 'compliance'}
											<DojoChip toneClass="text-accent">法</DojoChip>
										{/if}
									</div>
									<span class="mono whitespace-nowrap text-xs {meta.toneClass}" style="justify-self: end"
										>{meta.label}{#if rule.status === 'overridden' && rule.overriddenBy}
											<span class="text-ink-faint"> ({SCOPE_NAME[rule.overriddenBy]})</span
											>{/if}</span
									>
								</div>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- right · resolved constitution + conflicts -->
		<div class="min-w-0 overflow-y-auto" style="padding: 16px">
			<!-- conflicts, resolved -->
			<div class="flex items-center gap-2" style="margin-bottom: 12px">
				<span class="kanji text-warning text-sm">衝</span>
				<span class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.14em"
					>Conflicts, resolved</span
				>
				<span class="mono text-ink-faint text-xs">{store.constitution.conflicts.length}</span>
			</div>
			{#if store.constitution.conflicts.length === 0}
				<div
					class="bg-paper-soft border-paper-edge text-ink-mute rounded-lg border text-xs"
					style="padding: 12px 16px; margin-bottom: 24px; line-height: 1.5"
				>
					Nothing to settle — no rule on this project relaxes or refines another. Every rule below
					simply applies.
				</div>
			{:else}
				<div class="flex flex-col gap-2" style="margin-bottom: 24px">
					{#each store.constitution.conflicts as conflict (conflict.topic)}
						<div
							class="bg-paper-soft border-paper-edge border-l-warning rounded-lg border"
							style="border-left-width: 3px; padding: 12px 16px"
						>
							<div class="flex flex-wrap items-center gap-2" style="margin-bottom: 4px">
								<span class="text-ink text-sm font-semibold">{conflict.winner}</span>
								<span class="flex-1"></span>
								<span
									class="text-success bg-success-soft border-success-edge inline-flex items-center gap-1 rounded-full border text-xs"
									style="padding: 2px 8px"
								>
									{conflict.winnerScope} wins{#if conflict.locked}
										· locked{/if}
								</span>
							</div>
							<div class="text-ink-faint text-xs" style="margin-bottom: 4px">
								over <span class="line-through">{conflict.lost}</span>
							</div>
							<div class="text-ink-mute text-xs" style="line-height: 1.5">{conflict.why}</div>
						</div>
					{/each}
				</div>
			{/if}

			<!-- how it resolves -->
			<div
				class="bg-paper-soft border-paper-edge flex flex-col gap-1 rounded-lg border"
				style="padding: 16px; margin-bottom: 24px"
			>
				<div class="flex items-center gap-2" style="margin-bottom: 4px">
					<span class="kanji text-accent text-sm">序</span>
					<span class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.14em"
						>How it resolves</span
					>
				</div>
				{#each ['Everything sensei records is derived and anonymous, and stays on your machine — so classification changes which rules apply, never what leaves.', 'A non-negotiable (★) locks — no narrower scope can relax it.', 'Otherwise the more specific scope refines the broader (stack → project → personal → client → company).'] as line, i (i)}
					<div class="text-ink-soft flex gap-2 text-xs" style="line-height: 1.5">
						<span class="mono text-ink-faint flex-shrink-0">{i + 1}</span>{line}
					</div>
				{/each}
			</div>

			<!-- summary card (dark) -->
			<div class="bg-ink rounded-lg" style="padding: 16px">
				<div class="flex items-baseline gap-2" style="margin-bottom: 8px">
					<span class="display text-on-primary text-3xl font-light" style="line-height: 1"
						>{store.constitution.totalRules}</span
					>
					<span class="text-on-primary/70 text-sm">rules govern this project on day one</span>
				</div>
				<div class="flex flex-wrap gap-2" style="margin-bottom: 12px">
					<span
						class="text-warning bg-on-primary/10 inline-flex items-center gap-1 rounded-full text-xs"
						style="padding: 2px 8px">★ {store.constitution.lockedCount} non-negotiable</span
					>
					<span
						class="mono text-accent bg-on-primary/10 rounded-full text-xs"
						style="padding: 2px 8px">盾 derived · anonymous · stays on your machine</span
					>
					<span
						class="mono text-on-primary/50 bg-on-primary/10 rounded-full text-xs"
						style="padding: 2px 8px">{store.constitution.scopeCount} scopes composed</span
					>
				</div>
				<div class="text-on-primary/70 text-xs" style="line-height: 1.55">
					Inherited automatically — nothing to copy in. Sensei enforces the locked rules, the
					reviewers catch style and complexity mechanically, and the rest guide as you go.
					<span style="font-style: italic">Still listening.</span>
				</div>
				<div
					class="text-on-primary/50 border-t-on-primary/10 border-t text-xs"
					style="margin-top: 12px; padding-top: 12px; line-height: 1.5"
				>
					This is the same view a teammate sees when they join a Dōjō — here's what you'll follow.
				</div>
			</div>
		</div>
	</div>
</div>
