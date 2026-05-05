# Packaging Plan

`rclean` uses separate package and binary names:

- Cargo package: `rclean-cli`
- Installed command: `rclean`

This is intentional because the `rclean` crates.io package name is already
occupied. Users should still type `rclean` after installing.

## Cargo

Expected install path:

```bash
cargo install rclean-cli
rclean --help
```

Before publishing:

```bash
cargo package --list
cargo publish --dry-run
```

## GitHub Releases

Release artifacts should be named by target:

```text
rclean-aarch64-apple-darwin.tar.gz
rclean-x86_64-apple-darwin.tar.gz
rclean-x86_64-unknown-linux-gnu.tar.gz
rclean-x86_64-pc-windows-msvc.zip
checksums.txt
```

Each archive should contain:

- `rclean`
- `README.md`
- `LICENSE` once a license is added

## Homebrew

Formula shape:

```ruby
class Rclean < Formula
  desc "Find and clean rebuildable developer artifacts"
  homepage "https://github.com/<owner>/rclean"
  url "https://github.com/<owner>/rclean/releases/download/vX.Y.Z/rclean-aarch64-apple-darwin.tar.gz"
  sha256 "<sha>"
  license "MIT"

  def install
    bin.install "rclean"
  end

  test do
    system "#{bin}/rclean", "--help"
  end
end
```

## README Release Requirements

README must show:

- one-command scan
- dry-run clean
- ActionPlan workflow
- safety model
- supported ecosystems
- install options
