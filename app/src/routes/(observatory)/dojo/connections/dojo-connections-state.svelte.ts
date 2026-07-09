// Observatory · Dōjō connections — derivations, status→token mapping, copy and
// the connect-form state. The daemon owns memberships + sync-status derivation
// (`crates/senseid/src/dojo/memberships.rs`); this module only maps the wire
// discriminators (sync_status, kind) to named-token chip classes + copy, and
// keeps the add-a-connection form's fields / validation / submit state so the
// `+page.svelte` stays a pure template.
//
// Wire-API-wins: `sync_status` and `kind` come back as free strings, so an
// unexpected value degrades to muted ink rather than being dropped — mirroring
// the traceability state module's discipline.

import type { SenseiApi } from '$lib/api.js';
import type { ConnectDojoBody, DojoBoundProject, DojoMembership } from '$lib/types.js';
// Reuse the daemon's loopback/https rule (mirrors `require_secure_url`) rather
// than re-deriving it — the federation source form already owns this check.
import { validateSourceUrl } from '$lib/knowledge-sources.svelte.js';

/** The four membership kinds — mirrors `dojo.membership_kind`. `client` drives
 *  client-precedence routing, so it is surfaced with a distinct chip. */
export type MembershipKind = 'employer' | 'client' | 'community' | 'personal';

/** Semantic tone a sync status maps to. `ink` = an unexpected status we still
 *  surface rather than drop (never a status colour). */
export type SyncTone = 'success' | 'warning' | 'danger' | 'info' | 'ink';

/** Named-token utility classes for a chip, plus the label to print. */
export interface ChipClasses {
  bg: string;
  text: string;
  label: string;
}

/** A kind chip — kanji glyph + label + named-token classes. */
export interface KindPill {
  kanji: string;
  label: string;
  bg: string;
  text: string;
}

/** The connect form's field values, decoupled from the reactive class so the
 *  validation + body-building are pure and unit-testable. */
export interface ConnectDraft {
  membershipId: string;
  tenantKey: string;
  registryUrl: string;
  kind: MembershipKind;
  deviceToken: string;
  projectId: string;
}

/** Kind options for the connect form's select, in precedence-story order. */
export const KIND_OPTIONS: { value: MembershipKind; label: string }[] = [
  { value: 'employer', label: 'employer' },
  { value: 'client', label: 'client' },
  { value: 'community', label: 'community' },
  { value: 'personal', label: 'personal' },
];

/** Pure: sync status → semantic tone. `healthy` reads success, `stale` warning,
 *  `error` danger, `authenticating` info; anything else degrades to muted ink so
 *  an unexpected wire value is still shown, never hidden. */
export function syncTone(status: string): SyncTone {
  switch (status) {
    case 'healthy':
      return 'success';
    case 'stale':
      return 'warning';
    case 'error':
      return 'danger';
    case 'authenticating':
      return 'info';
    default:
      return 'ink';
  }
}

/** Pure: named-token chip classes + label for a sync status. The label is the
 *  raw wire status (honest — a `healthy` chip never claims "connected" when the
 *  heartbeat says otherwise). The lookup IS the rendering rule, so it lives with
 *  the domain rather than in the component. */
export function syncChip(status: string): ChipClasses {
  switch (syncTone(status)) {
    case 'success':
      return { bg: 'bg-success-soft', text: 'text-success', label: status };
    case 'warning':
      return { bg: 'bg-warning-soft', text: 'text-warning', label: status };
    case 'danger':
      return { bg: 'bg-danger-soft', text: 'text-danger', label: status };
    case 'info':
      return { bg: 'bg-info-soft', text: 'text-info', label: status };
    default:
      return { bg: 'bg-paper-mute', text: 'text-ink-mute', label: status };
  }
}

/** Pure: kind → kanji glyph + label + chip classes. `client` is given the accent
 *  treatment so client-precedence stays visible (spec "wrong gate": a client kind
 *  that isn't distinguished hides the precedence story); the rest read muted. */
export function kindPill(kind: string): KindPill {
  switch (kind) {
    case 'employer':
      return { kanji: '社', label: 'employer', bg: 'bg-paper-mute', text: 'text-ink-mute' };
    case 'client':
      return { kanji: '客', label: 'client', bg: 'bg-accent-soft', text: 'text-accent' };
    case 'community':
      return { kanji: '群', label: 'community', bg: 'bg-paper-mute', text: 'text-ink-mute' };
    case 'personal':
      return { kanji: '己', label: 'personal', bg: 'bg-paper-mute', text: 'text-ink-mute' };
    default:
      return { kanji: '結', label: kind || 'unknown', bg: 'bg-paper-mute', text: 'text-ink-mute' };
  }
}

