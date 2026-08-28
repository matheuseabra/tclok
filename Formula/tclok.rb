class Tclok < Formula
  desc "Dependency-free, resize-responsive terminal clock"
  homepage "https://github.com/matheuseabra/tclok"
  url "https://crates.io/api/v1/crates/tclok/0.1.1/download"
  sha256 "3f61824697af5ebaedca8c66a7f1521a3e28e9c5786379f1c5878aa680e88173"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tclok --version")
  end
end
