We will go step by step. first with bootstrap.

  Problem 1. sensei/crates/bootstrap/src/prereq/factory.rs
  This file should import the config and use the binary name from the config
  for (sensei, senseid and sensei-mcp sensei mcp is missing.

  the code is repeating itself. Ideally i would prefer if we do

  match provider.platform return the platform specific handler either
  brewHandler or wingetHandler.

  Both of these handlers should receive the same array of prerequisites

  instead of repeating the same thing 4 times x2 we should build a graph

  1. homebrew/winget is the base for mac/linux or windows. This should be
  internal knowledge for both.
  2. The remaining apps are postgres, ollama and sensei. sensei depends on
  portgres, ollama.
  so we need to run checkers which return what is the result. a result in
  array with status [family,item, version, status]
  This allows us to build actions.

  homebrew not present action is attempt install, stream status. if error
  return error and script with instructions for fixing it.

  if the attempt was successful, next action can be chained. which is using
  homebrew to install/upgrade using brew file.

  i need a clean design for this, i am not sure what is the best way.

  checks can run in parallel.
    checker for sensei needs to use config (dev suffix)
  actions
    - homebrew/winget install (system) depends on os, fixer also
    - postgres, ollama, sensei [install/upgrade via homebrew/winget] [dev mode
  does not have any fixer for sensei, it just has error. run build in dev
  mode] dev mode not supported for windows.
    - postprocess [after install - start db service, if not on, start senseid
  service if not on or if upgrade was run, do stop + start] reason older
  version can mess the impl.

  I think the checker and fixers are fine, but we may need a brew bundle fixer
  option as well. current fixers are all independent brew installs

  Database checker - should check for db exists, db versin from _dbd_meta (use
  dbd to get installed version)
  fixer -> run dbd deploy with the current version tag from github(if release,
  if dev we need to know what folder to use. how does the app know what
  folder to use. the code may be in a different place if built. maybe a build
  time compiled variable for the database folder path included only in debug
  build.

  We need to have integration tests for bootstrap.

  - mocks to fail the homebrew, ollama, postgres tests and mocked checks to
  verify the command used to fix, with one mock to test what happens if
  command is successful and one mock to test what happens if install fails.
  both cases verification is polled by running checkers again so mocking the
  checker results should help us run the tests for all scenarios of checker
  and install for these 3.
  - the sensei checker /fixer can also be tested similarly
  - next task is db checker. first run can test with no db. run the fixer  (db
  +tables installed. verify using the result of dbd.

  you can use (on_complete) callback to see if db was upgraded, or remained
  same and what changes were applied.

  I think there is a better way to make this bootstrap trait call

  bootstrap::check() -> returns status of all
  bootstrap::fix() -> fixes whatever is pending. on failure returns instructions, status, callback for progress. and also a flag indicating repeat fix for next step?
  ex bootstrap.fix() fixed homebrew + install + db + seed + start (best case)
  bootstrap.fix() failed homebrew -> wait for human -> check and fix again
  bootstrap.fix() failed install/service start .wait for human -> check and fix again
  
  bootstrap.fix() failed db create .wait for human -> check and fix again

 i think this flow covers the health/boostrap. We should be able to perform integration test using this.
