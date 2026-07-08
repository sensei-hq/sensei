//! PURE client-precedence routing.
//!
//! Given the memberships a project's work could reach and which membership the
//! project is bound to, decide which Dōjō(s) receive a contribution — and
//! whether the source must be dereferenced before it leaves the machine.
//!
//! This is the confidentiality-critical decision (it decides where potentially
//! sensitive work goes), so it is a pure function with no I/O and is heavily
//! unit-tested. The lifecycle rules it encodes (`docs/llm-spec/pipeline/
//! dojo-lifecycle.md`):
//!
//! - "Rule: **client takes precedence.** A project bound to a client membership
//!   routes findings to the client's Dōjō before the employer's, and the
//!   client's confidentiality policy governs." (Data invariants → Memberships)
//! - membership_kind.ddl: "`client` takes precedence over `employer` when a
//!   project could route to either (the client's confidentiality policy
//!   governs)."
//! - Attribution table: "Client work | **Source-dereferenced** | ... | Shareable
//!   anywhere — source dropped, not lesson." Dereference for client work is
//!   automatic; the universal strip is trusted.
//!
//! Forward seam: C4 defines and fully unit-tests this decision; its first
//! non-test caller is the upstream-contribution path (C6) — so the public
//! surface is `dead_code`-allowed at the module level until that chunk lands.
#![allow(dead_code)]
//! - Wrong gate: "Two memberships try to receive the same client project's
//!   artifacts. Precedence rule violated — **client wins uniquely.**"
//! - Wrong gate: "A client-project memory got published with its source repo
//!   identifiers intact ... this is the confidentiality gate; nothing else
//!   matters more."
//!
//! Encoded precedence (highest first): `client` > `employer` > `community` >
//! `personal`. When the project has an explicit binding that binding is the
//! routing anchor; when it does not, the highest-precedence available kind wins.

use crate::dojo::memberships::MembershipKind;

/// A membership the contribution could plausibly route to — the project's
/// binding plus any org/community memberships considered for auto-routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MembershipRef {
    pub membership_id: uuid::Uuid,
    pub kind: MembershipKind,
}

/// What is being routed. Kept minimal: routing only needs to know the project's
/// binding (`sensei.projects.dojo_id`). Everything else about the artifact
/// (payload, scope tags, attribution mode) is decided by dojo-protocol / the C5
/// attribution pass — this function decides *destination* + *dereference*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Contribution {
    /// The membership this contribution's project is bound to
    /// (`sensei.projects.dojo_id`), or `None` when the project is unbound.
    pub bound_membership: Option<uuid::Uuid>,
}

impl Contribution {
    /// A contribution from a project bound to `membership_id`.
    pub fn bound_to(membership_id: uuid::Uuid) -> Self {
        Self { bound_membership: Some(membership_id) }
    }
}

/// One resolved destination for a contribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteTarget {
    pub membership_id: uuid::Uuid,
    pub kind: MembershipKind,
    /// True when the source reference MUST be stripped before this leaves the
    /// machine. Always true for a `client` destination (client work is
    /// source-dereferenced — the confidentiality gate).
    pub dereference: bool,
}

/// The routing outcome: where the contribution goes, plus which memberships
/// were deliberately excluded (never silently dropped — the excluded employer
/// under client precedence is recorded so it is auditable and testable).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoutingDecision {
    pub targets: Vec<RouteTarget>,
    pub excluded: Vec<uuid::Uuid>,
}

impl RoutingDecision {
    /// Whether any client Dōjō is a destination.
    pub fn has_client_target(&self) -> bool {
        self.targets.iter().any(|t| t.kind == MembershipKind::Client)
    }
    /// Whether any employer Dōjō is a destination.
    pub fn has_employer_target(&self) -> bool {
        self.targets.iter().any(|t| t.kind == MembershipKind::Employer)
    }
}

fn ids_of(candidates: &[MembershipRef], kind: MembershipKind) -> Vec<uuid::Uuid> {
    candidates.iter().filter(|m| m.kind == kind).map(|m| m.membership_id).collect()
}

