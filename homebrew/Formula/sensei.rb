# Homebrew formula for the Sensei CLI tools.
# Installs: senseid (daemon), sensei (CLI), sensei-mcp (MCP server)
#
# Usage:
#   brew tap sensei-hq/tap
#   brew install sensei-hq/tap/sensei

class Sensei < Formula
  desc "AI development intelligence — CLI, daemon, and MCP server"
  homepage "https://github.com/sensei-hq/sensei"
  version "0.1.0"

  # Release archives built by GitHub Actions (release-daemon.yml).
  # Each tarball contains a single directory named after the artifact
  # (e.g. sensei-macos-arm64/) holding senseid, sensei, and sensei-mcp.
  if OS.mac? && Hardware::CPU.arm?
    url "https://github.com/sensei-hq/sensei/releases/download/v#{version}/sensei-macos-arm64.tar.gz"
    sha256 "REPLACE_WITH_ARM64_SHA256"
  elsif OS.mac? && Hardware::CPU.intel?
    url "https://github.com/sensei-hq/sensei/releases/download/v#{version}/sensei-macos-x86_64.tar.gz"
    sha256 "REPLACE_WITH_X86_64_SHA256"
  elsif OS.linux? && Hardware::CPU.arm?
    url "https://github.com/sensei-hq/sensei/releases/download/v#{version}/sensei-linux-arm64.tar.gz"
    sha256 "REPLACE_WITH_LINUX_ARM64_SHA256"
  else
    url "https://github.com/sensei-hq/sensei/releases/download/v#{version}/sensei-linux-x86_64.tar.gz"
    sha256 "REPLACE_WITH_LINUX_X86_64_SHA256"
  end

  def install
    # Binaries live inside the platform-specific subdirectory in the tarball
    arch_dir = Dir["*/"].first || ""
    bin.install "#{arch_dir}sensei"
    bin.install "#{arch_dir}senseid"
    bin.install "#{arch_dir}sensei-mcp"
  end

  def post_install
    (Pathname.new(Dir.home) / ".sensei").mkpath
  end

  service do
    run [opt_bin/"senseid"]
    keep_alive true
    log_path var/"log/sensei.log"
    error_log_path var/"log/sensei.error.log"
    working_dir Dir.home
    environment_variables HOME: Dir.home, PATH: "#{HOMEBREW_PREFIX}/bin:/usr/local/bin:/usr/bin:/bin"
  end

  def caveats
    <<~EOS
      After installing, configure sensei for your AI coding platform:

        sensei install --acp claude-code   # or cursor, windsurf
        sensei start                       # start the daemon
        sensei scan ~/Developer            # scan and index your repos

      Before uninstalling:

        sensei stop
        sensei uninstall

      Note: Homebrew formulas have no uninstall hook, so this step is manual.
    EOS
  end

  test do
    system "#{bin}/sensei", "--version"
  end
end
