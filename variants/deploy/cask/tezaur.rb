cask "tezaur" do
  version "0.4.0"
  sha256 "REPLACE_WITH_SHA256_OF_THE_DMG"

  url "https://github.com/chiuta/TEZAUR/releases/download/v#{version}/TEZAUR-#{version}.dmg"
  name "TEZAUR"
  desc "Local, citation-grounded research assistant"
  homepage "https://github.com/chiuta/TEZAUR"

  depends_on :macos

  app "TEZAUR.app"

  zap trash: "~/Library/Application Support/TEZAUR"
end