fn targets_of(candidates: &[MembershipRef], kind: MembershipKind, dereference: bool) -> Vec<RouteTarget> {
    candidates
        .iter()
        .filter(|m| m.kind == kind)
        .map(|m| RouteTarget { membership_id: m.membership_id, kind, dereference })
        .collect()
}

/// Decide which Dōjō(s) receive `contribution`, honoring client-precedence.
///
/// - If the project has an explicit binding, that binding is the routing anchor:
///   the contribution routes to exactly that membership. If the anchor is a
///   `client` membership the contribution is dereferenced and every `employer`
///   candidate is explicitly excluded (client wins uniquely). A binding whose
///   membership is not present in `candidates` FAILS CLOSED (routes nowhere) —
///   we never route work whose sensitivity we cannot establish.
/// - If the project is unbound, route to the highest-precedence available kind
///   (`client` > `employer` > `community` > `personal`); when `client` wins,
///   every `employer` candidate is excluded and the client target is
///   dereferenced.
pub fn client_precedence_route(
    candidates: &[MembershipRef],
    contribution: &Contribution,
) -> RoutingDecision {
    // 1. Explicit binding is the routing anchor.
    if let Some(bound) = contribution.bound_membership {
        let Some(anchor) = candidates.iter().find(|m| m.membership_id == bound).copied() else {
            // Fail closed: a binding we don't recognise must not leak anywhere.
            return RoutingDecision::default();
        };
        let is_client = anchor.kind == MembershipKind::Client;
        return RoutingDecision {
            targets: vec![RouteTarget {
                membership_id: anchor.membership_id,
                kind: anchor.kind,
                dereference: is_client,
            }],
            // Client precedence: an employer must NOT also receive client work.
            excluded: if is_client { ids_of(candidates, MembershipKind::Employer) } else { Vec::new() },
        };
    }

    // 2. Unbound — resolve by precedence among candidate kinds.
    if candidates.iter().any(|m| m.kind == MembershipKind::Client) {
        return RoutingDecision {
            targets: targets_of(candidates, MembershipKind::Client, true),
            excluded: ids_of(candidates, MembershipKind::Employer), // client wins uniquely
        };
    }
    for kind in [MembershipKind::Employer, MembershipKind::Community, MembershipKind::Personal] {
        let targets = targets_of(candidates, kind, false);
        if !targets.is_empty() {
            return RoutingDecision { targets, excluded: Vec::new() };
        }
    }
    RoutingDecision::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(kind: MembershipKind) -> MembershipRef {
        MembershipRef { membership_id: uuid::Uuid::new_v4(), kind }
    }

    #[test]
    fn client_binding_routes_to_client_dereferenced_and_excludes_employer() {
        let client = m(MembershipKind::Client);
        let employer = m(MembershipKind::Employer);
        let d = client_precedence_route(&[client, employer], &Contribution::bound_to(client.membership_id));

        assert_eq!(d.targets.len(), 1, "client work routes to exactly one Dōjō");
        assert_eq!(d.targets[0].membership_id, client.membership_id);
        assert_eq!(d.targets[0].kind, MembershipKind::Client);
        assert!(d.targets[0].dereference, "client work is source-dereferenced");
        assert!(d.has_client_target() && !d.has_employer_target(), "client wins uniquely");
        assert_eq!(d.excluded, vec![employer.membership_id], "employer explicitly excluded, not silently dropped");
    }

    #[test]
    fn client_binding_excludes_every_employer_candidate() {
        let client = m(MembershipKind::Client);
        let e1 = m(MembershipKind::Employer);
        let e2 = m(MembershipKind::Employer);
        let d = client_precedence_route(&[e1, client, e2], &Contribution::bound_to(client.membership_id));
        assert!(!d.has_employer_target());
        assert!(d.excluded.contains(&e1.membership_id) && d.excluded.contains(&e2.membership_id));
        assert_eq!(d.excluded.len(), 2);
    }

    #[test]
    fn employer_binding_routes_to_employer_named_not_dereferenced() {
        let employer = m(MembershipKind::Employer);
        let community = m(MembershipKind::Community);
        let d = client_precedence_route(&[employer, community], &Contribution::bound_to(employer.membership_id));
        assert_eq!(d.targets.len(), 1);
        assert_eq!(d.targets[0].kind, MembershipKind::Employer);
        assert!(!d.targets[0].dereference, "employer work is org-internal, not dereferenced");
        assert!(d.excluded.is_empty());
    }

    #[test]
    fn community_binding_routes_to_community() {
        let community = m(MembershipKind::Community);
        let d = client_precedence_route(&[community], &Contribution::bound_to(community.membership_id));
        assert_eq!(d.targets[0].kind, MembershipKind::Community);
        assert!(!d.targets[0].dereference);
    }

    #[test]
    fn personal_binding_routes_to_personal() {
        let personal = m(MembershipKind::Personal);
        let d = client_precedence_route(&[personal], &Contribution::bound_to(personal.membership_id));
        assert_eq!(d.targets[0].kind, MembershipKind::Personal);
        assert!(!d.targets[0].dereference);
    }

    #[test]
    fn unbound_with_client_and_employer_prefers_client_and_excludes_employer() {
        // membership_kind.ddl: "client takes precedence over employer when a
        // project could route to either."
        let client = m(MembershipKind::Client);
        let employer = m(MembershipKind::Employer);
        let d = client_precedence_route(&[employer, client], &Contribution::default());
        assert!(d.has_client_target() && !d.has_employer_target());
        assert!(d.targets.iter().all(|t| t.dereference));
        assert_eq!(d.excluded, vec![employer.membership_id]);
    }

    #[test]
    fn unbound_with_employer_and_community_prefers_employer() {
        let employer = m(MembershipKind::Employer);
        let community = m(MembershipKind::Community);
        let d = client_precedence_route(&[community, employer], &Contribution::default());
        assert_eq!(d.targets.len(), 1);
        assert_eq!(d.targets[0].kind, MembershipKind::Employer);
        assert!(d.excluded.is_empty());
    }

    #[test]
    fn unbound_with_community_and_personal_prefers_community() {
        let community = m(MembershipKind::Community);
        let personal = m(MembershipKind::Personal);
        let d = client_precedence_route(&[personal, community], &Contribution::default());
        assert_eq!(d.targets[0].kind, MembershipKind::Community);
    }

    #[test]
    fn unbound_personal_only_routes_to_personal() {
        let personal = m(MembershipKind::Personal);
        let d = client_precedence_route(&[personal], &Contribution::default());
        assert_eq!(d.targets[0].kind, MembershipKind::Personal);
    }

    #[test]
    fn no_candidates_routes_nowhere() {
        let d = client_precedence_route(&[], &Contribution::default());
        assert!(d.targets.is_empty() && d.excluded.is_empty());
    }

    #[test]
    fn unknown_binding_fails_closed() {
        // A binding whose membership isn't in the candidate set must not leak to
        // any other membership — fail closed (route nowhere).
        let employer = m(MembershipKind::Employer);
        let d = client_precedence_route(&[employer], &Contribution::bound_to(uuid::Uuid::new_v4()));
        assert!(d.targets.is_empty(), "unknown binding routes nowhere");
        assert!(d.excluded.is_empty());
    }

    #[test]
    fn employer_bound_project_ignores_a_client_membership_elsewhere() {
        // The binding — not mere presence of a client membership — decides
        // sensitivity. An employer-bound project routes to the employer even
        // when the developer also holds a client membership on another project.
        let employer = m(MembershipKind::Employer);
        let unrelated_client = m(MembershipKind::Client);
        let d = client_precedence_route(&[employer, unrelated_client], &Contribution::bound_to(employer.membership_id));
        assert_eq!(d.targets.len(), 1);
        assert_eq!(d.targets[0].kind, MembershipKind::Employer);
        assert!(!d.targets[0].dereference);
    }

    #[test]
    fn client_and_employer_never_both_receive_client_work() {
        // The core wrong-gate, asserted directly: no decision that touches a
        // client ever also targets an employer.
        let client = m(MembershipKind::Client);
        let employer = m(MembershipKind::Employer);
        for contribution in [Contribution::bound_to(client.membership_id), Contribution::default()] {
            let d = client_precedence_route(&[client, employer], &contribution);
            assert!(!(d.has_client_target() && d.has_employer_target()));
        }
    }
}
