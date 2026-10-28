<script lang="ts">
  // One session in the digest list. Clickable → the Instruments Replay tab
  // for this session (the parent builds the href / goto). Pure presentation:
  // every field is pre-derived on the EnrichedSession.
  //
  // Shared by the Observatory Sessions screen and the workspace-scoped Sessions
  // screen. The folder-role chip renders only when `folderRole` is populated
  // (multi-repo workspaces); it stays dormant on the multi-workspace observatory
  // view and on single-folder workspaces.
  import type { EnrichedSession, QualityTone } from '$lib/sessions-digest.js';
  import { sessionFtrView } from '$lib/ftr';

  interface Props {
    session: EnrichedSession;
    onselect: (id: string) => void;
  }
  let { session, onselect }: Props = $props();

  // Quality → dot utility (named-token background). The lookup IS the rule.
  const DOT: Record<QualityTone, string> = {
    good: 'bg-success',
    bad: 'bg-warning',
    ugly: 'bg-accent',
    neutral: 'bg-ink-soft',
  };

  // The shared session view (lib/ftr): the same words and colours as Today and
  // Instruments; an unscored session is a faint dash.
  const ftr = $derived(sessionFtrView(session.ftr));

  const correctionsLabel = $derived(
    session.corrections === 0 ? 'first-try' : `${session.corrections}× rework`,
  );

  // Token column: the in+out sum, with an in/out breakdown on hover. A dash when
  // the session's source carried no usage (honest — never shown as 0).
  const tokensTitle = $derived(
    session.tokens == null
      ? 'no token usage captured'
      : `${(session.tokensIn ?? 0).toLocaleString()} in · ${(session.tokensOut ?? 0).toLocaleString()} out`,
  );
</script>

<button
  type="button"
  class="session-row w-full grid grid-cols-[10px_56px_1fr_auto] items-center gap-3 py-3 px-4 text-left bg-transparent border-none cursor-pointer rounded-md"
  data-session-row={session.id}
  data-quality={session.quality}
  onclick={() => onselect(session.id)}
>
  <span class="w-2.5 h-2.5 rounded-full {DOT[session.quality]}"></span>

  <span class="font-mono text-xs text-ink-soft" style='font-feature-settings: "tnum";'>{session.time}</span>

  <span class="flex flex-col gap-0.5 min-w-0">
    <span class="text-sm text-ink truncate">{session.title}</span>
    <span class="text-xs text-ink-mute truncate flex items-center gap-1.5">
      <span class="truncate">{session.workspace || 'unlinked'}{#if session.agent} · {session.agent}{/if}</span>
      {#if session.folderRole}
        <span
          class="shrink-0 py-px px-1.5 rounded-full bg-paper-mute text-ink-soft uppercase tracking-wide text-xs"
          data-folder-role={session.folderRole}
        >{session.folderRole}</span>
      {/if}
    </span>
  </span>

  <span class="flex items-center gap-4 text-xs">
    <span class="font-mono text-ink-soft w-14 text-right" style='font-feature-settings: "tnum";'>{session.duration}</span>
    <span
      class="font-mono text-ink-faint w-14 text-right"
      style='font-feature-settings: "tnum";'
      title={tokensTitle}
    >{session.tokensLabel}</span>
    <span class="text-ink-mute w-20 text-right">{correctionsLabel}</span>
    <span class="{ftr.text} w-16 text-right uppercase tracking-wide">{ftr.label}</span>
    <span class="text-ink-mute w-20 text-right capitalize">{session.outcome}</span>
  </span>
</button>

<style>
  .session-row:hover {
    background: var(--paper-mute);
  }
</style>
