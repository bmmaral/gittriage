# Homebrew formula: build GitTriage from source (no per-arch binary checksums to update).
# Install:
#   brew install ./packaging/homebrew/gittriage.rb
# Or add this tap-style clone and:
#   brew install --formula packaging/homebrew/gittriage.rb
#
# After tagging a new release, bump `version` and `sha256` (source tarball):
#   curl -sL "https://github.com/bmmaral/gittriage/archive/refs/tags/vX.Y.Z.tar.gz" | shasum -a 256

class GitTriage < Formula
  desc "Local-first repo fleet triage CLI"
  homepage "https://github.com/bmmaral/gittriage"
  url "https://github.com/bmmaral/gittriage/archive/refs/tags/v0.1.1.tar.gz"
  sha256 "dee91c2c4482c831350bbd15a3144f20ed96ebf0a00e0f7a435974a7a9556ca1"
  license "MIT"
  head "https://github.com/bmmaral/gittriage.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(root: libexec, path: "crates/gittriage")
    bin.install_symlink libexec/"bin"/"gittriage"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/gittriage --version")
  end
end
