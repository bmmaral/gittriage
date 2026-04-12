#!/usr/bin/env bash
# Optional post-release check: list GitHub Release assets for a tag and verify expected prefixes.
# Usage: TAG=v0.1.1 ./packaging/verify-release-assets.sh
set -euo pipefail

TAG="${TAG:?set TAG=vX.Y.Z (example: TAG=v0.1.1)}"
REPO="${GITHUB_REPOSITORY:-bmmaral/gittriage}"
API="https://api.github.com/repos/${REPO}/releases/tags/${TAG}"

echo "Fetching release assets for ${TAG} (${REPO})…"
json="$(curl -fsSL "$API")"

if echo "$json" | grep -q '"message": "Not Found"'; then
  echo "Release or tag not found via GitHub API. Check TAG and repo." >&2
  exit 1
fi

echo "$json" | grep -o '"name": *"[^"]*"' | sed 's/"name": *"//;s/"$//' || true

# Expect at least one platform binary and a source tarball pattern (names vary by workflow).
if ! echo "$json" | grep -q 'gittriage-'; then
  echo "Warning: no asset name containing 'gittriage-' found." >&2
  exit 1
fi

echo "OK: release has gittriage-* assets."
