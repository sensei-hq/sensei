# Homebrew cask for the Sensei desktop app (.app bundle).
# Usage:
#   brew tap mizukisu/tap
#   brew install --cask mizukisu/tap/sensei-app

cask "sensei-app" do
  GITHUB_ORG = "mizukisu".freeze
  REPO_URL = "https://github.com/#{GITHUB_ORG}/sensei".freeze

  version "0.1.0"

  if Hardware::CPU.arm?
    url "#{REPO_URL}/releases/download/v#{version}/Sensei_aarch64.dmg"
    sha256 "REPLACE_WITH_ARM64_DMG_SHA256"
  else
    url "#{REPO_URL}/releases/download/v#{version}/Sensei_x86_64.dmg"
    sha256 "REPLACE_WITH_X86_64_DMG_SHA256"
  end

  name "Sensei"
  desc "AI development intelligence desktop app"
  homepage REPO_URL

  app "Sensei.app"

  zap trash: [
    "~/.sensei",
    "~/Library/Application Support/dev.sensei.desktop",
    "~/Library/Caches/dev.sensei.desktop",
    "~/Library/Logs/dev.sensei.desktop",
    "~/Library/Preferences/dev.sensei.desktop.plist",
    "~/Library/Saved Application State/dev.sensei.desktop.savedState",
  ]
end
