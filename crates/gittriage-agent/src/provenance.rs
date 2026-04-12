//! Provenance fields for agent-facing outputs (F7).

use chrono::{DateTime, Utc};
use gittriage_core::InventorySnapshot;
use serde::Serialize;

/// How fresh the structured output is relative to SQLite persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// Built from current inventory in memory (same as `plan` / `score` recompute).
    ComputedFromInventory,
}

#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    pub generated_at: DateTime<Utc>,
    pub inventory_run_id: Option<String>,
    /// Scan roots from the latest run, when present.
    pub scope: Vec<String>,
    pub freshness: Freshness,
    pub data_sources: Vec<&'static str>,
}

impl Provenance {
    pub fn from_snapshot(snapshot: &InventorySnapshot) -> Self {
        let run = snapshot.run.as_ref();
        Self {
            generated_at: Utc::now(),
            inventory_run_id: run.map(|r| r.id.clone()),
            scope: run.map(|r| r.roots.clone()).unwrap_or_default(),
            freshness: Freshness::ComputedFromInventory,
            data_sources: vec!["sqlite_inventory", "gittriage_plan_engine"],
        }
    }
}
