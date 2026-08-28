class Tclok < Formula
  desc "Dependency-free, resize-responsive terminal clock"
  homepage "https://github.com/matheuseabra/tclok"
  url "https://crates.io/api/v1/crates/tclok/0.1.0/download"
  sha256 "12bed5981b794aaf88db278dda6cb44244e98f0fa1a4885ed777d31ecadd2634"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(lock: true), "--path", "."
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tclok --version")
  end
end
