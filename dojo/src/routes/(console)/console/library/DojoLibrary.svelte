<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import CheckerChip from '$lib/components/CheckerChip.svelte';
	import LevelPills from '$lib/components/LevelPills.svelte';
	import { LIB_CATS, packsForArea, type LibAreaId, type LibLevelId } from '$lib/library-data';
	import { createLibraryStore } from '$lib/library-state.svelte';
	import { levelLabel } from '$lib/library-view';

	// The full constitution library browser (mockup dojo-library.jsx `DojoLibrary`).
	// Adopt proven governance rules INTO the constitution by area: pick rules, set
	// the level they apply at, mark the non-negotiables (★), and add your own.
	// Presentational — the working selection lives in the injected `store`
	// (createLibraryStore, $state); this component only reads getters and calls
	// mutators. `scopeName` is the target constitution the footer adds to.
	// Mobile-first: the areas rail is a horizontal chip strip below md:, a fixed
	// left column at md:+; the pack grid is one column on phone.
	let {
		store = createLibraryStore(),
		scopeName = 'this Dōjō'
	}: { store?: ReturnType<typeof createLibraryStore>; scopeName?: string } = $props();

	// The selected area (left rail). Core principles first, as in the mockup.
	let area = $state<LibAreaId>('core');

	// The "write your own rule" draft — free text classified under the current
	// area, at a level, optionally non-negotiable.
	let draft = $state('');
	let draftLevel = $state<LibLevelId>('org');
	let draftHard = $state(false);

	const areaMeta = $derived(LIB_CATS.find((c) => c.id === area) ?? LIB_CATS[0]);
	const packs = $derived(packsForArea(area));
	const authoredHere = $derived(store.authoredInArea(area));

	function addDraft() {
		if (store.addAuthored(draft, area, draftLevel, draftHard)) {
			draft = '';
			draftHard = false;
		}
	}
