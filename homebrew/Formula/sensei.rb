# Homebrew formula for the Sensei CLI.
# Usage:
#   brew tap mizukisu/tap
#   brew install mizukisu/tap/sensei

class Sensei < Formula
  GITHUB_ORG = "mizukisu".freeze
  REPO_NAME = "sensei".freeze
  REPO_URL = "https://github.com/#{GITHUB_ORG}/#{REPO_NAME}".freeze

  desc "AI development intelligence — CLI for indexing, MCP server, and telemetry"
  homepage REPO_URL
  version "0.1.0"

  # Release archives (produced by `bun run build` + packaging)
  if OS.mac? && Hardware::CPU.arm?
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-macos-arm64.tar.gz"
    sha256 "REPLACE_WITH_ARM64_SHA256"
  elsif OS.mac? && Hardware::CPU.intel?
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-macos-x86_64.tar.gz"
    sha256 "REPLACE_WITH_X86_64_SHA256"
  elsif OS.linux? && Hardware::CPU.arm?
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-linux-arm64.tar.gz"
    sha256 "REPLACE_WITH_LINUX_ARM64_SHA256"
  else
    url "#{REPO_URL}/releases/download/v#{version}/sensei-cli-linux-x86_64.tar.gz"
    sha256 "REPLACE_WITH_LINUX_X86_64_SHA256"
  end

  bottle :unneeded

  def install
    bin.install "sensei"
    bin.install "senseid"
    bin.install "sensei-mcp"
  end

  def post_install
    (Pathname.new(ENV["HOME"]) / ".sensei").mkpath
  end

  service do
    run [opt_bin/"senseid"]
    keep_alive true
    log_path var/"log/sensei.log"
    error_log_path var/"log/sensei.error.log"
    working_dir ENV["HOME"]
    environment_variables HOME: ENV["HOME"], PATH: "#{HOMEBREW_PREFIX}/bin:/usr/local/bin:/usr/bin:/bin"
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
