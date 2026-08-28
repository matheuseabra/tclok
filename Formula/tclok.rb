class Tclok < Formula
  desc "Dependency-free, resize-responsive terminal clock"
  homepage "https://github.com/matheuseabra/tclok"
  url "https://crates.io/api/v1/crates/tclok/0.1.2/download"
  sha256 "ebca7d613c3429b815d3d22931cb638c610428a5370174611cc49320ffad0659"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tclok --version")
  end
end
