//! Which documentation source to serve, and whether to label the answer
//! (02b S9). PURE — no network, no database.
//!
//! Three routes can supply a library's docs, and they are NOT equal:
//!
//! - **website** — the library's own published docs. Curated, current, and
//!   what its authors intend a reader to see.
//! - **github** — raw but complete, and the only route with VERSION HISTORY.
//! - **local** — whatever happens to be vendored in `node_modules/` or the
//!   registry cache. A real fallback, and often stripped of docs entirely.
//!
//! **THE PRECEDENCE INVERTS WHEN THE VERSION DOES NOT MATCH, and that is the
//! whole reason this is a decision and not a constant.** A website documents
//! ONE version, normally the latest. If the project pins `1.2` and the site
//! documents `3.0`, the site is the higher-quality source of the WRONG answer,
//! and github's tag for `1.2` is the correct one. So the rule is:
//!
//!     pick = the highest-precedence source THAT CAN SERVE THE PINNED VERSION
//!
//! and only when nothing can serve it does rank alone decide.
//!
//! When no source serves the pin, the closest is served and the answer is
//! LABELLED. An answer marked "this documents 3.0, you are on 1.2" is useful;
//! the same answer unlabelled is a wrong one (R4) — and it is wrong in the
//! worst way, because the caller cannot tell.

use super::version::parse_semver;

/// A documentation route, ordered by quality when the version is not in
/// question. `Ord` IS the precedence: `Website < GitHub < Local` by
/// declaration order, and lower is better.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocRoute {
    Website,
    GitHub,
    Local,
}

impl DocRoute {
    /// The `sensei.library_source_type` value this route stores as.
    #[allow(dead_code)]
    pub fn as_source_type(self) -> &'static str {
        match self {
            DocRoute::Website => "llms.txt",
            DocRoute::GitHub => "http",
            DocRoute::Local => "local",
        }
    }

    /// The route a stored `sensei.library_source_type` came from — the inverse
    /// of [`DocRoute::as_source_type`], and here rather than at the query so
    /// the two directions cannot drift apart. A value this module does not know
    /// reads as [`DocRoute::Local`], which is what an unlabelled local page is.
    pub fn of_source_type(source_type: Option<&str>) -> Self {
        match source_type {
            Some("llms.txt") => DocRoute::Website,
            Some("http") => DocRoute::GitHub,
            _ => DocRoute::Local,
        }
    }
}

/// A source that could serve this library's docs, and the version it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocCandidate {
    pub route: DocRoute,
    /// The version this source documents. `None` means UNKNOWN — the source
    /// exists but does not say which release it describes.
    pub version: Option<String>,
}

/// Why a served answer may not be what the caller asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionFit {
    /// The source serves exactly the pinned version.
    Exact,
    /// It serves a DIFFERENT version. Carries both so the label can say which.
    Mismatch { serves: String, pinned: String },
    /// The source does not state its version, so the fit cannot be confirmed.
    /// NOT the same as a mismatch: unknown is unknown, and claiming either a
    /// match or a mismatch would be inventing a fact.
    Unknown,
}

impl VersionFit {
    /// Whether an answer from this source needs labelling before a caller
    /// should trust it as describing their version.
    #[allow(dead_code)]
    pub fn needs_label(&self) -> bool {
        !matches!(self, VersionFit::Exact)
    }

    /// A human-readable label, or `None` when the answer is exact.
    pub fn label(&self) -> Option<String> {
        match self {
            VersionFit::Exact => None,
            VersionFit::Mismatch { serves, pinned } => {
                Some(format!("documents {serves}, you are on {pinned}"))
            }
            VersionFit::Unknown => {
                Some("version not stated by this source — may not match yours".to_string())
            }
        }
    }
}

/// The chosen source and how well it fits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocChoice {
    pub route: DocRoute,
    pub fit: VersionFit,
}

