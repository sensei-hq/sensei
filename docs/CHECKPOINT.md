# Checkpoint — 2026-11-17 (#446 merged; #447 baseline measured, floors pending)

**Slice:** spec-first relay/analysis follow-ups (`openspec/changes/`).

**Done:** #456 #460 #480 merged to develop; #395 #396 #422 #425 #446 #449 #450 #451 closed by hand (a develop
merge does not auto-close). #446: rusqlite 0.43 + dbd v0.27.10 (released; dbd#118/#120) without sqlite — audit
clean on all three lockfiles; DeepSource Rust on `crates/mcp` fixed (dispatcher split into families + tests).
#447 (`lane/447-correction-intent`): 189 real prompts labelled (2 blind labellers, κ 0.90), kept LOCAL in
`~/.sensei/eval/correction-intent-2026-11-17/` (public repo). Phrase list P 0.75 / R ~0.05; gemma4 today
P 0.64 / R 0.81; with context P 0.47 / R 0.91 (design.md).

**Next command:** on the user's floors (recommended R ≥ 0.80, P ≥ 0.75): sharpen the classifier prompt and
re-measure (`~/.sensei/eval/…/llm.py` + `score.py`), then build 2.1–3.1 red-first; #420 phase 2 after.

**Open questions (user):** #447 floors; keep the labelled set local (recommended) or commit it anonymised;
`record_outcome` turns bad `outcomes` JSON into `[]` (masking) — issue or fix?; #457 wiki.

---

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
