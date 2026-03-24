#!/usr/bin/env bash
# Tag the current commit as the last pre–v2-Rust snapshot (run manually once).
set -euo pipefail
TAG="${1:-legacy-py-mvp}"
git tag -a "$TAG" -m "Legacy Python/TS project-memory MVP before Nexus v2 (Rust)"
echo "Created tag $TAG (push with: git push origin $TAG)"
