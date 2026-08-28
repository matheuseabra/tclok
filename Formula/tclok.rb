class Tclok < Formula
  desc "Dependency-free, resize-responsive terminal clock"
  homepage "https://github.com/matheuseabra/tclok"
  url "https://crates.io/api/v1/crates/tclok/0.1.0/download"
  sha256 "25d3d579aa1bec8c287610a38815934aab8ea4c8acbac8957b34e6c6ba4e09eb"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(lock: true), "--path", "."
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tclok --version")
  end
end
