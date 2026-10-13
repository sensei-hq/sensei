Feature: Dōjō Core — membership, routing, federation and governance resolution

  # pinned: membership_kind_db_strings_match_ddl_and_round_trip
  Scenario: membership kind DB strings match the DDL enum and round-trip
    Given the MembershipKind enum
    When the DB strings are compared to the DDL and round-tripped
    Then they match

  # pinned: derive_dojo_url_joins_registry_and_tenant_without_double_slash
  Scenario: the Dōjō URL joins registry and tenant without double slash
    Given a registry and a tenant key
    When the Dōjō URL is derived
    Then it joins them without a double slash

  # pinned: dojo_run_url_builds_the_auth_gated_run_detail_link
  Scenario: the Dōjō run URL builds the auth-gated run detail link
    Given a run ID and a tenant
    When the run URL is built
    Then it includes the auth-gated path

  # pinned: normalize_org_slugs_lowercases_trims_dedupes_and_drops_empties
  Scenario: org slugs are normalized — lowercase, trim, dedupe, drop empties
    Given a list of org slugs with mixed case, whitespace, duplicates, empties
    When they are normalized
    Then they are lowercased, trimmed, deduped and empties dropped

  # pinned: new_connection_validates_every_enum_field
  Scenario: a new connection validates every enum field
    Given a connection payload
    When it is validated
    Then every enum field is checked

  # pinned: client_binding_routes_to_client_dereferenced_and_excludes_employer
  Scenario: a client binding routes to the client Dōjō and excludes the employer
    Given a repo bound to a client membership
    When the binding is inferred
    Then it routes to the client Dōjō and the employer is excluded

  # pinned: client_binding_excludes_every_employer_candidate
  Scenario: a client binding excludes every employer candidate
    Given a repo with both client and employer memberships
    When the binding is inferred
    Then the client membership wins and all employer candidates are excluded

  # pinned: employer_binding_routes_to_employer_named_not_dereferenced
  Scenario: an employer binding routes to the employer (named, not dereferenced)
    Given a repo bound to an employer membership
    When the binding is inferred
    Then it routes to the employer Dōjō by name

  # pinned: community_binding_routes_to_community
  Scenario: a community binding routes to the community Dōjō
    Given a repo bound to a community membership
    When the binding is inferred
    Then it routes to the community Dōjō

  # pinned: personal_binding_routes_to_personal
  Scenario: a personal binding routes to the personal Dōjō
    Given a repo with only a personal membership
    When the binding is inferred
    Then it routes to the personal Dōjō

  # pinned: unbound_with_client_and_employer_prefers_client_and_excludes_employer
  Scenario: unbound with client and employer prefers client and excludes employer
    Given an unbound repo with both client and employer memberships
    When the binding is inferred
    Then it prefers the client and excludes the employer

  # pinned: unbound_with_employer_and_community_prefers_employer
  Scenario: unbound with employer and community prefers employer
    Given an unbound repo with employer and community memberships
    When the binding is inferred
    Then it prefers the employer

  # pinned: unbound_with_community_and_personal_prefers_community
  Scenario: unbound with community and personal prefers community
    Given an unbound repo with community and personal memberships
    When the binding is inferred
    Then it prefers the community

  # pinned: unbound_personal_only_routes_to_personal
  Scenario: unbound with only personal routes to personal
    Given an unbound repo with only a personal membership
    When the binding is inferred
    Then it routes to the personal Dōjō

  # pinned: no_candidates_routes_nowhere
  Scenario: no candidates routes nowhere
    Given an unbound repo with no memberships
    When the binding is inferred
    Then it routes nowhere

  # pinned: unknown_binding_fails_closed
  Scenario: an unknown binding fails closed
    Given a binding kind that is not recognized
    When the binding is inferred
    Then it fails closed (routes nowhere)

  # pinned: employer_bound_project_ignores_a_client_membership_elsewhere
  Scenario: an employer-bound project ignores a client membership elsewhere
    Given a project bound to an employer Dōjō, and the user has a client membership for another repo
    When the binding is inferred for the employer-bound project
    Then it ignores the client membership

  # pinned: client_and_employer_never_both_receive_client_work
  Scenario: client and employer never both receive client work
    Given a repo that is client work
    When the contribution is routed
    Then only the client Dōjō receives it, never the employer

  # pinned: infer_binding_matches_owner_to_membership_org
  Scenario: infer binding matches the repo owner to the membership org
    Given a repo owned by an org and a membership for that org
    When the binding is inferred
    Then it matches the owner to the membership org

  # pinned: infer_binding_is_case_insensitive_on_both_sides
  Scenario: infer binding is case-insensitive on both sides
    Given a repo owner and membership org with different casing
    When the binding is inferred
    Then it matches case-insensitively

  # pinned: infer_binding_prefers_client_over_employer_when_both_cover_the_owner
  Scenario: infer binding prefers client over employer when both cover the owner
    Given a repo whose owner is covered by both a client and an employer membership
    When the binding is inferred
    Then it prefers the client membership

  # pinned: token_words_splits_camel_snake_kebab
  Scenario: token words split camel, snake, and kebab case
    Given identifiers in camelCase, snake_case, kebab-case
    When token words are extracted
    Then all three are split correctly

  # pinned: every_repo_form_is_stripped
  Scenario: every repo form is stripped by the anonymizer
    Given a text with various repo URL forms (github.com/org/repo, org/repo, etc.)
    When the anonymizer runs
    Then every repo form is stripped

  # pinned: absolute_paths_all_forms_stripped
  Scenario: absolute paths in all forms are stripped
    Given absolute paths (/home/user/project, C:\Users\project, etc.)
    When the anonymizer runs
    Then all absolute path forms are stripped

  # pinned: uuid_email_session_are_stripped
  Scenario: UUIDs, emails, and session IDs are stripped
    Given text containing UUIDs, email addresses, session IDs
    When the anonymizer runs
    Then they are all stripped

  # pinned: client_and_person_and_project_are_stripped
  Scenario: client, person, and project identifiers are stripped
    Given text with client names, person names, project names
    When the anonymizer runs
    Then they are stripped

  # pinned: known_token_embedded_mid_sentence_and_mid_path_is_removed
  Scenario: a known token embedded mid-sentence and mid-path is removed
    Given a token embedded in prose and in a path
    When the anonymizer runs
    Then it is removed in both contexts

  # pinned: residual_risk_flags_surviving_absolute_path
  Scenario: residual risk flags a surviving absolute path
    Given an absolute path that the anonymizer could not strip
    When the anonymizer runs
    Then it is flagged as residual risk

  # pinned: composes_effective_rules_in_daemon_scope_vocabulary_strongest_first
  Scenario: the effective rules compose in daemon scope vocabulary, strongest first
    Given rules at multiple scopes with different enforcement
    When they are resolved
    Then the daemon-scope vocabulary orders them strongest first

  # pinned: dedup_records_the_discarded_weaker_duplicate_as_a_conflict
  Scenario: dedup records the discarded weaker duplicate as a conflict
    Given two rules with the same content but different enforcement
    When they are deduped
    Then the weaker is discarded and recorded as a conflict

  # pinned: skips_empty_content
  Scenario: rules with empty content are skipped
    Given a rule with empty content
    When resolution runs
    Then it is skipped

  # pinned: gated_tools_parses_and_trims
  Scenario: gated tools parses and trims the config
    Given a gated tools config string
    When it is parsed
    Then it is trimmed and split correctly

  # pinned: gated_tools_empty_when_unset_or_blank
  Scenario: gated tools is empty when unset or blank
    Given an unset or blank gated tools config
    When it is parsed
    Then the result is empty

  # pinned: should_gate_only_listed_tools_case_sensitive
  Scenario: should_gate only applies to listed tools, case-sensitive
    Given a tool call for a tool not in the gated list
    When should_gate is checked
    Then it returns false
    Given a tool call differing only in case
    When should_gate is checked
    Then it returns false

  # pinned: should_gate_false_when_list_empty
  Scenario: should_gate is false when the list is empty
    Given an empty gated tools list
    When should_gate is checked
    Then it returns false

  # pinned: decision_deny_only_on_explicit_deny
  Scenario: the gate decision denies only on explicit deny
    Given a gate decision that is not explicit deny
    When the decision is evaluated
    Then it does not deny

  # pinned: decision_allow_on_approve_or_anything_else
  Scenario: the gate decision allows on approve or anything else
    Given a gate decision of approve or any non-deny value
    When the decision is evaluated
    Then it allows

  # pinned: decision_allow_on_none_timeout
  Scenario: the gate decision allows on none/timeout
    Given a gate decision of none or a timeout
    When the decision is evaluated
    Then it allows

  # pinned: hard_block_main_branch_pushes
  Scenario: a main branch push is a hard block
    Given a bash command `git push --force origin main`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_history_rewrite
  Scenario: a history rewrite is a hard block
    Given a bash command `git rebase -i HEAD~5`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_destructive_deletes
  Scenario: a destructive delete is a hard block
    Given a bash command `rm -rf /`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_deploy_publish_bump
  Scenario: a deploy/publish/bump is a hard block
    Given a bash command `npm publish` or `aws deploy` or `cargo publish`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_credentials
  Scenario: credential exposure is a hard block
    Given a bash command with `AWS_SECRET_ACCESS_KEY=...`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_money_providers
  Scenario: a money provider command is a hard block
    Given a bash command with `stripe` or `paypal` or `braintree`
    When the gate classifies
    Then it is a hard block

  # pinned: allow_normal_progress_commands
  Scenario: normal progress commands are allowed
    Given a bash command like `cargo test` or `npm run build`
    When the gate classifies
    Then it is allowed

  # pinned: rm_rf_safe_build_dirs_are_allowed
  Scenario: rm -rf on safe build dirs is allowed
    Given a bash command `rm -rf target/` or `rm -rf dist/`
    When the gate classifies
    Then it is allowed

  # pinned: hard_block_classifier_gap_fixes_p3_5
  Scenario: the hard block classifier fixes the P3.5 gaps
    Given the P3.5 gap commands
    When the gate classifies
    Then they are blocked

  # pinned: hard_block_security_review_p3_fixes
  Scenario: the hard block security review fixes the P3 gaps
    Given the P3 security review commands
    When the gate classifies
    Then they are blocked

  # pinned: hard_block_write_edit_to_secret_files
  Scenario: a write/edit to a secret file is a hard block
    Given a write/edit to `.env` or `secrets.yaml`
    When the gate classifies
    Then it is a hard block

  # pinned: allow_write_edit_to_normal_source
  Scenario: a write/edit to normal source is allowed
    Given a write/edit to `src/main.rs`
    When the gate classifies
    Then it is allowed

  # pinned: other_tools_and_malformed_input_are_progress
  Scenario: other tools and malformed input are progress
    Given a non-bash tool or malformed input
    When the gate classifies
    Then it is progress

  # pinned: word_match_respects_boundaries
  Scenario: word matching respects boundaries
    Given a needle that is a substring of another word
    When word match runs
    Then it does not match

  # pinned: semantic_review_flags_obfuscation_markers
  Scenario: semantic review flags obfuscation markers
    Given a bash command with `base64 -d` or `eval` or `curl | sh`
    When the semantic review runs
    Then it flags for review

  # pinned: semantic_review_stays_rare_on_benign
  Scenario: semantic review stays rare on benign commands
    Given a benign bash command
    When the semantic review runs
    Then it does not flag

  # pinned: semantic_review_ok_to_overlap_deterministic_block
  Scenario: semantic review may overlap with deterministic block
    Given a command that is both a hard block and semantically suspicious
    When the gate classifies
    Then both apply

  # pinned: semantic_review_only_bash
  Scenario: semantic review only runs on bash
    Given a non-bash tool call
    When the semantic review runs
    Then it does not run

  # pinned: parse_semantic_verdict_dangerous_true_yields_block
  Scenario: a semantic verdict with dangerous=true yields a block
    Given a semantic verdict with dangerous: true
    When it is parsed
    Then it yields a block

  # pinned: parse_semantic_verdict_dangerous_false_is_none
  Scenario: a semantic verdict with dangerous=false is none
    Given a semantic verdict with dangerous: false
    When it is parsed
    Then it is none

  # pinned: parse_semantic_verdict_tolerates_fences_and_prose
  Scenario: semantic verdict parsing tolerates fences and prose
    Given a verdict wrapped in markdown fences or prose
    When it is parsed
    Then it is tolerated

  # pinned: parse_semantic_verdict_unknown_category_maps_to_default
  Scenario: an unknown semantic category maps to default
    Given a verdict with an unknown category
    When it is parsed
    Then it maps to default

  # pinned: parse_semantic_verdict_missing_reason_is_none
  Scenario: a missing reason is none
    Given a verdict with no reason
    When it is parsed
    Then the reason is none

  # pinned: parse_semantic_verdict_garbage_is_none
  Scenario: garbage input is none
    Given garbage input
    When it is parsed
    Then it is none

  # pinned: semantic_prompt_builders_are_structured
  Scenario: semantic prompt builders are structured
    Given the semantic prompt builders
    When they are inspected
    Then they are structured