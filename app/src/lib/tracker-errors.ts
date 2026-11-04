// The issue tracker's fixed daemon codes, in the words the screens show
// (docs/spec/screen/workspace-integrations.md error table). Shared by the
// Integrations and Sprints screens so a code reads the same everywhere.

/** A failure as the screen shows it. */
export interface TrackerAlert {
  code: string;
  text: string;
  /** The form field the failure concerns, for `aria-describedby`. */
  field: 'board_url' | 'email' | 'token' | 'level' | null;
  link: { href: string; label: string } | null;
}

export const CREATE_TOKEN_URL = 'https://id.atlassian.com/manage-profile/security/api-tokens';

const reach = (code: string) =>
  `Couldn’t reach Jira (${code}). Check the site address and your connection.`;
const keychain = 'The saved token can’t be read from the system keychain.';

/** Fixed daemon codes → the screen's words (the spec's error table). */
const WORDS: Record<string, string> = {
  invalid_base_url:
    'That isn’t a Jira Cloud board address. It should look like https://<your-site>.atlassian.net/…/boards/<number> (older sites: <your-site>.jira.com).',
  unauthorized: 'Jira rejected the email and token.',
  forbidden: 'This account can’t read that board.',
  email_required: 'Enter your Atlassian account email and an API token.',
  token_required: 'Enter your Atlassian account email and an API token.',
  board_project_mismatch: 'That board belongs to a different Jira project than the one given.',
  not_found: 'Jira no longer has this board (it may have been deleted).',
  rate_limited: 'Jira is rate-limiting requests. Try again in a minute.',
  transport: reach('transport'),
  http_error: reach('http_error'),
  redirect: reach('redirect'),
  decode: reach('decode'),
  token_missing: keychain,
  credential_store: keychain,
  forbidden_caller: 'The sensei daemon refused this window’s request.',
  not_connected: 'This workspace has no tracker connected.',
  tracker_not_found: 'This board is no longer connected to this workspace.',
  level_required: 'Choose what the board serves.',
  invalid_level: 'A board serves epics, sprints or tasks.',
  workspace_not_found: 'This workspace no longer exists in sensei.',
  internal: 'The sensei daemon hit a database error.',
  db: 'sensei hit a database error while syncing.',
  unsupported_tracker: 'This tracker type isn’t available in this version of sensei.',
  busy: 'A sync is already running.',
  superseded: 'The board’s token changed while it synced. Sync again.',
  task_not_found: 'No ticket with that key in sensei.',
  bad_request: 'Jira rejected the request.',
  invalid_body: 'sensei refused that change.',
};

/** Why a write-back pass could not run or stopped (tracker-metrics-writeback W-UI), by its
 *  `writeback_error` code. Apart from `WORDS`: the same codes mean something else for a sync. */
export const WRITEBACK_STOPPED: Record<string, string> = {
  token_missing: 'Couldn’t read the Jira token to write back. Replace the token.',
  credential_store: 'Couldn’t read the Jira token to write back. Replace the token.',
  db: 'Couldn’t record what was written. Write-back retries on each sync.',
  rate_limited: 'Jira is rate-limiting writes. Write-back resumes on the next sync.',
  transport: 'Writing didn’t finish in time. Write-back resumes on the next sync.',
};

/** Why the Fields editor's save was refused (W7), shown on the row it names (W-UI). Apart
 *  from `WORDS`: there `invalid_body` is the write-back switch's text. */
export const FIELD_MAPPING_ERRORS: Record<string, string> = {
  unknown_metric: 'Unknown metric.',
  duplicate_mapping: 'Already mapped.',
  invalid_field_ref: 'Use a custom field id like customfield_10050.',
  field_not_found: 'No such field on this Jira site.',
  field_not_number: 'That field isn’t a number field.',
  field_in_use: 'That field already has values on this board. Pick an empty field.',
  invalid_body: 'Couldn’t save the mapping.',
  busy: 'The board is syncing or writing back. Save again in a moment.',
};

const FIELD: Record<string, TrackerAlert['field']> = {
  invalid_base_url: 'board_url',
  board_not_found: 'board_url',
  email_required: 'email',
  token_required: 'token',
  unauthorized: 'token',
  level_required: 'level',
  invalid_level: 'level',
};

/** The alert for a failed request: `status` 0 is the daemon not answering. */
export function errorFor(status: number, body: Record<string, unknown> | undefined): TrackerAlert {
  if (status === 0) {
    return { code: 'unreachable', text: 'The sensei daemon didn’t respond.', field: null, link: null };
  }
  const code = typeof body?.error === 'string' ? body.error : `http_${status}`;
  const field = FIELD[code] ?? null;
  if (code === 'board_not_found') {
    return {
      code,
      text: `No board #${String(body?.board_ref ?? '')} on ${String(body?.base_url ?? '')}.`,
      field,
      link: null,
    };
  }
  if (code === 'task_not_on_board') {
    return {
      code,
      text: `${String(body?.key ?? '')} isn’t on board #${String(body?.board_ref ?? '')}: it may be in another Jira project, or outside the board’s filter.`,
      field,
      link: null,
    };
  }
  if (code === 'board_already_connected') {
    const workspace = (body?.workspace ?? {}) as { id?: string; name?: string };
    return {
      code,
      text: `This board is already connected to workspace ${workspace.name ?? ''}.`,
      field,
      link: workspace.id ? { href: `/workspace/${workspace.id}/integrations`, label: workspace.name ?? workspace.id } : null,
    };
  }
  const link = code === 'unauthorized' ? { href: CREATE_TOKEN_URL, label: 'create a new token' } : null;
  return { code, text: WORDS[code] ?? `Unexpected error from sensei (${code}).`, field, link };
}
