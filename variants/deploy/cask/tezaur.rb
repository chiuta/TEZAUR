cask "tezaur" do
  version "0.1.0"
  sha256 "REPLACE_WITH_SHA256_OF_THE_DMG"
  url "https://github.com/chiuta/tezaur/releases/download/v#{version}/TEZAUR-#{version}.dmg"
  name "TEZAUR"
  desc "Local, citation-grounded research assistant"
  homepage "https://github.com/chiuta/tezaur"
  app "TEZAUR.app"
  zap trash: "~/Library/Application Support/TEZAUR"
end
