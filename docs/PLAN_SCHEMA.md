# Plan schema

```json
{
  "generated_at": "2026-03-24T12:00:00Z",
  "generated_by": "nexus 0.1.0",
  "clusters": [
    {
      "cluster_id": "cluster-001",
      "label": "nexus",
      "confidence": 0.91,
      "canonical_clone_id": "clone-123",
      "canonical_remote_id": "remote-456",
      "scores": {
        "canonical": 91,
        "usability": 67,
        "oss_readiness": 43,
        "risk": 28
      },
      "actions": [
        {
          "priority": "high",
          "action_type": "archive_local_duplicate",
          "target_kind": "clone",
          "target_id": "clone-789",
          "reason": "older duplicate with lower canonical score"
        }
      ],
      "evidence": [
        {
          "kind": "remote_url_match",
          "delta": 25,
          "detail": "matched origin to demo/example"
        }
      ]
    }
  ]
}
```
