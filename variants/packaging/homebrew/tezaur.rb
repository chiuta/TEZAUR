class Tezaur < Formula
  desc "Local, citation-grounded research assistant for writing a thesis"
  homepage "https://trom.tf"
  url "https://github.com/OWNER/tezaur/archive/refs/tags/v0.4.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000" # replace with the tarball sum when tagging
  license "MIT"
  head "https://github.com/OWNER/tezaur.git", branch: "main"

  depends_on "rust" => :build
  depends_on "poppler" => :recommended

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "thesis", shell_output("#{bin}/tezaur --help")
  end
end