impl DocChoice {
    /// Whether one HELD version — its version string and the
    /// `library_source_type` it was stored under — is the one this choice
    /// names.
    ///
    /// A caller gets a choice back and still has to find the row that serves
    /// it, and that map-back belongs here rather than at the query: it has to
    /// ask the same question [`choose_docs`] asked, and a second spelling of
    /// "is this the pinned release" is a second answer waiting to disagree.
    ///
    /// The ROUTE alone never names a row. Two versions of one library are held
    /// through the same route whenever their `source_type` agrees, which is the
    /// ordinary case, so route alone admits both and a caller scanning an
    /// unordered result set takes whichever came back first. On the EXACT path
    /// that is an UNLABELLED answer about a release the caller does not run —
    /// the one outcome this module exists to prevent (R4).
    pub fn is_served_by(&self, version: &str, source_type: Option<&str>, pinned: &str) -> bool {
        DocRoute::of_source_type(source_type) == self.route
            && match &self.fit {
                VersionFit::Exact => same_version(version, pinned, &parse_semver(pinned)),
                VersionFit::Mismatch { serves, .. } => version == serves,
                // Nothing stated a version, so no version can disqualify a row.
                VersionFit::Unknown => true,
            }
    }
}

/// Pick the documentation source to serve for a pinned version (S9). PURE.
///
/// `None` when there are no candidates at all — a real state ("we hold no docs
/// for this library"), and the caller must say so rather than serve nothing
/// while implying it looked.
///
/// The ordering, in full:
///
/// 1. Sources that serve the PINNED version exactly. Among them, best route.
/// 2. Failing that, sources that state SOME version: the closest one wins, and
///    the answer is labelled. Ties break toward the LOWER version — docs for a
///    release older than yours omit things, while docs for a newer one
///    describe things that do not exist in your code, and a reader acting on
///    the second is worse off than one acting on the first.
/// 3. Failing that, a source with no stated version, labelled `Unknown`.
///
/// A pinned version that is not semver (a range like `^1.2`, or `latest`)
/// cannot be compared, so step 1 falls back to exact STRING equality and step 2
/// cannot rank — every stating source is then equally close and route decides.
pub fn choose_docs(pinned: &str, candidates: &[DocCandidate]) -> Option<DocChoice> {
    if candidates.is_empty() {
        return None;
    }
    let pinned_sv = parse_semver(pinned);

    // 1. Exact fit — the only case that needs no label.
    if let Some(c) = candidates
        .iter()
        .filter(|c| c.version.as_deref().is_some_and(|v| same_version(v, pinned, &pinned_sv)))
        .min_by_key(|c| c.route)
    {
        return Some(DocChoice { route: c.route, fit: VersionFit::Exact });
    }

    // 2. Closest stated version, labelled.
    let stating: Vec<&DocCandidate> = candidates.iter().filter(|c| c.version.is_some()).collect();
    if !stating.is_empty() {
        let best = stating
            .iter()
            .min_by(|a, b| {
                let (av, bv) = (a.version.as_deref().unwrap(), b.version.as_deref().unwrap());
                distance(av, &pinned_sv)
                    .cmp(&distance(bv, &pinned_sv))
                    // Lower version first: describing what does not exist is
                    // worse than omitting what does.
                    .then_with(|| av.cmp(bv))
                    .then_with(|| a.route.cmp(&b.route))
            })
            .unwrap();
        let serves = best.version.clone().unwrap();
        return Some(DocChoice {
            route: best.route,
            fit: VersionFit::Mismatch { serves, pinned: pinned.to_string() },
        });
    }

    // 3. Nothing states a version. Best route, labelled unknown.
    let c = candidates.iter().min_by_key(|c| c.route).unwrap();
    Some(DocChoice { route: c.route, fit: VersionFit::Unknown })
}

/// Whether `have` is the same release as `pinned`. Semver when both parse —
/// so `1.2.0` and `v1.2.0` agree — else exact string equality, because a
/// non-semver pin (`latest`, a range) has no comparable form and guessing one
/// would be the fabrication S11 warns about.
fn same_version(have: &str, pinned: &str, pinned_sv: &Option<semver::Version>) -> bool {
    match (parse_semver(have), pinned_sv) {
        (Some(h), Some(p)) => h == *p,
        _ => have.trim() == pinned.trim(),
    }
}

