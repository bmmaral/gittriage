#!/usr/bin/env python3
"""Build a CI regression dashboard artifact from GitTriage JSON outputs."""
from __future__ import annotations

import argparse
import json
import os
from collections import Counter
from pathlib import Path
from statistics import fmean
from typing import Any


def _load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def _count(items: list[dict[str, Any]], key: str) -> dict[str, int]:
    return dict(Counter(item.get(key, "unknown") for item in items))


def _score_averages(clusters: list[dict[str, Any]]) -> dict[str, float]:
    axes = ("canonical", "usability", "recoverability", "oss_readiness", "risk")
    if not clusters:
        return {axis: 0.0 for axis in axes}
    return {
        axis: round(fmean(float(cluster.get("scores", {}).get(axis, 0.0)) for cluster in clusters), 2)
        for axis in axes
    }


def build_dashboard(plan: dict[str, Any], summary: dict[str, Any]) -> dict[str, Any]:
    clusters = list(plan.get("clusters", []))
    plan_clusters = [cp.get("cluster", {}) for cp in clusters]
    actions = [action for cp in clusters for action in cp.get("actions", [])]
    evidence = [ev for cp in clusters for ev in cp.get("cluster", {}).get("evidence", [])]

    status_counts = _count(plan_clusters, "status")
    action_priority_counts = _count(actions, "priority")
    evidence_kind_counts = _count(evidence, "kind")

    cluster_rows = []
    for cp in clusters:
        cluster = cp.get("cluster", {})
        row_actions = cp.get("actions", [])
        cluster_rows.append(
            {
                "id": cluster.get("id"),
                "cluster_key": cluster.get("cluster_key"),
                "label": cluster.get("label"),
                "status": cluster.get("status"),
                "canonical_clone_id": cluster.get("canonical_clone_id"),
                "canonical_remote_id": cluster.get("canonical_remote_id"),
                "confidence": cluster.get("confidence"),
                "scores": cluster.get("scores", {}),
                "evidence_count": len(cluster.get("evidence", [])),
                "action_count": len(row_actions),
                "action_priority_counts": _count(row_actions, "priority"),
            }
        )

    return {
        "schema_version": 1,
        "kind": "gittriage_regression_dashboard",
        "generated_at": plan.get("generated_at"),
        "generated_by": plan.get("generated_by", "gittriage-ci"),
        "ci": {
            "workflow": os.environ.get("GITHUB_WORKFLOW"),
            "repository": os.environ.get("GITHUB_REPOSITORY"),
            "ref": os.environ.get("GITHUB_REF_NAME"),
            "sha": os.environ.get("GITHUB_SHA"),
            "run_id": os.environ.get("GITHUB_RUN_ID"),
        },
        "overview": {
            "cluster_count": len(clusters),
            "resolved_count": status_counts.get("Resolved", 0),
            "ambiguous_count": status_counts.get("Ambiguous", 0),
            "manual_review_count": status_counts.get("ManualReview", 0),
            "action_count": len(actions),
            "action_priority_counts": action_priority_counts,
            "evidence_count": len(evidence),
            "evidence_kind_counts": evidence_kind_counts,
            "unsafe_target_count": len(summary.get("unsafe_targets", [])),
            "duplicate_group_count": len(summary.get("duplicate_groups", [])),
            "canonical_path_count": len(summary.get("canonical_paths", [])),
            "nested_repo_warning_count": len(summary.get("nested_repo_warnings", [])),
            "score_averages": _score_averages(plan_clusters),
        },
        "clusters": cluster_rows,
        "canonical_paths": summary.get("canonical_paths", []),
        "duplicate_groups": summary.get("duplicate_groups", []),
        "unsafe_targets": summary.get("unsafe_targets", []),
        "dirty_canonical_repos": summary.get("dirty_canonical_repos", []),
        "nested_repo_warnings": summary.get("nested_repo_warnings", []),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("plan_json", type=Path)
    parser.add_argument("summary_json", type=Path)
    parser.add_argument("output_json", type=Path)
    args = parser.parse_args()

    plan = _load_json(args.plan_json)
    summary = _load_json(args.summary_json)
    dashboard = build_dashboard(plan, summary)
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    with args.output_json.open("w", encoding="utf-8") as f:
        json.dump(dashboard, f, indent=2, sort_keys=True)
        f.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
