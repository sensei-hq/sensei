// dojo component kit — the reusable foundation every dojo screen composes
// from (ported from docs/mockups/Sensei/lib/dojo/dojo2-kit.jsx). Chrome +
// generic primitives + the DOMAIN components (governance: ladder / rule /
// conflict / stance; relay: run / gate / needs / decision / chat).
//
// Namespaced under lib/components/kit/. Import as
// `import { AppShell, ProjectRow } from '$lib/components/kit'`.

// Vocab + shared types.
export * from './vocab';
export * from './types';
export * from './needs';
export * from './plan';

// Primitives.
export { default as Icon } from './Icon.svelte';
export { default as KanjiToken } from './KanjiToken.svelte';
export { default as Avatar } from './Avatar.svelte';
export { default as Chip } from './Chip.svelte';
export { default as ClassChip } from './ClassChip.svelte';
export { default as RoleTag } from './RoleTag.svelte';
export { default as PhasePill } from './PhasePill.svelte';
export { default as SectionHead } from './SectionHead.svelte';
export { default as Banner } from './Banner.svelte';
export { default as StatBadge } from './StatBadge.svelte';
export { default as EmptyState } from './EmptyState.svelte';
export { default as Btn } from './Btn.svelte';
export { default as ListSection } from './ListSection.svelte';
export { default as ProjectRow } from './ProjectRow.svelte';
export { default as MyDojoRow } from './MyDojoRow.svelte';

// Plan / run-detail visualizers (ported from dojo2-kit.jsx K2Plan*).
export { default as PlanBar } from './PlanBar.svelte';
export { default as PlanPips } from './PlanPips.svelte';
export { default as PlanNode } from './PlanNode.svelte';
export { default as PlanStage } from './PlanStage.svelte';
export { default as PlanGraph } from './PlanGraph.svelte';
export { default as PlanOutline } from './PlanOutline.svelte';
export { default as RunActivity } from './RunActivity.svelte';

// Inbox — the one list of in-flight sessions + its filter tabs.
export { default as SubTabs } from './SubTabs.svelte';
export { default as InboxRow } from './InboxRow.svelte';

// Reused shipped primitives, re-exported as thin kit wrappers (Spark / EnsoRing
// / Confidence live once in ../).
export { default as Spark } from './Spark.svelte';
export { default as Enso } from './Enso.svelte';
export { default as ConfidenceBar } from './ConfidenceBar.svelte';

// Chrome.
export { default as OrgSwitcher } from './OrgSwitcher.svelte';
export { default as TopBar } from './TopBar.svelte';
export { default as ContextHeader } from './ContextHeader.svelte';
export { default as NavPane } from './NavPane.svelte';
export { default as AppShell } from './AppShell.svelte';
export { default as AccountMenu } from './AccountMenu.svelte';
export { default as TabBar } from './TabBar.svelte';

// Governance — the constitution ladder + stance + the rule editor.
export { default as LadderRung } from './LadderRung.svelte';
export { default as RuleRow } from './RuleRow.svelte';
export { default as RuleEditor } from './RuleEditor.svelte';
export { default as ConflictCard } from './ConflictCard.svelte';
export { default as StanceDial } from './StanceDial.svelte';

// Relay — needs-you · chat.
export { default as NeedsYouBand } from './NeedsYouBand.svelte';
export { default as NeedsRow } from './NeedsRow.svelte';
export { default as NeedsActions } from './NeedsActions.svelte';
export { default as NeedsResolved } from './NeedsResolved.svelte';
export { default as ChatThread } from './ChatThread.svelte';
