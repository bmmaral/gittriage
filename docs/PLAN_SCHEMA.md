# Plan JSON (`plan.json`)

Machine-readable plans use **`schema_version`** `1` (integer). Older files without this field deserialize as version `1`.

Top-level object:

| Field | Type | Description |
| --- | --- | --- |
| `schema_version` | number | Format version; always `1` for this shape |
| `scoring_rules_version` | number | Optional; defaults to `1` if omitted. Version of deterministic scoring rules (`nexus-plan`); see `docs/SCORING.md` |
| `generated_at` | string | RFC 3339 UTC timestamp |
| `generated_by` | string | Producer string, e.g. `nexus 0.1.0` |
| `clusters` | array | One entry per `ClusterPlan` |

Each element of `clusters` is:

```json
{
  "cluster": { ... ClusterRecord ... },
  "actions": [ ... PlanAction ... ]
}
```

### `ClusterRecord`

| Field | Type | Notes |
| --- | --- | --- |
| `id` | string | Stable cluster id |
| `cluster_key` | string | Dedup key (e.g. normalized URL bucket) |
| `label` | string | Human label |
| `status` | string | `Resolved`, `Ambiguous`, or `ManualReview` |
| `confidence` | number | 0.0–1.0 style confidence |
| `canonical_clone_id` | string or null | |
| `canonical_remote_id` | string or null | |
| `members` | array | `{ "kind": "Clone" \| "Remote", "id": "..." }` |
| `evidence` | array | See below |
| `scores` | object | `canonical`, `usability`, `recoverability` (optional, defaults `0`), `oss_readiness`, `risk` (numbers 0–100) |

### `EvidenceItem`

| Field | Type |
| --- | --- |
| `id` | string |
| `subject_kind` | `Clone` or `Remote` |
| `subject_id` | string |
| `kind` | string (e.g. `remote_url_match`, adapter-specific kinds) |
| `score_delta` | number |
| `detail` | string |

### `PlanAction`

| Field | Type |
| --- | --- |
| `id` | string |
| `priority` | `Low`, `Medium`, `High` |
| `action_type` | Enum serialized as string, e.g. `MarkCanonical`, `ArchiveLocalDuplicate`, … |
| `target_kind` | `Clone` or `Remote` |
| `target_id` | string |
| `reason` | string |
| `commands` | array of strings (often empty in read-only mode) |
| `evidence_summary` | optional string — short pointer to motivating evidence |
| `confidence` | optional number — planner confidence for this action, 0.0–1.0 |
| `risk_note` | optional string — trade-off or caution |

Omitted optional fields deserialize as null/absent and remain backward compatible for older `plan.json` files.

### Example (excerpt)

See `fixtures/golden/plan-v1.json` for a full round-trippable example.

## Stability

For **v0.x**, command names and flags are intentionally conservative; `plan.json` may gain optional fields in backward-compatible ways. Breaking changes will bump `schema_version` and be noted in `CHANGELOG.md`.
