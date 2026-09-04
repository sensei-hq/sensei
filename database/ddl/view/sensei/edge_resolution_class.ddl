set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.edge_resolution_class AS
SELECT e.id
     , e.folder_id
     , e.kind
     , CASE
         WHEN e.target_id IS NOT NULL THEN 'resolved'
         -- A name that matches exactly ONE local definition is a resolver MISS:
         -- the target was there and nothing found it. This is the only class
         -- that is a defect.
         WHEN (SELECT count(*) FROM sensei.nodes n
                WHERE n.folder_id = e.folder_id
                  AND n.name = e.target_name
                  AND n.file_path IS NOT NULL) = 1 THEN 'unambiguous-miss'
         -- Several local definitions share the name. Leaving it unresolved is
         -- CORRECT: picking one would publish a guess as a fact.
         WHEN (SELECT count(*) FROM sensei.nodes n
                WHERE n.folder_id = e.folder_id
                  AND n.name = e.target_name
                  AND n.file_path IS NOT NULL) > 1 THEN 'ambiguous'
         -- Nothing local bears the name: an external dependency, a runtime
         -- builtin, or prose naming something that does not exist. Unresolved
         -- is the truthful state, not a failure.
         ELSE 'absent'
       END AS resolution_class
  FROM sensei.edges e;

comment on view edge_resolution_class is
'Partitions every edge into resolved / unambiguous-miss / ambiguous / absent.

Exists because a single "% resolved" number cannot be read: it conflates a
resolver defect with an external dependency and with prose naming something
imaginary. MEASURED at creation over 405,787 unresolved edges — unambiguous-miss
27,495 (6.8%), ambiguous 42,236 (10.4%), absent 336,056 (82.8%) — so the raw
rates (imports 18.9%, references 30.1%, calls 65.2%) overstate the defect by
more than an order of magnitude.

- resolved: target_id set.
- unambiguous-miss: THE DEFECT CLASS. Exactly one local definition bears the
  name and resolution did not find it. Driving this toward zero is the goal;
  the biggest population is calls (18,687).
- ambiguous: two or more local definitions share the name. Correctly
  unresolved — resolving would fabricate. Not a defect, and must not be
  optimised away.
- absent: nothing local bears the name. External dependency, stdlib, or a prose
  mention. For imports this is 109,944 of 110,785 (99.2%), which is why
  externals-as-lib_symbol is the right treatment rather than better local
  resolution.

The name match is a heuristic upper bound on the defect class, not proof: a
same-named local symbol need not be the true target. It bounds the defect from
above, which is what makes it safe to use as a ceiling in a test.';
