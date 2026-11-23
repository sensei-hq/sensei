## ADDED Requirements

### Requirement: Concurrent compiler subprocesses are capped globally

At most 2 compiler subprocesses (any engine, any repo) SHALL run at once;
further compiles SHALL wait for a permit instead of spawning. The cap is a
named constant beside the compile call.

#### Scenario: A scan storm does not saturate the machine

- **WHEN** 5 repos queue compiles simultaneously
- **THEN** at most 2 subprocesses run at any instant and all 5 complete
  with correct indexes

### Requirement: One flight per compilation root

Two concurrent compiles of the same (engine, root) SHALL share one
subprocess; the second caller awaits the first's result instead of
spawning. Keyed on the canonical root the locator returns.

#### Scenario: Duplicate compiles collapse

- **WHEN** two tasks compile the same package concurrently
- **THEN** the engine runs once and both callers receive its result

### Requirement: Skips never regress working packages (reversal record)

The governor SHALL contain storms by capping concurrency and SHALL NOT
skip compilable work to do it. (A tsconfig-less fast-skip was implemented
and REVERTED: packages that compile fine without a tsconfig regressed
from resolved to `engine_not_run`, caught by
`process_file_ts_emits_fqn_nodes`. Root-causing the ts-driver hang on
some tsconfig-less input is a separate follow-up, not this change.)

#### Scenario: A working config-less package still compiles

- **WHEN** a `package.json`-only fixture compiles through the driver
- **THEN** its calls resolve exactly as before this change
