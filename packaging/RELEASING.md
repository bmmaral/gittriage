# Release checklist (packaging)

1. Tag `vX.Y.Z` and push; GitHub Actions **release** workflow uploads binaries + `.sha256` sidecars. macOS arm64 and x86_64 are built on one `macos-latest` runner (native + `x86_64-apple-darwin` cross-compile), so Intel macOS assets do not depend on deprecated `macos-13` hosts.
2. Update **source tarball** checksum anywhere it is pinned:
   - `packaging/homebrew/gittriage.rb` (`url` + `sha256`)
   - `packaging/aur/PKGBUILD` (`sha256sums`)
3. Set **Windows** checksums for template installers:
   - `packaging/chocolatey/tools/chocolateyinstall.ps1` → `checksum64` from `gittriage-vX.Y.Z-x86_64-pc-windows-msvc.exe.sha256`
   - `packaging/scoop/gittriage.json` → `architecture.64bit.hash` (or rely on `autoupdate` + `checkver`)
4. Bump **`packaging/npm/package.json`** `version` to match the tag (no leading `v`). Publishing **`@bmmaral/gittriage`** to GitHub Packages runs on **release published** via [`.github/workflows/npm-github-packages.yml`](../.github/workflows/npm-github-packages.yml). If **Packages** is still empty (e.g. the workflow was added after the release), open **Actions → npm-github-packages → Run workflow** once; `npm publish` uses `GITHUB_TOKEN` with `packages: write` (no extra secret). Publishing an **unscoped** `gittriage` package on the public **npmjs** registry (for `npm install -g gittriage`) is a separate maintainer step: reserve the name on npmjs if needed, add a second `package.json` or scope-flip workflow, authenticate with an npm token, and document the install path in README once live.
5. **Automated publishing (optional):** On each `v*.*.*` tag, after binaries upload, the **release** workflow can push updates if repository secrets are set:
   - **`HOMEBREW_TAP_TOKEN`**: PAT with `contents: write` on `${OWNER}/homebrew-gittriage`. Create that repository first (e.g. initialize with a README), then `brew tap ${OWNER}/gittriage https://github.com/${OWNER}/homebrew-gittriage` and `brew install gittriage`.
   - **`SCOOP_BUCKET_TOKEN`**: PAT with `contents: write` on `${OWNER}/scoop-gittriage`. Bucket layout uses `bucket/gittriage.json`. Add the bucket with `scoop bucket add gittriage https://github.com/${OWNER}/scoop-gittriage` then `scoop install gittriage`.
   - **`CHOCOLATEY_API_KEY`**: API key from [chocolatey.org](https://chocolatey.org/) for `choco push` to the community feed (reserve the **`gittriage`** package id first if needed).
   Checksums and versions are filled by `packaging/scripts/bump_release_packaging.py` from the GitHub Release `.sha256` sidecars (and the source tarball for Homebrew).
6. **crates.io:** the workspace is not a single published crate on crates.io yet — README badges point at **install-from-Git** where relevant.
7. Optionally run `nix flake lock` after dependency changes, then commit `flake.lock`.

Quick checksums from a machine with `sha256sum`:

```bash
TAG=v0.1.1
for f in gittriage-${TAG}-*; do sha256sum "$f"; done
```
