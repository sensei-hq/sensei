-- is_test backfill — flag existing graph nodes that live in test files, so the UI
-- can filter tests out immediately WITHOUT a full reindex. One-time seed: the
-- indexer sets `is_test` going forward via `languages::is_test_path` (process_file
-- → set_nodes_is_test_for_file). Path-convention based, mirroring is_test_path.
--
-- Prereq: the additive `nodes.is_test` column exists (nodes.ddl / `dbd reconcile`).
-- lib_symbol/lib_package nodes (file_path NULL) are left false (external deps
-- aren't test). Idempotent (guarded on is_test = false).

UPDATE sensei.nodes
   SET is_test = true, modified_at = now()
 WHERE file_path IS NOT NULL
   AND is_test = false
   AND (
        -- test directory segments (whole segment; not a substring, so latest/ contest/ don't match)
        file_path ~ '(^|/)(tests?|__tests?__|specs?|e2e|testing)(/|$)'
        -- filename conventions: foo.test.ts / foo.spec.ts, foo_test.rs|go|py, test_*.py, conftest.py
     OR file_path ~ '\.(test|spec)\.'
     OR file_path ~ '_test\.[A-Za-z0-9]+$'
     OR file_path ~ '(^|/)test_[^/]*$'
     OR file_path ~ '(^|/)conftest\.py$'
        -- Java/Kotlin class-name suffixes (case-sensitive: IT ≠ lowercase "it" in Init/Audit)
     OR file_path ~ '(Test|Tests|IT|ITCase)\.(java|kt|kts)$'
   );
