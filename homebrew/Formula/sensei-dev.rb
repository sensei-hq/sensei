# Homebrew formula for the Sensei dev binaries.
# Builds the senseid / sensei / sensei-mcp crates from source on the
# `develop` branch with --features dev, and installs them under -dev
# suffixed names alongside the prod `sensei` formula's binaries.
#
# Build invocation lives in the monorepo's `make crates-dev` target so
# both the formula and `make install-dev` go through the same cargo
# command — one source of truth for build flags.
#
# Usage:
#   brew tap sensei-hq/tap
#   brew install --HEAD sensei-hq/tap/sensei-dev
# Or via Brewfile-dev:
#   brew bundle --file=https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile-dev

class SenseiDev < Formula
  desc "Sensei dev binaries — built from source with --features dev"
  homepage "https://github.com/sensei-hq/sensei"
  version "0.2.14-dev"

  # No stable URL — dev formulae always build from HEAD of develop.
  # SSH (not HTTPS) so the private sensei-hq/sensei repo can be cloned
  # via the developer's existing GitHub SSH key — HTTPS would prompt for
  # username/password and hang brew bundle.
  # `using: :git` forces the git download strategy — brew only auto-detects
  # git from `.git`-suffixed HTTPS URLs, not from SSH-style URLs.
  head "git@github.com:sensei-hq/sensei.git", branch: "develop", using: :git

  depends_on "rust" => :build
  depends_on "make" => :build
  depends_on "postgresql@17"

  def install
    # Reuse the monorepo's `make crates-dev` target — single cargo invocation
    # for `senseid`, `sensei-cli`, and `sensei-mcp` with --features dev.
    system "make", "crates-dev"

    # Install the three binaries with -dev suffixes into the brew prefix.
    bin.install "target/debug/senseid"    => "senseid-dev"
    bin.install "target/debug/sensei"     => "sensei-dev"
    bin.install "target/debug/sensei-mcp" => "sensei-mcp-dev"

    # Adhoc codesign on macOS so the Tauri sidecar can spawn them.
    # (macOS Sequoia Code Signing Monitor at level 2 requires this.)
    if OS.mac?
      %w[senseid-dev sensei-dev sensei-mcp-dev].each do |name|
        system "codesign", "--sign", "-", "--options", "runtime", "--force",
               bin/name
      end
    end
  end

  def post_install
    (Pathname.new(Dir.home) / ".sensei-dev").mkpath
  end

  def caveats
    <<~EOS
      Installed sensei dev binaries (port 7745, db sensei_dev, dir ~/.sensei-dev/).

      Start the dev daemon:

        senseid-dev start

      The dev binaries coexist with the prod `sensei` formula — both can be
      installed on the same machine and registered with Claude Code under
      distinct MCP keys (`sensei` and `sensei-dev`).
    EOS
  end

  test do
    system "#{bin}/sensei-dev", "--version"
  end
end
