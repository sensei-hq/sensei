<script lang="ts">
	import { untrack } from 'svelte';
	import Icon from './Icon.svelte';
	import Btn from './Btn.svelte';
	import { RULE_FAMILIES } from '$lib/org-ladder-view';
	import type { KitRule } from './types';

	// The rule add/edit editor (mockup RuleEditor) — an overlay card composed from
	// kit atoms. Adds or edits ONE rule of a dōjō's authored constitution: the rule
	// text, its family (a brand glyph), and the ★ non-negotiable lock. Presentational
	// — the local text/family/hard are draft state; `onSave` bubbles the composed
	// rule and `onClose` dismisses. `rule` seeds edit mode (absent ⇒ add). `scope` +
	// `scopeName` name the section the rule belongs to (shown in the header).
	let {
		rule,
		scope,
		scopeName,
		onClose,
		onSave
	}: {
		rule?: KitRule;
		scope: string;
		scopeName: string;
		onClose?: () => void;
		onSave?: (rule: KitRule) => void;
	} = $props();

	// Seed the draft ONCE from `rule` — the screen re-mounts the editor per open
	// (keyed on the target), so capturing the initial value is intentional.
	// `untrack` makes that explicit (the same idiom as ScrProjectPreview).
	let text = $state(untrack(() => rule?.text ?? ''));
	let fam = $state(untrack(() => rule?.kanji ?? '守'));
	let hard = $state(untrack(() => rule?.hard ?? false));

	function save() {
		onSave?.({ kanji: fam, text: text.trim(), hard });
	}
</script>

<!-- The overlay: a full-bleed ink scrim button dismisses on click (the established
     dojo modal pattern — NavPane / ConsoleTopBar), the centered dialog sits above. -->
<div class="absolute inset-0 z-50 flex items-center justify-center p-6">
	<button
		type="button"
		aria-label="Dismiss"
		class="bg-ink absolute inset-0 cursor-default border-none opacity-40"
		onclick={onClose}
	></button>
	<div
		class="bg-paper relative w-full overflow-hidden rounded-lg shadow-lg"
		style="max-width: 520px"
		role="dialog"
		aria-modal="true"
		aria-label={rule ? 'Edit rule' : 'New rule'}
		onkeydown={(e) => e.key === 'Escape' && onClose?.()}
		tabindex="-1"
	>
		<div class="border-paper-edge flex items-center gap-2 border-b" style="padding: 16px 24px">
			<Icon name={rule ? 'pen-2' : 'add-circle'} size={18} toneClass="text-accent" />
			<span class="display text-ink text-lg" style="letter-spacing: -0.01em"
				>{rule ? 'Edit rule' : 'New rule'}</span
			>
			<span class="flex-1"></span>
			<span class="mono text-ink-mute text-xs">{scope} · {scopeName}</span>
		</div>

		<div class="flex flex-col gap-4" style="padding: 24px">
			<div>
				<div
					class="text-ink-mute text-xs font-semibold uppercase"
					style="letter-spacing: 0.18em; margin-bottom: 8px"
				>
					Rule
				</div>
				<textarea
					bind:value={text}
					rows={3}
					placeholder="State the rule as an instruction sensei can follow…"
					class="text-ink bg-paper-mute border-paper-edge text-sm w-full resize-none rounded border"
					style="padding: 12px; line-height: 1.5; outline: none"
				></textarea>
			</div>

			<div>
				<div
					class="text-ink-mute text-xs font-semibold uppercase"
					style="letter-spacing: 0.18em; margin-bottom: 8px"
				>
					Family
				</div>
				<div class="flex flex-wrap gap-2">
					{#each RULE_FAMILIES as f (f.kanji)}
						{@const on = fam === f.kanji}
						<button
							type="button"
							onclick={() => (fam = f.kanji)}
							aria-pressed={on}
							class="inline-flex cursor-pointer items-center gap-2 rounded-full border text-sm {on
								? 'bg-accent-soft border-accent text-accent'
								: 'bg-paper-mute border-paper-edge text-ink-soft'}"
							style="padding: 8px 12px"
						>
							<span class="kanji" style="font-size: 14px">{f.kanji}</span>{f.label}
						</button>
					{/each}
				</div>
			</div>

			<button
				type="button"
				onclick={() => (hard = !hard)}
				aria-pressed={hard}
				class="flex cursor-pointer items-center gap-3 rounded-lg border text-left {hard
					? 'bg-accent-soft border-accent-soft'
					: 'bg-paper-mute border-paper-edge'}"
				style="padding: 12px 16px"
			>
				<span class={hard ? 'text-accent' : 'text-ink-faint'} style="font-size: 16px">★</span>
				<div class="flex-1">
					<div class="text-ink text-sm font-medium">Non-negotiable</div>
					<div class="mono text-ink-mute text-xs">
						Locks the rule — no narrower scope can relax it.
					</div>
				</div>
				<span
					class="relative flex-shrink-0 rounded-full {hard ? 'bg-accent' : 'bg-paper-mute'}"
					style="width: 34px; height: 20px; transition: background 0.15s"
				>
					<span
						class="bg-paper absolute rounded-full"
						style="top: 2px; left: {hard ? 16 : 2}px; width: 16px; height: 16px; transition: left 0.15s"
					></span>
				</span>
			</button>
		</div>

		<div
			class="bg-paper-mute border-paper-edge flex gap-2 border-t"
			style="padding: 16px 24px"
		>
			<span class="flex-1"></span>
			<Btn size="sm" variant="ghost" onclick={onClose}>Cancel</Btn>
			<Btn size="sm" icon="check-circle" onclick={save}>{rule ? 'Save rule' : 'Add rule'}</Btn>
		</div>
	</div>
</div>