/** Pure: "3 projects bound" / "1 project bound" / "no projects bound". */
export function boundProjectsSummary(projects: DojoBoundProject[]): string {
  const n = projects.length;
  if (n === 0) return 'no projects bound';
  return `${n} ${n === 1 ? 'project' : 'projects'} bound`;
}

/** Pure: the list header line, e.g. "3 connections · 2 healthy." Sentence case,
 *  no exclamation — voice charter. */
export function connectionsHeadline(connections: DojoMembership[]): string {
  const n = connections.length;
  const healthy = connections.filter((c) => c.sync_status === 'healthy').length;
  const conns = `${n} ${n === 1 ? 'connection' : 'connections'}`;
  return `${conns} · ${healthy} healthy.`;
}

// A membership id must be the service-assigned uuid (`dojo.memberships.id`) —
// the daemon parses it as a uuid and 400s otherwise, so reject it client-side.
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Pure: validate an optional registry url. Empty is allowed (the daemon
 *  defaults it); a provided url must be https or loopback http — reusing the
 *  federation rule so the two stay in lockstep with `require_secure_url`. */
export function validateRegistryUrl(url: string): string | null {
  if (!url.trim()) return null;
  return validateSourceUrl(url);
}

/** Pure: validate the connect draft. Returns null when submittable, else a short
 *  message. Mirrors the daemon's required fields (membership uuid, tenant key,
 *  device token) so an invalid pairing is caught before the round-trip. */
export function validateConnect(d: ConnectDraft): string | null {
  const membershipId = d.membershipId.trim();
  if (!membershipId) return 'membership id required';
  if (!UUID_RE.test(membershipId)) return 'membership id must be the service membership uuid';
  if (!d.tenantKey.trim()) return 'tenant key required';
  if (!d.deviceToken.trim()) return 'device token required';
  return validateRegistryUrl(d.registryUrl);
}

/** Pure: build the `POST /api/dojo/memberships` body from a draft. Trims and
 *  drops the optional url/project when blank so the daemon applies its defaults;
 *  role / authenticated_via / attribution_default are left to the daemon
 *  (contributor · device_code · named). The device token is write-only. */
export function toConnectBody(d: ConnectDraft): ConnectDojoBody {
  const body: ConnectDojoBody = {
    membership_id: d.membershipId.trim(),
    tenant_key: d.tenantKey.trim(),
    kind: d.kind,
    credential: d.deviceToken,
  };
  const registry = d.registryUrl.trim();
  if (registry) body.registry_url = registry;
  const project = d.projectId.trim();
  if (project) body.project_id = project;
  return body;
}

/**
 * The connect-a-Dōjō form. Holds the field values, exposes validation as a
 * getter, and owns the submit action. The api client is passed into `submit`
 * as the seam, so the state is testable with a hand-rolled mock and never
 * reaches for a global. `submit` resolves `true` on success (the page then runs
 * `invalidateAll()` to re-load the list) and `false` on a validation or wire
 * failure, in which case `error` carries the reason.
 */
export class ConnectForm {
  /** Disclosure — the form is opened from the header's "Connect a Dōjō" action. */
  open = $state(false);
  membershipId = $state('');
  tenantKey = $state('');
  registryUrl = $state('');
  kind = $state<MembershipKind>('employer');
  deviceToken = $state('');
  projectId = $state('');
  submitting = $state(false);
  error = $state<string | null>(null);

  get draft(): ConnectDraft {
    return {
      membershipId: this.membershipId,
      tenantKey: this.tenantKey,
      registryUrl: this.registryUrl,
      kind: this.kind,
      deviceToken: this.deviceToken,
      projectId: this.projectId,
    };
  }

  get validationError(): string | null {
    return validateConnect(this.draft);
  }

  get canSubmit(): boolean {
    return !this.submitting && this.validationError === null;
  }

  reset(): void {
    this.membershipId = '';
    this.tenantKey = '';
    this.registryUrl = '';
    this.kind = 'employer';
    this.deviceToken = '';
    this.projectId = '';
    this.error = null;
  }

  async submit(api: SenseiApi): Promise<boolean> {
    const invalid = this.validationError;
    if (invalid) {
      this.error = invalid;
      return false;
    }
    this.submitting = true;
    this.error = null;
    const res = await api.connectDojo(toConnectBody(this.draft));
    this.submitting = false;
    if (res.ok) {
      this.reset();
      this.open = false;
      return true;
    }
    this.error = res.error.message;
    return false;
  }
}