</script>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="蔵"
		eyebrow="Govern · start from proven rules"
		title="Constitution library"
		sub="Pull principles, patterns, compliance controls, stack reviewers and design guardrails straight into the constitution — prevention is cheaper than rework. Cherry-pick rules, set the level they apply at, and mark the ones that are non-negotiable."
	>
		{#snippet right()}
			<DojoChip toneClass="text-ink-soft">→ {scopeName}</DojoChip>
		{/snippet}
	</ConsoleHead>

	<div class="flex min-h-0 flex-1 flex-col md:flex-row">
		<!-- areas rail — horizontal chip strip on phone, fixed left column at md:+ -->
		<div
			class="border-paper-edge bg-paper-soft flex flex-shrink-0 flex-wrap gap-1 overflow-x-auto border-b md:w-[250px] md:flex-col md:flex-nowrap md:gap-1 md:overflow-y-auto md:border-b-0 md:border-r"
			style="padding: 8px 12px"
			role="tablist"
			aria-label="Areas"
		>
			<div
				class="text-ink-faint hidden w-full font-semibold uppercase md:block text-xs"
				style="letter-spacing: 0.14em; padding: 4px 8px 6px"
			>
				Areas
			</div>
			{#each LIB_CATS as cat (cat.id)}
				{@const on = cat.id === area}
				<button
					type="button"
					role="tab"
					aria-selected={on}
					onclick={() => (area = cat.id)}
					class="grid w-auto items-center gap-3 rounded-full text-sm md:w-full md:rounded-lg {on
						? 'bg-ink text-on-primary md:bg-paper md:text-ink border-paper-edge md:border'
						: 'text-ink-soft border-paper-edge border md:border-transparent'}"
					style="grid-template-columns: auto auto; text-align: left; padding: 6px 12px; cursor: pointer; font-family: inherit"
				>
					<span
						class="kanji text-sm text-center {on
							? 'text-on-primary md:text-accent'
							: 'text-accent md:text-ink-mute'}"
						style="width: 20px">{cat.kanji}</span
					>
					<span class="grid" style="min-width: 0">
						<span class="{on ? 'font-semibold' : ''} truncate">{cat.name}</span>
						<span class="text-ink-faint hidden truncate text-xs md:block" style="margin-top: 1px"
							>{cat.sub}</span
						>
					</span>
				</button>
			{/each}
		</div>

		<!-- pack browser -->
		<main class="min-w-0 flex-1 overflow-y-auto" style="padding: 16px">
			<div class="flex items-center gap-2" style="margin-bottom: 12px">
				<span class="kanji text-accent text-base">{areaMeta.kanji}</span>
				<span class="display text-ink text-lg font-semibold">{areaMeta.name}</span>
				<span class="mono text-ink-faint text-xs">{packs.length} packs</span>
			</div>

			<!-- write your own rule — classified into this area -->
			<div
				class="bg-paper-soft border-ink-faint rounded-lg border border-dashed"
				style="padding: 12px 16px; margin-bottom: 16px"
			>
				<div class="flex flex-wrap items-center gap-2" style="margin-bottom: 8px">
					<span class="kanji text-accent text-base">筆</span>
					<span class="text-ink text-sm font-semibold">Write your own rule</span>
					<span class="mono text-ink-faint text-xs">classified under {areaMeta.name}</span>
				</div>
				<div class="flex flex-wrap items-center gap-2">
					<input
						bind:value={draft}
						onkeydown={(e) => {
							if (e.key === 'Enter') addDraft();
						}}
						placeholder="e.g. Feature flags removed within two releases"
						aria-label="New rule text"
						class="bg-paper border-paper-edge text-ink flex-1 rounded border text-sm"
						style="min-width: 220px; box-sizing: border-box; padding: 8px 12px; font-family: inherit"
					/>
					<LevelPills value={draftLevel} onChange={(l) => (draftLevel = l)} />
					<span class="inline-flex items-center gap-1">
						<button
							type="button"
							onclick={() => (draftHard = !draftHard)}
							title={draftHard ? 'Non-negotiable — click to relax' : 'Mark non-negotiable'}
							aria-pressed={draftHard}
							class="text-base {draftHard ? 'text-warning' : 'text-ink-faint'}"
							style="background: none; border: none; cursor: pointer; padding: 0; line-height: 1"
							>{draftHard ? '★' : '☆'}</button
						>
						<span class="whitespace-nowrap text-xs {draftHard ? 'text-warning' : 'text-ink-faint'}"
							>{draftHard ? 'non-negotiable' : 'negotiable'}</span
						>
					</span>
					<button
						type="button"
						onclick={addDraft}
						disabled={!draft.trim()}
						class="rounded text-sm font-medium {draft.trim()
							? 'bg-ink text-on-primary'
							: 'bg-paper-mute text-ink-faint'}"
						style="padding: 8px 16px; border: none; font-family: inherit; cursor: {draft.trim()
							? 'pointer'
							: 'default'}">Add rule</button
					>
				</div>
				{#if authoredHere.length > 0}
					<div
						class="border-paper-edge flex flex-col gap-1 border-t"
						style="margin-top: 12px; padding-top: 8px"
					>
						{#each authoredHere as rule (rule.id)}
							<div class="text-ink flex items-center gap-2 text-sm">
								<span class="kanji text-accent text-sm">己</span>
								<span class="flex-1" style="min-width: 0">{rule.text}</span>
								<DojoChip>{levelLabel(rule.level)}</DojoChip>
								{#if rule.hard}
									<span class="text-warning text-xs">★</span>
								{/if}
								<button
									type="button"
									onclick={() => store.removeAuthored(rule.id)}
									title="Remove"
									aria-label="Remove rule"
									class="text-ink-faint text-base"
									style="background: none; border: none; cursor: pointer; line-height: 1">×</button
								>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- area explainer banners (stack / compliance / design) -->
			{#if area === 'stack'}
				<div
					class="bg-paper-soft border-paper-edge flex items-start gap-2 rounded-lg border"
					style="padding: 12px 16px; margin-bottom: 16px"
				>
					<span class="kanji text-accent text-base flex-shrink-0">検</span>
					<span class="text-ink-mute text-xs" style="line-height: 1.5">
						Each stack has a <b class="text-ink-soft font-semibold">reviewer</b> wired to open-source
						checkers — <span class="mono">qlty</span>, <span class="mono">eslint</span>,
						<span class="mono">ruff</span>, <span class="mono">clippy</span> and friends. Lint, format
						and complexity smells are caught mechanically before a human ever reads the diff.
					</span>
				</div>
			{:else if area === 'comp'}
				<div
					class="bg-accent-soft border-accent-edge flex items-start gap-2 rounded-lg border"
					style="padding: 12px 16px; margin-bottom: 16px"
				>
					<span class="kanji text-accent text-base flex-shrink-0">法</span>
					<span class="text-ink-soft text-xs" style="line-height: 1.5">
						Compliance controls come pre-written from the framework. Cherry-pick exactly the ones in
						scope, then <b class="font-semibold">mark them non-negotiable</b> (★) so no scope below can
						relax them. These are the wedge for regulated and agency work.
					</span>
				</div>
			{:else if area === 'design'}
				<div
					class="bg-paper-soft border-paper-edge flex items-start gap-2 rounded-lg border"
					style="padding: 12px 16px; margin-bottom: 16px"
				>
					<span class="kanji text-accent text-base flex-shrink-0">意</span>
					<span class="text-ink-mute text-xs" style="line-height: 1.5">
						These guards catch drift at the source — a raw <span class="mono">#hex</span>, a literal
						<span class="mono">font-size</span>, a locally redefined token. The exact mistakes that
						quietly grow a design system into thirty font sizes and a broken color ramp. Caught on
						write, not in a later audit.
					</span>
				</div>
			{/if}

			<!-- pack cards -->
			<div class="grid gap-3 md:grid-cols-2" style="padding-bottom: 32px">
				{#each packs as pack (pack.id)}
					{@const chosen = store.packChosenCount(pack)}
					{@const allOn = store.isPackFullyChosen(pack)}
					<div
						class="bg-paper-soft overflow-hidden rounded-lg border {chosen > 0
							? 'border-accent-edge'
							: 'border-paper-edge'}"
					>
						<!-- pack header -->
						<div
							class="border-paper-edge flex items-center gap-3 border-b"
							style="padding: 12px 16px"
						>
							<div class="flex-1" style="min-width: 0">
								<div class="flex flex-wrap items-center gap-2">
									<span class="text-ink text-sm font-semibold">{pack.name}</span>
									<span class="mono text-ink-faint text-xs">{pack.source}</span>
									{#if pack.regulated}
										<DojoChip toneClass="text-accent">法 regulated</DojoChip>
									{/if}
									{#if pack.rec}
										<DojoChip toneClass="text-success">recommended</DojoChip>
									{/if}
								</div>
								{#if pack.checkers.length > 0}
									<div class="flex flex-wrap items-center gap-1" style="margin-top: 8px">
										<span class="text-ink-mute text-xs">caught by</span>
										{#each pack.checkers as checker (checker)}
											<CheckerChip name={checker} />
										{/each}
									</div>
								{/if}
							</div>
							<LevelPills
								value={store.levelForPack(pack.id)}
								onChange={(l) => store.setPackLevel(pack.id, l)}
							/>
							<button
								type="button"
								onclick={() => store.toggleAll(pack)}
								class="whitespace-nowrap text-xs {allOn ? 'text-ink-mute' : 'text-accent'}"
								style="background: none; border: none; cursor: pointer; font-family: inherit"
								>{allOn ? 'clear all' : 'add all'}</button
							>
						</div>

						<!-- per-rule rows -->
						<div>
							{#each pack.rules as rule, i (rule.id)}
								{@const on = store.isIncluded(rule.id)}
								{@const nn = store.isNonNegotiable(rule.id)}
								<div
									class="grid items-center gap-3 {i < pack.rules.length - 1
										? 'border-paper-edge border-b'
										: ''}"
									style="grid-template-columns: 1fr auto; padding: 8px 16px"
								>
									<button
										type="button"
										onclick={() => store.toggleRule(rule.id)}
										aria-pressed={on}
										class="flex items-center gap-3 text-left"
										style="background: none; border: none; cursor: pointer; font-family: inherit; min-width: 0; padding: 4px 0"
									>
										<span
											class="inline-flex flex-shrink-0 items-center justify-center rounded-sm {on
												? 'bg-ink'
												: 'border-paper-edge border-2'}"
											style="width: 18px; height: 18px"
											aria-hidden="true"
										>
											{#if on}<span class="text-on-primary text-xs" style="line-height: 1">✓</span>{/if}
										</span>
										<span class="text-sm {on ? 'text-ink' : 'text-ink-soft'}">{rule.text}</span>
										{#if rule.hard}
											<span class="mono text-ink-faint whitespace-nowrap text-xs">hard guard</span>
										{/if}
									</button>
									<div class="flex items-center gap-2" style="justify-self: end">
										{#if on}
											<span class="inline-flex items-center gap-1">
												<button
													type="button"
													onclick={() => store.toggleStar(rule.id)}
													disabled={rule.hard}
													aria-pressed={nn}
													title={rule.hard
														? 'Hard guard — always non-negotiable'
														: nn
															? 'Non-negotiable — click to relax'
															: 'Mark non-negotiable'}
													class="text-base {nn ? 'text-warning' : 'text-ink-faint'}"
													style="background: none; border: none; padding: 0; line-height: 1; cursor: {rule.hard
														? 'default'
														: 'pointer'}">{nn ? '★' : '☆'}</button
												>
												<span
													class="whitespace-nowrap text-xs {nn ? 'text-warning' : 'text-ink-faint'}"
													>{nn ? 'non-negotiable' : 'negotiable'}</span
												>
											</span>
										{/if}
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		</main>
	</div>

	<!-- sticky selection footer -->
	<div
		class="border-paper-edge bg-paper-soft flex flex-shrink-0 flex-wrap items-center gap-4 border-t"
		style="padding: 12px 16px"
	>
		<div class="flex items-baseline gap-2">
			<span class="display text-ink text-2xl font-light" style="line-height: 1"
				>{store.selectedCount}</span
			>
			<span class="text-ink-mute text-sm">rules selected</span>
		</div>
		{#if store.nonNegotiableCount > 0}
			<span
				class="text-warning bg-warning-soft border-warning-edge inline-flex items-center gap-1 rounded-full border text-xs"
				style="padding: 4px 12px"
			>
				★ {store.nonNegotiableCount} non-negotiable
			</span>
		{/if}
		<span class="flex-1"></span>
		<span class="mono text-ink-faint hidden text-xs md:inline">cascades to every scope below</span>
		<button
			type="button"
			disabled={store.selectedCount === 0}
			class="inline-flex items-center gap-2 rounded-lg text-sm font-medium {store.selectedCount === 0
				? 'bg-paper-mute text-ink-faint'
				: 'bg-ink text-on-primary'}"
			style="padding: 8px 24px; border: none; cursor: {store.selectedCount === 0
				? 'default'
				: 'pointer'}"
		>
			<span class="kanji text-sm {store.selectedCount === 0 ? 'text-ink-faint' : 'text-accent'}"
				>掟</span
			>
			Add {store.selectedCount || ''} to constitution
		</button>
	</div>
</div>
