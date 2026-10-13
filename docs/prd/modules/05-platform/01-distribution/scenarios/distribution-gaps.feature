Feature: Distribution — the unbuilt Linux/Windows service layer and unified install

  # gap: P-5
  NO TEST
  Scenario: the Linux service layer uses systemd --user
    Given a Linux machine
    When sensei is installed
    Then a systemd user unit is created, enabled, and started
    This is unbuilt — design first (reorg §10.3), then implement.

  # gap: P-5
  NO TEST
  Scenario: the Linux data directory follows XDG
    Given a Linux machine
    When sensei runs
    Then the data dir is ~/.local/share/sensei (XDG_DATA_HOME)
    This is unbuilt.

  # gap: P-5
  NO TEST
  Scenario: the Linux shell integration adds sensei to PATH and completions
    Given a Linux machine with bash/zsh/fish
    When sensei is installed
    Then the shell rc files are updated with PATH and completions
    This is unbuilt.

  # gap: P-5
  NO TEST
  Scenario: the Windows service runs as a native service or scheduled task
    Given a Windows machine
    When sensei is installed
    Then a Windows service is created and started
    This is unbuilt.

  # gap: P-5
  NO TEST
  Scenario: the Windows data directory follows LOCALAPPDATA
    Given a Windows machine
    When sensei runs
    Then the data dir is %LOCALAPPDATA%\sensei
    This is unbuilt.

  # gap: P-5
  NO TEST
  Scenario: the Windows shell integration adds sensei to PATH in PowerShell and cmd
    Given a Windows machine
    When sensei is installed
    Then the user PATH is updated and completions are registered
    This is unbuilt.

  # gap: P-5
  NO TEST
  Scenario: the unified install script detects platform and runs the right path
    Given a machine (macOS, Linux, or Windows)
    When curl ... | sh runs
    Then it detects the platform and runs the correct install path
    This is unbuilt — macOS path exists via Homebrew; Linux/Windows paths unbuilt.

  # gap: P-5
  NO TEST
  Scenario: the tarball matrix publishes checksums for all three targets
    Given a release
    When artifacts are published
    Then sensei-<version>-<target>.tar.gz + .sha256 exist for all targets
    This is unbuilt — macOS formula exists; Linux/Windows tarballs unbuilt.

  # gap: P-5
  NO TEST
  Scenario: the Windows artifact is Authenticode signed
    Given a Windows release artifact
    When it is inspected
    Then it has a valid Authenticode signature
    This is unbuilt.

  # gap: D-13
  NO TEST
  Scenario: transitive OSV advisories are monitored and acceptable
    Given the dependency tree
    When OSV is scanned
    Then the 28 medium advisories are tracked and accepted as non-code-fixable
    This is a known state (P3), not a build gap.

  # gap: D-12
  NO TEST
  Scenario: @rokkit/* and @kavach/* are not held back by lockfiles
    Given the package.json lockfiles
    When dependencies are updated
    Then rokkit and kavach packages are current
    This is a lockfile maintenance gap (P3).