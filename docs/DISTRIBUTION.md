# Distribution

GitTriage ships **prebuilt binaries** on [GitHub Releases](https://github.com/bmmaral/gittriage/releases) (Linux musl x86_64, macOS arm64/x86_64, Windows x86_64) plus `.sha256` sidecars. You can also **build from source** with Rust.

## Which channel should I use?

```text
Need GitTriage on PATH quickly?
├─ Prefer a prebuilt binary → GitHub Releases (download + verify `.sha256`)
├─ Use a package manager you already trust
│  ├─ macOS / Linux with Homebrew → in-repo formula (`packaging/homebrew/`) or your tap
│  ├─ Arch → AUR PKGBUILD reference (`packaging/aur/`)
│  ├─ Nix → `nix run .#gittriage` / `nix build` from this flake
│  ├─ Windows Scoop / Chocolatey → manifests under `packaging/scoop/` and `packaging/chocolatey/`
│  └─ Node/npm user → `@bmmaral/gittriage` on GitHub Packages (`packaging/npm/`); unscoped `gittriage` on public npmjs remains a future maintainer action if the name is reserved
└─ You ship Rust toolchains → `cargo install --path crates/gittriage` or `--git … --tag vX.Y.Z`
```

After each release, maintainers can sanity-check asset filenames against a tag with [`packaging/verify-release-assets.sh`](../packaging/verify-release-assets.sh) (optional; requires `curl`/`grep`).

## Cargo (from crates.io)

When published:

```bash
cargo install gittriage
```

Until then, from a git checkout:

```bash
cargo install --path crates/gittriage
# or
cargo install --locked --git https://github.com/bmmaral/gittriage --tag v0.1.1 --package gittriage
```

## Homebrew (formula in repo)

Builds from the release **source tarball** (Rust required):

```bash
brew install --formula ./packaging/homebrew/gittriage.rb
```

After each upstream tag, update `url`, `sha256`, and `version` in `packaging/homebrew/gittriage.rb` (tarball checksum: `curl -sL …/vX.Y.Z.tar.gz | shasum -a 256`).

To publish a **tap**, mirror `gittriage.rb` into a `homebrew-*` repository’s `Formula/` directory and document `brew tap owner/repo`.

## Scoop (Windows)

Manifest: [`packaging/scoop/gittriage.json`](../packaging/scoop/gittriage.json).

1. Copy the manifest into a Scoop bucket (or install from a raw URL).
2. Set `"hash"` for the Windows `.exe` to the value from the matching `.sha256` file on the release, **or** use `checkver` / `autoupdate` to refresh from `$url.sha256`.

`pre_install` renames the downloaded executable to `gittriage.exe`.

## Chocolatey (Windows)

Packaging lives under [`packaging/chocolatey/`](../packaging/chocolatey/). Set `checksum64` in `tools/chocolateyinstall.ps1` from the release `.sha256` file for that version, then:

```powershell
choco pack packaging/chocolatey/gittriage.nuspec
choco install gittriage -s . -y
```

For a one-off install before the checksum is wired, Chocolatey supports `--ignore-checksums` (not recommended for automation).

## npm / npx / bunx (thin binary wrapper)

Package name on **GitHub Packages**: **`@bmmaral/gittriage`** (`packaging/npm/`). The wrapper downloads the GitHub Release binary for the current platform into `~/.cache/gittriage/<version>/`.

### Install from GitHub Packages

Create or extend `~/.npmrc` (use a classic PAT with `read:packages` if the repo is private):

```
@bmmaral:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=YOUR_GITHUB_TOKEN
```

Then:

```bash
npm install -g @bmmaral/gittriage
gittriage --version
```

Releases also run [`.github/workflows/npm-github-packages.yml`](../.github/workflows/npm-github-packages.yml) to publish the package when a GitHub Release is published (keep `package.json` `version` aligned with the tag, without a leading `v`).

### Local tarball (for testing)

```bash
cd packaging/npm && npm pack && npm install -g ./bmmaral-gittriage-*.tgz
gittriage --version
```

Keep `package.json` `version` in sync with a GitHub tag (`v0.1.1` → assets `gittriage-v0.1.1-…`).

```bash
bunx --bun ./packaging/npm/bin/gittriage.js -- --version   # after local pack/install
```

## AUR (Arch Linux)

Reference [`packaging/aur/PKGBUILD`](../packaging/aur/PKGBUILD): copy into an AUR package, add a `Maintainer:` line, and submit. It builds the `gittriage` crate from the tagged source tarball.

## Nix

A [`flake.nix`](../flake.nix) at the repo root builds `gittriage` from this workspace.

```bash
nix run .#gittriage -- --version
nix build .#gittriage
```

First time in a fresh clone, generate a lockfile:

```bash
nix flake lock
```

## Support policy

| Channel    | Status |
| ---------- | ------ |
| GitHub Releases | Primary |
| Cargo / source | Primary |
| Homebrew formula (in-repo) | Supported template |
| Nix flake | Supported template |
| AUR PKGBUILD | Upstream reference (maintainer submits to AUR) |
| npm wrapper (GitHub Packages) | Supported template |
| Scoop / Chocolatey | Supported templates (checksums per release) |

Windows binaries exist **from the first release that includes the `windows` job** in `.github/workflows/release.yml`; update Scoop/Chocolatey hashes from the published `.sha256` files.
