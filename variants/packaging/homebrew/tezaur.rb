class Tezaur < Formula
  desc "Local, citation-grounded research assistant for writing a thesis"
  homepage "https://trom.tf"
  url "https://github.com/chiuta/TEZAUR/archive/refs/tags/v0.5.3.tar.gz"
  sha256 "580019b3a2d2fb252319401aab64c03e747503328c025e03d37bb31d353adcaf"
  license "MIT"
  head "https://github.com/chiuta/TEZAUR.git", branch: "main"

  depends_on "rust" => :build
  depends_on "poppler" => :recommended

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "thesis", shell_output("#{bin}/tezaur --help")
  end
end
