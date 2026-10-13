Feature: Distribution — the macOS/Homebrew/bootstrap health path (built)

  # pinned: builds_a_checker_for_every_component_id
  Scenario: the macOS platform builds a checker for every component id
    Given the macOS platform checker set
    When it is inspected
    Then it has a checker for every component id

  # pinned: each_brew_install_resolver_covers_one_component
  Scenario: each brew install resolver covers one component
    Given the brew install resolvers
    When they are inspected
    Then each covers exactly one component

  # pinned: default_remedy_uses_prod_formula_no_head
  Scenario: the default remedy uses the prod formula without --head
    Given a component needing install
    When the default remedy is built
    Then it uses the prod formula without --head

  # pinned: package_manager_is_homebrew
  Scenario: the macOS package manager is Homebrew
    Given the macOS platform
    When its package manager is queried
    Then it is Homebrew

  # pinned: platform_is_macos_or_linux
  Scenario: the platform is detected as macOS or Linux
    Given the platform detection
    When it runs on macOS
    Then it returns macOS
    When it runs on Linux
    Then it returns Linux

  # pinned: provides_five_resolvers
  Scenario: the macOS platform provides five resolvers
    Given the macOS platform
    When its resolvers are counted
    Then there are five

  # pinned: package_manager_for_platform_returns_winget_on_windows
  Scenario: the Windows package manager is winget
    Given the platform detection on Windows
    When the package manager is queried
    Then it is winget

  # pinned: package_manager_id_str_for_each_variant
  Scenario: each package manager variant has a string id
    Given the package manager variants
    When their ids are read
    Then each has a string id

  # pinned: parse_component_id_round_trip
  Scenario: component ids parse and round-trip
    Given a component id string
    When it is parsed and serialised
    Then it round-trips

  # pinned: pool_warm_floor_is_below_the_ceiling
  Scenario: the pool warm floor is below the ceiling
    Given the pool config
    When the warm floor and ceiling are compared
    Then the floor is below the ceiling

  # pinned: pool_max_lifetime_drives_prompt_reclaim
  Scenario: the pool max lifetime drives prompt reclaim
    Given a pool with max lifetime set
    When a connection exceeds it
    Then it is reclaimed

  # pinned: home_dir_is_not_empty
  Scenario: the home directory is not empty
    Given the home directory detection
    When it runs
    Then it returns a non-empty path

  # pinned: sensei_tap_formula_is_prod_tap
  Scenario: the sensei tap formula is the prod tap
    Given the sensei tap
    When its formula is inspected
    Then it is the prod tap

  # pinned: db_url_format_round_trips
  Scenario: the database URL format round-trips
    Given a database URL
    When it is parsed and formatted
    Then it round-trips

  # pinned: config_instance_override_and_defaults
  Scenario: config instance respects override and defaults
    Given a config with override and defaults
    When the instance is created
    Then override wins and defaults fill gaps

  # pinned: binary_names
  Scenario: the binary names are correct
    Given the binary names constant
    When it is inspected
    Then it matches the expected names

  # pinned: db_url_is_well_formed
  Scenario: the database URL is well-formed
    Given a database URL from config
    When it is validated
    Then it is well-formed

  # pinned: const_binary_names_match_accessors
  Scenario: the constant binary names match the accessors
    Given the binary name accessors
    When they are compared to the constants
    Then they match

  # pinned: mcp_registry_key_is_sensei
  Scenario: the MCP registry key is "sensei"
    Given the MCP registry key constant
    When it is read
    Then it is "sensei"

  # pinned: a_retention_setting_can_never_mean_keep_nothing
  Scenario: a retention setting can never mean keep nothing
    Given any retention setting
    When it is interpreted
    Then it never means keep nothing

  # pinned: brew_install_script_has_no_head_flag
  Scenario: the brew install script has no --head flag
    Given the brew install script
    When it is inspected
    Then it does not contain --head

  # pinned: only_engines_that_are_not_ready_are_installed
  Scenario: only engines that are not ready are installed
    Given a set of engines
    When the install logic runs
    Then only not-ready engines are installed

  # pinned: a_probe_that_timed_out_is_its_own_cause_not_ready
  Scenario: a probe that timed out is its own cause, not ready
    Given a probe that timed out
    When its cause is classified
    Then it is its own cause, not "not ready"

  # pinned: rust_analyzer_unresolved_carries_what_rustup_said
  Scenario: an unresolved rust-analyzer carries what rustup said
    Given a rust-analyzer probe that is unresolved
    When its payload is read
    Then it carries rustup's message

  # pinned: rust_is_ready_when_both_components_are_there
  Scenario: Rust is ready when both components are there
    Given rustup and rust-src are installed
    When Rust readiness is checked
    Then it is ready

  # pinned: rust_src_absent_from_the_installed_list_is_named
  Scenario: rust-src absent from the installed list is named
    Given rustup is installed but rust-src is not
    When the installed list is read
    Then rust-src is named as absent

  # pinned: a_package_manifest_without_a_version_is_unreadable
  Scenario: a package manifest without a version is unreadable
    Given a manifest without a version
    When it is parsed
    Then it is unreadable

  # pinned: a_manifest_without_dependencies_is_an_error_not_an_empty_list
  Scenario: a manifest without dependencies is an error, not an empty list
    Given a manifest without dependencies
    When it is parsed
    Then it is an error

  # pinned: without_rustup_the_one_cause_is_rustup
  Scenario: without rustup, the one cause is rustup
    Given rustup is not installed
    When the causes are read
    Then the only cause is rustup

  # pinned: a_package_at_its_pinned_version_is_ready
  Scenario: a package at its pinned version is ready
    Given a package at the pinned version
    When its readiness is checked
    Then it is ready

  # pinned: a_package_at_another_version_names_both
  Scenario: a package at another version names both
    Given a package at a non-pinned version
    When its version is read
    Then both the pinned and actual versions are named

  # pinned: a_manifest_other_than_the_pinned_one_is_stale
  Scenario: a manifest other than the pinned one is stale
    Given a manifest that is not the pinned one
    When it is compared
    Then it is stale

  # pinned: an_installed_engines_directory_has_no_causes
  Scenario: an installed engines directory has no causes
    Given an engines directory where all are installed
    When causes are read
    Then there are none

  # pinned: the_manifest_is_compared_as_json_not_as_bytes
  Scenario: the manifest is compared as JSON, not as bytes
    Given two manifests with same JSON, different formatting
    When they are compared
    Then they are equal

  # pinned: an_empty_engines_directory_names_the_manifest_and_every_package
  Scenario: an empty engines directory names the manifest and every package
    Given an empty engines directory
    When it is inspected
    Then the manifest and every package are named

  # pinned: the_embedded_manifest_pins_every_package_the_driver_loads
  Scenario: the embedded manifest pins every package the driver loads
    Given the embedded manifest
    When it is inspected
    Then every package the driver loads is pinned

  # pinned: a_package_at_another_version_is_named_in_the_directory
  Scenario: a package at another version is named in the directory
    Given an engines directory with a non-pinned version
    When it is inspected
    Then that package is named

  # pinned: the_embedded_manifest_pins_every_package_the_driver_loads
  Scenario: the embedded manifest pins every package the driver loads
    Given the embedded manifest
    When it is inspected
    Then every package the driver loads is pinned

  # pinned: model_name_matching
  Scenario: model names match correctly
    Given model names to match
    When matching runs
    Then they match correctly

  # pinned: model_name_matching_not_found
  Scenario: model names not found are handled
    Given a model name that does not exist
    When matching runs
    Then it is handled correctly

  # pinned: required_models_minimum_includes_embedded_defaults
  Scenario: required models minimum includes embedded defaults
    Given the required models list
    When it is inspected
    Then it includes the embedded defaults

  # pinned: required_models_recommended_includes_embedded_defaults_and_reasoning
  Scenario: required models recommended includes embedded defaults and reasoning
    Given the recommended models list
    When it is inspected
    Then it includes embedded defaults and reasoning

  # pinned: app_version_matches_cargo_pkg_version
  Scenario: the app version matches the Cargo package version
    Given the app version
    When it is compared to Cargo pkg version
    Then they match

  # pinned: covers_database_only
  Scenario: the db setup resolver covers database only
    Given the db setup resolver
    When its coverage is checked
    Then it covers database only

  # pinned: failed_remedy_for_missing_pg_binary_points_at_install_link_rather_than_createdb
  Scenario: failed remedy for missing pg binary points at install link, not createdb
    Given pg binary is missing
    When the failed remedy is built
    Then it points at the install link, not createdb

  # pinned: failed_remedy_uses_db_name_in_script
  Scenario: the failed remedy uses the db name in the script
    Given a db setup failure
    When the remedy script is built
    Then it uses the db name

  # pinned: fallback_remedy_carries_db_name_and_manual_script
  Scenario: the fallback remedy carries the db name and manual script
    Given a db setup failure
    When the fallback remedy is built
    Then it carries the db name and manual script

  # pinned: id_is_db_setup
  Scenario: the db setup resolver id is "db_setup"
    Given the db setup resolver
    When its id is read
    Then it is "db_setup"

  # pinned: does_not_cover_others
  Scenario: each resolver does not cover others
    Given the resolvers
    When their coverage is checked
    Then each covers only its own component

  # pinned: fallback_remedy_restarts_service
  Scenario: the ollama fallback remedy restarts the service
    Given ollama is not running
    When the fallback remedy runs
    Then it restarts the service

  # pinned: id_is_ollama_install
  Scenario: the ollama install resolver id is "ollama_install"
    Given the ollama install resolver
    When its id is read
    Then it is "ollama_install"

  # pinned: resolves_only_ollama
  Scenario: the ollama install resolver resolves only ollama
    Given the ollama install resolver
    When its coverage is checked
    Then it resolves only ollama

  # pinned: id_is_postgres_install
  Scenario: the postgres install resolver id is "postgres_install"
    Given the postgres install resolver
    When its id is read
    Then it is "postgres_install"

  # pinned: resolves_only_postgres
  Scenario: the postgres install resolver resolves only postgres
    Given the postgres install resolver
    When its coverage is checked
    Then it resolves only postgres

  # pinned: id_is_sensei_install
  Scenario: the sensei install resolver id is "sensei_install"
    Given the sensei install resolver
    When its id is read
    Then it is "sensei_install"

  # pinned: resolves_only_sensei
  Scenario: the sensei install resolver resolves only sensei
    Given the sensei install resolver
    When its coverage is checked
    Then it resolves only sensei

  # pinned: does_not_cover_others
  Scenario: the sensei install resolver does not cover others
    Given the sensei install resolver
    When its coverage is checked
    Then it does not cover others

  # pinned: fallback_remedy_uses_brew_install_script
  Scenario: the sensei fallback remedy uses the brew install script
    Given sensei is not installed
    When the fallback remedy is built
    Then it uses the brew install script

  # pinned: spec_can_be_const_constructed
  Scenario: the service cascade spec can be const-constructed
    Given the service cascade spec
    When it is const-constructed
    Then it succeeds

  # pinned: stage_2_is_optional
  Scenario: stage 2 of the service cascade is optional
    Given the service cascade
    When stage 2 is checked
    Then it is optional

  # pinned: check_and_resolve_emits_initial_report_before_resolving
  Scenario: check and resolve emits initial report before resolving
    Given a health check
    When check_and_resolve runs
    Then the initial report is emitted before resolving

  # pinned: resolve_emits_phase_and_terminal_report_when_not_ok
  Scenario: resolve emits phase and terminal report when not ok
    Given a health check that is not ok
    When resolve runs
    Then it emits phase and terminal reports

  # pinned: check_returns_validated_payload
  Scenario: check returns a validated payload
    Given a health check
    When check runs
    Then it returns a validated payload

  # pinned: check_traced_records_binary_probes
  Scenario: traced check records binary probes
    Given a traced health check
    When it runs
    Then binary probes are recorded

  # pinned: upgrade_event_helpers_set_expected_fields
  Scenario: upgrade event helpers set expected fields
    Given the upgrade event helpers
    When they are used
    Then they set expected fields

  # pinned: run_all_steps_succeed_returns_true
  Scenario: run all steps succeed returns true
    Given an upgrade that succeeds
    When all steps run
    Then it returns true

  # pinned: run_prereqs_failure_does_not_skip_db_deploy
  Scenario: prereq failure does not skip db deploy
    Given an upgrade where prereqs fail
    When it runs
    Then db deploy still runs

  # pinned: run_emits_in_canonical_order
  Scenario: upgrade emits in canonical order
    Given an upgrade
    When it runs
    Then the steps emit in canonical order

  # pinned: user_local_bin_dirs_includes_qlty_home_dir
  Scenario: user local bin dirs includes qlty home dir
    Given the user local bin dirs
    When they are listed
    Then the qlty home dir is included

  # pinned: which_binary_falls_back_to_homebrew_opt_for_keg_only_postgres
  Scenario: which binary falls back to homebrew opt for keg-only postgres
    Given postgres is keg-only
    When which_binary runs
    Then it falls back to homebrew opt

  # pinned: which_binary_finds_ls_on_unix
  Scenario: which binary finds ls on Unix
    Given a Unix system
    When which_binary runs for ls
    Then it finds it

  # pinned: which_binary_resolves_qlty_when_installed_in_user_local_dir
  Scenario: which binary resolves qlty when installed in user local dir
    Given qlty is installed in user local dir
    When which_binary runs for qlty
    Then it resolves it

  # pinned: which_binary_returns_none_for_missing
  Scenario: which binary returns none for missing binary
    Given a binary that does not exist
    When which_binary runs
    Then it returns none

  # pinned: only_the_tables_a_plan_alters_are_backed_up_not_its_enums
  Scenario: only the tables a plan alters are backed up, not its enums
    Given a plan that alters tables
    When backup runs
    Then only those tables are backed up, not enums

  # pinned: database_exists_does_not_partial_match
  Scenario: database exists does not partial match
    Given a database name
    When existence is checked
    Then it does not partial match

  # pinned: database_exists_parses_psql_output
  Scenario: database exists parses psql output
    Given psql output
    When it is parsed
    Then it parses correctly

  # pinned: db_url_format_round_trips
  Scenario: db URL format round-trips
    Given a db URL
    When it is parsed and formatted
    Then it round-trips

  # pinned: db_url_contains_db_name
  Scenario: db URL contains the db name
    Given a db URL
    When it is inspected
    Then it contains the db name

  # pinned: db_schema_source_default_and_override
  Scenario: db schema source respects default and override
    Given a db schema source with default and override
    When it is read
    Then override wins and default fills gaps