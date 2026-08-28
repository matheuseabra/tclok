class Tclok < Formula
  desc "Dependency-free, resize-responsive terminal clock"
  homepage "https://github.com/matheuseabra/tclok"
  url "https://github.com/matheuseabra/tclok/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "ce6b8b22332e3bfbb49a1037d33330fd0613e34683ba79b1f6fb7d1383a4cb6f"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tclok --version")
  end
end
