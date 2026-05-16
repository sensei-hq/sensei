//! Resolver trait — a Resolver self-declares which dependencies it can fix
//! via `resolves()`. The orchestrator iterates the resolver list, asks each
//! one which of its targets are failed, and runs that resolver ONCE with
//! all its targets.

use super::types::{ComponentId, Remedy};

#[derive(Debug, Clone)]
pub enum ResolveOutcome {
    /// Resolver completed successfully; orchestrator will re-check the targets.
    Resolved,
    /// Resolver couldn't fix it on its own; UI shows this Remedy to the user.
    NeedsHumanAction(Remedy),
}

pub trait Resolver: Send + Sync {
    /// Stable id (used in logs and event metadata; not user-facing).
    fn id(&self) -> &'static str;

    /// Which dependencies this resolver covers. Multiple deps may share one
    /// resolver — the orchestrator dedupes so it runs once per resolver.
    fn resolves(&self) -> &'static [ComponentId];

    /// Run the fix. `targets` is the subset of `resolves()` that's currently
    /// failed. Resolver decides whether to batch all targets in one shell-out
    /// or iterate.
    fn resolve(&self, targets: &[ComponentId]) -> ResolveOutcome;

    /// Remedy to surface when `resolve()` returned `Resolved` but the
    /// orchestrator's post-resolve re-check still reports the target(s) as
    /// Failed. Lets each resolver own the manual fallback for its own
    /// component (e.g. `brew link --force postgresql@17` when the install
    /// succeeded but the keg-only formula isn't on PATH).
    fn fallback_remedy(&self) -> Remedy;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Smoke: a tiny test impl to lock the trait shape against future changes.
    struct NoopResolver;
    impl Resolver for NoopResolver {
        fn id(&self) -> &'static str { "noop" }
        fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Daemon] }
        fn resolve(&self, _: &[ComponentId]) -> ResolveOutcome { ResolveOutcome::Resolved }
        fn fallback_remedy(&self) -> Remedy {
            Remedy { message: "fallback".into(), script: "noop".into(), url: None }
        }
    }

    #[test]
    fn trait_object_safe() {
        let r: Box<dyn Resolver> = Box::new(NoopResolver);
        assert_eq!(r.id(), "noop");
        assert_eq!(r.resolves(), &[ComponentId::Daemon]);
        assert!(matches!(r.resolve(&[ComponentId::Daemon]), ResolveOutcome::Resolved));
    }

    #[test]
    fn outcome_clone() {
        let r = ResolveOutcome::Resolved;
        let _ = r.clone();
        let r2 = ResolveOutcome::NeedsHumanAction(Remedy {
            message: "m".into(), script: "s".into(), url: None,
        });
        let _ = r2.clone();
    }
}
