# Homebrew cask for the Sensei desktop app (.app bundle).
# Depends on the sensei formula for CLI tools (senseid, sensei-mcp).
#
# Usage:
#   brew tap sensei-hq/tap
#   brew install --cask sensei-hq/tap/senseihq

cask "senseihq" do
  version "0.10.1"

  # Universal binary — runs natively on both Apple Silicon and Intel
  url "https://github.com/sensei-hq/sensei/releases/download/v#{version}/Sensei_#{version}_universal.dmg"
  sha256 "fd4c1ead11781cad7a83689c84844ce0b9ff798005410ab7179d485f3f4028d8"

  name "Sensei"
  desc "AI development intelligence desktop app"
  homepage "https://github.com/sensei-hq/sensei"

  depends_on formula: "sensei"

  app "Sensei.app"

  zap trash: [
    "~/.sensei",
    "~/Library/Application Support/com.senseihq.desktop",
    "~/Library/Caches/com.senseihq.desktop",
    "~/Library/Logs/com.senseihq.desktop",
    "~/Library/Preferences/com.senseihq.desktop.plist",
    "~/Library/Saved Application State/com.senseihq.desktop.savedState",
  ]
end
