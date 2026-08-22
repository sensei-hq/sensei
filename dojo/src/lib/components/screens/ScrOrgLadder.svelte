<script lang="ts">
	import {
		SectionHead,
		Btn,
		Icon,
		Chip,
		KanjiToken,
		RuleRow,
		RuleEditor
	} from '$lib/components/kit';
	import type { KitConstitutionSection, KitRule } from '$lib/components/kit/types';
	import { createOrgLadder } from '$lib/org-ladder-state.svelte';
	import { untrack } from 'svelte';

	// The dōjō-authoring constitution (mockup ScrOrgLadder) — the richest org
	// screen. The dōjō authors its OWN rules by scope (company-wide · per team ·
	// per stack, packs per stack); this is NOT the resolution ladder (that appears
	// only at project-preview time). A left rail groups the sections (Company ·
	// Teams · Stacks); the right pane shows the active section's adopted packs
	// (stacks) and its rules, each with an include toggle + edit pencil, plus a
	// New/Add rule affordance that opens the RuleEditor overlay. Presentational:
	// the page supplies the sections (kit fixtures this chunk); the active section,
	// include toggles, and editor open/save live in the `org-ladder-state`
	// rune store. `orgName` heads the section.
	let {
		orgName,
		sections
	}: {
		orgName: string;
		sections: KitConstitutionSection[];
	} = $props();

	// Seed the authoring store ONCE from the props — a different org re-mounts this
	// screen (an /org/[slug]/ladder navigation), so the active section + include
	// state is captured at mount (the same idiom as ScrProjectPreview).
	const ladder = untrack(() => createOrgLadder(sections));

	// Save a rule from the editor. Presentational this chunk — the composed rule is
	// acknowledged and the editor closes (real /v1 authoring lands later).
	function onSave(_rule: KitRule) {
		ladder.closeEditor();
	}
</script>

<div class="relative">
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		<SectionHead
		kanji="掟" eyebrow={orgName + ' · governance'} title="Constitution"
		description="The dōjō authors its rules by scope — company-wide, per team, per stack. Stacks also adopt rule packs. These are the dōjō's own rules; how they combine with your personal and a project's rules is resolved — and shown — when you open a project."
	>
			{#snippet right()}
				<Btn size="sm" icon="add-circle" onclick={ladder.openNew}>New rule</Btn>
			{/snippet}
		</SectionHead>

		<div
			class="grid gap-4 items-start grid-cols-1 md:grid-cols-[252px_1fr]"
		>
			<!-- Left rail — sections grouped Company · Teams · Stacks. -->
			<div class="flex flex-col gap-4">
				{#each ladder.groups as g (g.group)}
					<div>
						<div
							class="text-ink-mute text-xs font-semibold uppercase"
							style="letter-spacing: 0.18em; padding: 0 8px; margin-bottom: 8px"
						>
							{g.group}
						</div>
						<div class="flex flex-col gap-2">
							{#each g.sections as s (s.id)}
								{@const on = ladder.active === s.id}
								<button
									type="button"
									onclick={() => ladder.setActive(s.id)}
									aria-pressed={on}
									class="flex w-full cursor-pointer items-center gap-3 rounded-lg border text-left {on
										? 'bg-accent-soft border-accent'
										: 'bg-paper-mute border-paper-edge'}"
									style="padding: 12px"
								>
									<KanjiToken
										char={s.kanji}
										size="lg"
										toneClass={on ? 'text-accent' : 'text-ink-soft'}
										w={24}
									/>
									<div class="flex-1" style="min-width: 0">
										<div class="text-ink truncate text-sm font-medium">{s.scope}</div>
										<div class="mono text-ink-mute text-xs">
											{(s.rules ?? []).length} rules{s.packs && s.packs.length
												? ' · ' + s.packs.length + ' pack'
												: ''}
										</div>
									</div>
								</button>
							{/each}
						</div>
					</div>
				{/each}
			</div>

			<!-- Right pane — the active section's packs (stacks) + its rules. -->
			<div class="flex flex-col gap-4">
				{#if ladder.section.packs}
					<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
						<div
							class="border-paper-edge flex items-center gap-2 border-b"
							style="padding: 12px 16px"
						>
							<Icon name="box" size={16} toneClass="text-accent" />
							<span
								class="text-ink text-xs font-semibold uppercase"
								style="letter-spacing: 0.18em">Rule packs for this stack</span
							>
							<span class="flex-1"></span>
							<Btn size="sm" variant="ghost" icon="add-circle">Adopt pack</Btn>
						</div>
						{#if ladder.section.packs.length}
							{#each ladder.section.packs as pack (pack)}
								<div
									class="border-paper-edge flex items-center gap-3 border-b"
									style="padding: 12px 16px"
								>
									<Icon name="box" size={17} toneClass="text-accent" />
									<span class="text-ink flex-1 text-sm">{pack}</span>
									<Chip
										icon="check-circle"
										toneClass="text-success"
										softClass="bg-success-soft"
										edgeClass="border-success-soft">adopted</Chip
									>
								</div>
							{/each}
						{:else}
							<div class="mono text-ink-mute text-xs" style="padding: 12px 16px">
								No pack adopted — this stack runs on its own rules and the company baseline.
							</div>
						{/if}
					</div>
				{/if}

				<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
					<div
						class="border-paper-edge flex items-center gap-2 border-b"
						style="padding: 12px 16px"
					>
						<KanjiToken char={ladder.section.kanji} size="base" toneClass="text-accent" />
						<span
							class="text-ink text-xs font-semibold uppercase"
							style="letter-spacing: 0.18em">{ladder.section.scope} · rules</span
						>
						<span class="flex-1"></span>
						{#if ladder.excluded > 0}
							<button
								type="button"
								onclick={ladder.toggleShowExcluded}
								class="mono text-ink-mute inline-flex cursor-pointer items-center gap-1 bg-transparent text-xs"
							>
								<Icon
									name={ladder.showExcluded ? 'eye-closed' : 'eye'}
									size={14}
									toneClass="text-ink-mute"
								/>{ladder.showExcluded ? 'Hide' : 'Show'}
								{ladder.excluded} excluded
							</button>
						{/if}
						<Btn size="sm" variant="ghost" icon="add-circle" onclick={ladder.openNew}>Add</Btn>
					</div>
					{#each ladder.section.rules ?? [] as rule, i (i)}
						{#if ladder.showExcluded || ladder.isIncluded(i)}
							<RuleRow
								{rule}
								included={ladder.isIncluded(i)}
								onToggle={() => ladder.toggleInclude(i)}
								onEdit={() => ladder.openEdit(rule)}
							/>
						{/if}
					{/each}
					{#if !ladder.showExcluded && ladder.excluded > 0}
						<div
							class="bg-paper mono text-ink-mute text-xs"
							style="padding: 8px 16px"
						>
							{ladder.excluded} excluded rule{ladder.excluded > 1 ? 's' : ''} hidden · consolidated view
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>

	{#if ladder.editing}
		<RuleEditor
			rule={ladder.editing.rule}
			scope={ladder.section.scope}
			scopeName={orgName}
			onClose={ladder.closeEditor}
			{onSave}
		/>
	{/if}
</div>
