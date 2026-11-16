# Checkpoint — 2026-11-16 (DeepSource low-hanging fixes on develop; #460 merged)

**Slice:** DeepSource review findings → tracking issues #461–#479, then fix the
low-hanging ones on `develop` (this session, rebased over #460).

**Done (pushed):** `2e9b6c91d` (20 files: shell quoting/disables, Rust
T::new/as_bool/create_dir_all/tmp-free fixtures, static attempt, attrOf move,
writebackText default via TDD, quote swaps, openSidebarEntry, vi.fn mocks,
skipcq with reasons, `.deepsource.toml` with test_patterns + 2 .mjs excludes)
and `df9649198` (the 2 rustfmt violations #460 landed with; CI lint was red).
Full suite green: cargo 5532/0, vitest 3739/0, svelte-check, clippy, fmt,
test-fast, 3× audit. Closed #466, #468–#470, #472–#478.
Kept open with verdicts: #462–#464, #467, #471 (dashboard ignore-clicks or
test_patterns confirmation on next run), #461, #465 (next batch), #479
(umbrella: secrets scanners clean, dashboard triage needed).

**Carried forward:** release PR #459 open (develop→main v0.23.0); #460 merged.
dbd#118 + dbd v0.27.10 release, then move the four dbd pins and land
`wip/446-sqlite-cve` (#446); then #447, #457.

**Next command:** review + merge #459; dashboard ignore-clicks for
#462–#464, #467, #471; then #461 (need() sweep) and #465 (refactors).

**Open questions (user):** v0.23.0 release decision; the dbd release; #447
floors; #457.