/// How far `have` is from the pin, for "closest available". Non-comparable
/// versions sort last rather than pretending to a distance.
fn distance(have: &str, pinned_sv: &Option<semver::Version>) -> (u8, u64, u64, u64) {
    let (Some(h), Some(p)) = (parse_semver(have), pinned_sv.as_ref()) else {
        return (1, 0, 0, 0); // incomparable — after everything comparable
    };
    (0, h.major.abs_diff(p.major), h.minor.abs_diff(p.minor), h.patch.abs_diff(p.patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(route: DocRoute, version: Option<&str>) -> DocCandidate {
        DocCandidate { route, version: version.map(str::to_string) }
    }

    /// An EXACT choice names one held version, not every version stored the
    /// same way.
    ///
    /// The two versions of a library a store holds are normally stored through
    /// the SAME route, so route alone does not tell them apart. The caller
    /// finds its row by scanning what the database returned, and that query has
    /// no total order — so a map-back on route alone serves whichever row came
    /// back first and labels it EXACT. MEASURED: this failed
    /// `docs_are_served_for_the_version_a_folder_pins_and_labelled_when_they_cannot_be`
    /// intermittently, serving 3.0.0's pages to a folder pinned at 1.2.0 with
    /// no caveat — a confident answer about an API the caller does not have,
    /// which is the one outcome this module exists to prevent (R4).
    ///
    /// The held versions are listed NEWEST FIRST on purpose: in the pinned
    /// order the bug cannot show.
    #[test]
    fn an_exact_choice_is_served_only_by_the_release_that_was_pinned() {
        let held = [("3.0.0", Some("local")), ("1.2.0", Some("local"))];
        let choice = choose_docs(
            "1.2.0",
            &[c(DocRoute::Local, Some("3.0.0")), c(DocRoute::Local, Some("1.2.0"))],
        )
        .unwrap();
        assert_eq!(choice.fit, VersionFit::Exact, "1.2.0 is held, so the fit is exact");

        let served: Vec<&str> = held
            .iter()
            .filter(|(v, st)| choice.is_served_by(v, *st, "1.2.0"))
            .map(|(v, _)| *v)
            .collect();
        assert_eq!(served, ["1.2.0"], "only the pinned release serves an exact choice");
    }

    /// `v1.2.0` and `1.2.0` are one release, so the map-back has to ask
    /// [`same_version`] rather than compare strings — the same predicate
    /// `choose_docs` used to call the fit exact.
    #[test]
    fn a_held_version_spelled_with_a_v_still_serves_the_pin_it_matches() {
        let choice = choose_docs("1.2.0", &[c(DocRoute::Local, Some("v1.2.0"))]).unwrap();
        assert_eq!(choice.fit, VersionFit::Exact);
        assert!(choice.is_served_by("v1.2.0", Some("local"), "1.2.0"));
    }

    /// A stored `library_source_type` reads back as the route that wrote it.
    #[test]
    fn a_stored_source_type_reads_back_as_the_route_that_wrote_it() {
        for route in [DocRoute::Website, DocRoute::GitHub, DocRoute::Local] {
            assert_eq!(DocRoute::of_source_type(Some(route.as_source_type())), route);
        }
        assert_eq!(DocRoute::of_source_type(None), DocRoute::Local, "an unlabelled page is local");
    }

    #[test]
    fn with_every_source_on_the_pinned_version_the_best_route_wins() {
        let got = choose_docs(
            "1.2.0",
            &[
                c(DocRoute::Local, Some("1.2.0")),
                c(DocRoute::Website, Some("1.2.0")),
                c(DocRoute::GitHub, Some("1.2.0")),
            ],
        )
        .unwrap();
        assert_eq!(got.route, DocRoute::Website, "website is the authors' own docs");
        assert_eq!(got.fit, VersionFit::Exact);
        assert!(got.fit.label().is_none(), "an exact answer needs no caveat");
    }

    #[test]
    fn precedence_inverts_when_the_better_route_documents_the_wrong_version() {
        // The heart of S9. The site is the higher-quality source of the WRONG
        // answer; github's tag for the pinned release is the right one. A rule
        // that just ranked routes would serve 3.0 docs to a 1.2 project and
        // say nothing.
        let got = choose_docs(
            "1.2.0",
            &[c(DocRoute::Website, Some("3.0.0")), c(DocRoute::GitHub, Some("1.2.0"))],
        )
        .unwrap();
        assert_eq!(got.route, DocRoute::GitHub);
        assert_eq!(got.fit, VersionFit::Exact);
    }

    #[test]
    fn when_nothing_serves_the_pin_the_closest_is_served_and_labelled() {
        // R4: an unlabelled wrong-version answer is the failure, and it is the
        // worst kind — the caller cannot tell.
        let got = choose_docs(
            "1.2.0",
            &[c(DocRoute::Website, Some("3.0.0")), c(DocRoute::GitHub, Some("1.4.0"))],
        )
        .unwrap();
        assert_eq!(got.route, DocRoute::GitHub, "1.4 is closer to 1.2 than 3.0 is");
        assert_eq!(
            got.fit,
            VersionFit::Mismatch { serves: "1.4.0".into(), pinned: "1.2.0".into() }
        );
        assert_eq!(got.fit.label().as_deref(), Some("documents 1.4.0, you are on 1.2.0"));
    }

    #[test]
    fn an_equally_distant_older_version_beats_a_newer_one() {
        // 1.1 and 1.3 are both one minor from 1.2. Docs for an older release
        // omit things; docs for a newer one describe things that do not exist
        // in your code, and acting on those is worse.
        let got = choose_docs(
            "1.2.0",
            &[c(DocRoute::Website, Some("1.3.0")), c(DocRoute::Website, Some("1.1.0"))],
        )
        .unwrap();
        assert_eq!(
            got.fit,
            VersionFit::Mismatch { serves: "1.1.0".into(), pinned: "1.2.0".into() }
        );
    }

    #[test]
    fn a_source_that_does_not_state_its_version_is_unknown_not_a_match() {
        // Claiming either a match or a mismatch would be inventing a fact.
        let got = choose_docs("1.2.0", &[c(DocRoute::Local, None)]).unwrap();
        assert_eq!(got.route, DocRoute::Local);
        assert_eq!(got.fit, VersionFit::Unknown);
        assert!(got.fit.needs_label(), "unconfirmed is not confirmed");
    }

    #[test]
    fn a_stated_version_is_preferred_over_an_unstated_one_even_from_a_better_route() {
        // A known mismatch can be labelled precisely; an unknown cannot be
        // labelled at all. The caller is better served by the one it can
        // reason about.
        let got =
            choose_docs("1.2.0", &[c(DocRoute::Website, None), c(DocRoute::Local, Some("1.3.0"))])
                .unwrap();
        assert_eq!(got.route, DocRoute::Local);
        assert!(matches!(got.fit, VersionFit::Mismatch { .. }));
    }

    #[test]
    fn a_v_prefix_is_the_same_release() {
        let got = choose_docs("1.2.0", &[c(DocRoute::GitHub, Some("v1.2.0"))]).unwrap();
        assert_eq!(got.fit, VersionFit::Exact, "v1.2.0 and 1.2.0 are one release");
    }

    #[test]
    fn a_non_semver_pin_falls_back_to_exact_string_equality() {
        // `latest` and ranges have no comparable form. Matching them by string
        // is honest; inventing an ordering for them is not.
        let exact = choose_docs("latest", &[c(DocRoute::Website, Some("latest"))]).unwrap();
        assert_eq!(exact.fit, VersionFit::Exact);

        let got = choose_docs("latest", &[c(DocRoute::Website, Some("3.0.0"))]).unwrap();
        assert!(matches!(got.fit, VersionFit::Mismatch { .. }), "not comparable, so labelled");
    }

    #[test]
    fn no_candidates_is_none_not_a_fabricated_choice() {
        // "We hold no docs for this library" is a real state the caller must
        // be able to say.
        assert_eq!(choose_docs("1.2.0", &[]), None);
    }

    #[test]
    fn the_route_order_is_the_documented_precedence() {
        assert!(DocRoute::Website < DocRoute::GitHub);
        assert!(DocRoute::GitHub < DocRoute::Local);
        assert_eq!(DocRoute::Website.as_source_type(), "llms.txt");
        assert_eq!(DocRoute::GitHub.as_source_type(), "http");
        assert_eq!(DocRoute::Local.as_source_type(), "local");
    }
}
