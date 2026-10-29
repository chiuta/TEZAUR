class Tezaur < Formula
  desc "Local, citation-grounded research assistant for writing a thesis"
  homepage "https://trom.tf"
  url "https://github.com/chiuta/tezaur/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "SHA256-OF-SOURCE-TARBALL"
  license "MIT"
  head "https://github.com/chiuta/tezaur.git", branch: "main"

  depends_on "rust" => :build
  depends_on "poppler" => :recommended

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "thesis", shell_output("#{bin}/tezaur --help")
  end
end
