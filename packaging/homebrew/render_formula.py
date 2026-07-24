#!/usr/bin/env python3
"""Render Formula/birch.rb for the Homebrew tap from a release's per-platform checksums.

Usage:
  render_formula.py --tag v0.1.0 --repo slukyano/birch --shas-dir dist > birch.rb

Reads `birch-<target>.tar.gz.sha256` files (as produced by the release workflow) from
--shas-dir and points the formula at the matching GitHub Release download URLs.
"""
import argparse
import pathlib

TARGETS = {
    "arm_mac": "aarch64-apple-darwin",
    "intel_mac": "x86_64-apple-darwin",
    "linux": "x86_64-unknown-linux-gnu",
}

TEMPLATE = '''class Birch < Formula
  desc "Lean and beautiful interactive file tree for the terminal"
  homepage "https://github.com/slukyano/birch"
  version "{version}"
  license "MIT"

  on_macos do
    on_arm do
      url "{url_arm_mac}"
      sha256 "{sha_arm_mac}"
    end
    on_intel do
      url "{url_intel_mac}"
      sha256 "{sha_intel_mac}"
    end
  end

  on_linux do
    on_intel do
      url "{url_linux}"
      sha256 "{sha_linux}"
    end
  end

  def install
    bin.install "birch", "birch-ctl", "birch-cmux", "birch-tmux", "birch-herdr"
  end

  test do
    assert_match "Usage", shell_output("#{{bin}}/birch --help")
  end
end
'''


def read_sha(shas_dir, target):
    # A .sha256 file is "<hexdigest>  birch-<target>.tar.gz".
    path = pathlib.Path(shas_dir) / f"birch-{target}.tar.gz.sha256"
    return path.read_text().split()[0]


def url(repo, tag, target):
    return f"https://github.com/{repo}/releases/download/{tag}/birch-{target}.tar.gz"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", required=True)
    ap.add_argument("--repo", required=True)
    ap.add_argument("--shas-dir", required=True)
    a = ap.parse_args()

    fields = {"version": a.tag.lstrip("v")}
    for key, target in TARGETS.items():
        fields[f"url_{key}"] = url(a.repo, a.tag, target)
        fields[f"sha_{key}"] = read_sha(a.shas_dir, target)

    print(TEMPLATE.format(**fields), end="")


if __name__ == "__main__":
    main()
