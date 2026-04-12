#!/usr/bin/env python3
"""Render packaging files for a tagged release (checksums from GitHub release assets)."""
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import urllib.request


def _read_url(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "gittriage-release-packaging"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def _sha256_file(url: str) -> str:
    return _read_url(url).decode().strip().split()[0]


def _sha256_tarball(url: str) -> str:
    h = hashlib.sha256()
    h.update(_read_url(url))
    return h.hexdigest()


def homebrew_formula(repo: str, tag: str, template_path: str) -> str:
    ver = tag.removeprefix("v")
    tarball = f"https://github.com/{repo}/archive/refs/tags/{tag}.tar.gz"
    sha = _sha256_tarball(tarball)
    text = open(template_path, encoding="utf-8").read()
    text = re.sub(
        r'^\s*url\s+"https://github\.com/[^"]+"\s*$',
        f'  url "{tarball}"',
        text,
        count=1,
        flags=re.M,
    )
    text = re.sub(
        r'^\s*sha256\s+"[a-f0-9]{64}"\s*$',
        f'  sha256 "{sha}"',
        text,
        count=1,
        flags=re.M,
    )
    return text


def scoop_manifest(repo: str, tag: str, path: str) -> None:
    ver = tag.removeprefix("v")
    base = f"https://github.com/{repo}/releases/download/{tag}"
    win_url = f"{base}/gittriage-{tag}-x86_64-pc-windows-msvc.exe"
    win_hash = _sha256_file(f"{win_url}.sha256")
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    data["version"] = ver
    data["architecture"]["64bit"]["url"] = win_url
    data["architecture"]["64bit"]["hash"] = win_hash
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
        f.write("\n")


def chocolatey_files(repo: str, tag: str, root: str) -> None:
    ver = tag.removeprefix("v")
    base = f"https://github.com/{repo}/releases/download/{tag}"
    win_url = f"{base}/gittriage-{tag}-x86_64-pc-windows-msvc.exe"
    win_hash = _sha256_file(f"{win_url}.sha256")
    nuspec = os.path.join(root, "chocolatey", "gittriage.nuspec")
    text = open(nuspec, encoding="utf-8").read()
    text = re.sub(r"<version>[^<]+</version>", f"<version>{ver}</version>", text, count=1)
    open(nuspec, "w", encoding="utf-8").write(text)
    ps1 = os.path.join(root, "chocolatey", "tools", "chocolateyinstall.ps1")
    t = open(ps1, encoding="utf-8").read()
    t = re.sub(r"\$checksum64 = '[^']+'", f"$checksum64 = '{win_hash}'", t, count=1)
    open(ps1, "w", encoding="utf-8").write(t)


def main() -> None:
    if len(sys.argv) < 2:
        print("usage: bump_release_packaging.py homebrew|scoop|chocolatey", file=sys.stderr)
        sys.exit(2)
    cmd = sys.argv[1]
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    tag = os.environ.get("GITHUB_REF_NAME", "")
    if not repo or not tag:
        print("GITHUB_REPOSITORY and GITHUB_REF_NAME are required", file=sys.stderr)
        sys.exit(1)
    root = os.path.join(os.path.dirname(__file__), "..")
    root = os.path.normpath(root)
    if cmd == "homebrew":
        tpl = os.path.join(root, "homebrew", "gittriage.rb")
        sys.stdout.write(homebrew_formula(repo, tag, tpl))
    elif cmd == "scoop":
        manifest = os.path.join(root, "scoop", "gittriage.json")
        scoop_manifest(repo, tag, manifest)
    elif cmd == "chocolatey":
        chocolatey_files(repo, tag, root)
    else:
        print(f"unknown command: {cmd}", file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
