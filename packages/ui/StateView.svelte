<script lang="ts">
  // The four states every data screen has, in one switch, for sensei and the
  // dōjō: loading (Loading), an honest empty (EmptyState), a failure (Notice at
  // screen size, with Retry), and ready (the children). A failure is always
  // visibly distinct from an empty result — the no-fabrication guarantee.
  //
  // The error copy is the screen's `notice` (from its state module,
  // `noticeFromApiError`) when it has one, else generic. Either way it names a
  // next step: the app's default remedy (`setStateDefaults`) when the notice
  // has none.
  import type { Snippet } from 'svelte';
  import Notice from './Notice.svelte';
  import EmptyState from './EmptyState.svelte';
  import Loading from './Loading.svelte';
  import { stateErrorNotice } from './notice.js';
  import { useStateDefaults } from './state-defaults.js';
  import type { NoticeAction, NoticeTone } from './types.js';

  interface StateNotice {
    tone?: NoticeTone;
    eyebrow?: string;
    title: string;
    detail?: string;
    remedy?: string;
    code?: string;
  }

  let {
    status,
    error = null,
    failure = null,
    notice,
    onretry,
    kanji = '空',
    title = '',
    description = '',
    action,
    children,
  }: {
    status: 'loading' | 'error' | 'empty' | 'ready';
    error?: string | null;
    /** The read that failed, status and all (`ApiError`): said through
     *  noticeFromApiError, so a 401/403 reads as a refusal, not a fault. */
    failure?: { status: number; message: string } | null;
    notice?: StateNotice;
    onretry?: () => void;
    kanji?: string;
    title?: string;
    description?: string;
    /** The empty state's next step. */
    action?: NoticeAction;
    children?: Snippet;
  } = $props();

  const defaults = useStateDefaults();
  const view = $derived(stateErrorNotice(error, { notice, failure, remedy: defaults.remedy, service: defaults.service }));
</script>

{#if status === 'loading'}
  <Loading data-screen-state="loading" />
{:else if status === 'error'}
  <Notice
    size="screen"
    tone={view.tone}
    eyebrow={view.eyebrow}
    title={view.title}
    detail={view.detail}
    remedy={view.remedy}
    code={view.code}
    primary={onretry ? { label: 'Retry', onclick: onretry, name: 'retry' } : undefined}
    data-screen-state="error"
  />
{:else if status === 'empty'}
  <EmptyState size="screen" {kanji} {title} {description} {action} />
{:else}
  {@render children?.()}
{/if}
