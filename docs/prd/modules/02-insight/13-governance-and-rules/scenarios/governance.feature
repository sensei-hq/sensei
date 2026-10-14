Feature: Governance & Rules — resolution, personal file, promotion, enforcement

  # pinned: render_rules_tiers_filters_to_requested_tiers
  Scenario: resolution filters to the requested tiers
    Given a ruleset spanning several tiers
    When it is rendered for a requested tier range
    Then only the rules in range are returned

  # pinned: dedup_keeps_highest_authority_first_occurrence
  Scenario: a duplicated rule keeps the highest-authority copy
    Given the same rule at two scopes
    When the ruleset is resolved
    Then the higher-authority occurrence wins

  # pinned: mandatory_flag_only_for_mandatory
  Scenario: the mandatory flag applies only to mandatory rules
    Given rules at each enforcement level
    When they are rendered
    Then only mandatory rules carry the mandatory flag

  # pinned: render_groups_by_enforcement_strongest_first
  Scenario: rules group by enforcement, strongest first
    Given a mixed enforcement ruleset
    When it is rendered
    Then the groups order strongest to weakest

  # pinned: render_empty_ruleset_has_placeholder_not_sections
  Scenario: an empty ruleset renders a placeholder, not empty sections
    Given a project with no rules
    When its ruleset is rendered
    Then a placeholder is shown rather than blank sections

  # pinned: source_hash_stable_and_content_sensitive
  Scenario: a rule's source hash is stable and content-sensitive
    Given a rule's content
    When its source hash is computed
    Then the hash is stable for the same content
    And changes with the content

  # pinned: materialize_writes_managed_global_rules_file
  Scenario: adoption materializes into the managed global rules file
    Given an adopted rule
    When the global rules file is materialized
    Then the managed block is written

  # pinned: splice_is_a_fixed_point_on_second_run
  Scenario: the managed block is a fixed point on re-materialization
    Given a managed block already in the file
    When materialization runs again
    Then the block is replaced, not accumulated

  # pinned: only_the_default_instance_writes_the_users_claude_md
  Scenario: only the default instance writes the user-level file
    Given several instances of sensei
    When materialization runs
    Then the default instance writes the user file
    And the others do not

  # pinned: adopted_pack_rules_resolve_with_never_weaken
  Scenario: an adopted pack's rules resolve with never-weaken
    Given an adopted rule pack
    When its rules resolve against the ladder
    Then the pack rules hold their mandatory strength

  # pinned: default_constitution_import_adopts_offline_but_not_stack_templates
  Scenario: a fresh install resolves the default constitution offline
    Given a fresh database seeded by the rule-pack imports
    When the constitution resolves
    Then the three default packs are adopted at the global-dojo namespace
    And the stack templates are not adopted

  # pinned: mcp_proxy_knowledge_context_and_rules_resolve_by_project_name
  Scenario: rules resolve per repository through the MCP surface
    Given a repository name and its rules
    When get_rules is called through the proxy
    Then the rules resolve for that project

  # pinned: accept_as_rule_materializes_a_memory_and_records_provenance
  Scenario: a promoted memory becomes a rule with provenance
    Given a battle-tested memory chosen to become a rule
    When the acceptance is applied
    Then a rule materializes
    And its provenance records the source memory

  # pinned: rule_candidate_yields_memory_plus_promote_recommendation
  Scenario: a rule candidate yields a memory and a promote recommendation
    Given a distilled rule candidate
    When generation runs
    Then a memory is written
    And a promote recommendation is produced

  # pinned: most_specific_scoped_stance_beats_default_and_lower_rungs
  Scenario: the most specific scoped stance wins
    Given a default stance and a tighter-scoped stance
    When the stance resolves
    Then the most specific scoped stance is used

  # pinned: strict_fails_closed_on_every_dial
  Scenario: a strict stance fails closed on every dial
    Given a strict stance
    When autonomy is evaluated
    Then every autonomous action is refused unless permitted

  # pinned: composes_effective_rules_in_daemon_scope_vocabulary_strongest_first
  Scenario: the constitution relays rules strongest-first in daemon vocabulary
    Given a dōjō constitution
    When it is relayed to the daemon
    Then its effective rules compose strongest-first

  # pinned: parse_semantic_verdict_dangerous_true_yields_block
  Scenario: a dangerous semantic verdict blocks the action
    Given a semantic verdict of dangerous
    When the gate parses it
    Then the action is blocked

  NO TEST
  Scenario: a bypass requires an expiring, audited override
    Given a user attempting to bypass a mandatory rule
    When the bypass is requested
    Then it is refused without an override row
    And any granted override carries an expiry and an approver

  # gap: instruction-stickiness
  NO TEST
  Scenario: a mandatory rule is in context at session start without a pull
    Given an org rule at the mandatory tier
    When a session starts in that org's repo
    Then the mandatory rule is in the session context from turn zero
    And the model did not have to call get_rules

  # gap: instruction-stickiness
  NO TEST
  Scenario: an advisory rule is not pushed
    Given an advisory rule
    When a session starts
    Then the advisory rule is not injected
    And it resolves only when the model pulls it

  # gap: instruction-stickiness
  NO TEST
  Scenario: compaction re-injects the full mandatory tier
    Given a long session that has crossed compaction
    When the PreCompact hook runs
    Then the full mandatory and required tier is re-injected
    And it is not replaced by the thinner refocus summary

  # gap: instruction-stickiness
  NO TEST
  Scenario: a drift signal re-emits the mandatory tier
    Given a session where the mandatory tier has drifted out of attention
    When a drift signal fires
    Then the mandatory tier is re-emitted
    Drift-triggered re-assert is the recommended trigger, not yet a decision (design doc §7.3).

  # gap: instruction-stickiness
  NO TEST
  Scenario: the durable anchor names the resolved artifact for hostless ACPs
    Given an assistant that never runs a SessionStart hook
    When the session begins
    Then the managed CLAUDE.md/AGENTS.md pointer names the resolved artifact
    And that pointer is the backstop, not the constitution itself

  # gap: instruction-stickiness
  NO TEST
  Scenario: a mandatory tool-observable rule denies the violating call
    Given a mandatory rule that carries a machine checker
    When a tool call violates it
    Then the PreToolUse gate denies the call
    And a mandatory rule without a checker stays delivery-only