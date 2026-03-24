use anyhow::{bail, Result};
use clap::ValueEnum;
use nexus_core::{ClusterPlan, InventorySnapshot, MemberKind, PlanDocument};

#[derive(Debug, Clone, ValueEnum)]
pub enum ExplainFormat {
    Text,
    Json,
}

pub fn run_explain(
    snapshot: &InventorySnapshot,
    plan: &PlanDocument,
    target: ExplainTarget,
    format: ExplainFormat,
) -> Result<()> {
    let cp = match target {
        ExplainTarget::Cluster { query } => resolve_cluster_plan(plan, &query)?,
        ExplainTarget::Clone { id } => cluster_for_member(plan, MemberKind::Clone, &id)?,
        ExplainTarget::Remote { id } => cluster_for_member(plan, MemberKind::Remote, &id)?,
    };

    match format {
        ExplainFormat::Text => println!("{}", render_explain_text(cp, snapshot)),
        ExplainFormat::Json => {
            let v = serde_json::json!({
                "schema_version": 1u32,
                "kind": "nexus_explain",
                "cluster": cp.cluster,
                "actions": cp.actions,
            });
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}

#[derive(Debug, clap::Subcommand)]
pub enum ExplainTarget {
    /// Cluster id, exact label (case-insensitive), or unique substring of id/label.
    Cluster {
        #[arg(value_name = "ID_OR_LABEL")]
        query: String,
    },
    /// Clone member id (from inventory / plan).
    Clone {
        #[arg(value_name = "CLONE_ID")]
        id: String,
    },
    /// Remote member id (from inventory / plan).
    Remote {
        #[arg(value_name = "REMOTE_ID")]
        id: String,
    },
}

fn resolve_cluster_plan<'a>(plan: &'a PlanDocument, query: &str) -> Result<&'a ClusterPlan> {
    let q = query.trim();
    if q.is_empty() {
        bail!("cluster query is empty");
    }

    let by_id: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| cp.cluster.id == q)
        .collect();
    if by_id.len() == 1 {
        return Ok(by_id[0]);
    }

    let by_label: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| cp.cluster.label.eq_ignore_ascii_case(q))
        .collect();
    if by_label.len() == 1 {
        return Ok(by_label[0]);
    }

    let q_lower = q.to_lowercase();
    let substr: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| {
            cp.cluster.id.contains(q) || cp.cluster.label.to_lowercase().contains(&q_lower)
        })
        .collect();

    if substr.is_empty() {
        bail!("no cluster matches {:?}", q);
    }
    if substr.len() > 1 {
        let ids: Vec<String> = substr
            .iter()
            .map(|cp| format!("{} ({})", cp.cluster.label, cp.cluster.id))
            .collect();
        bail!("ambiguous cluster query {:?}: {}", q, ids.join("; "));
    }
    Ok(substr[0])
}

fn cluster_for_member<'a>(
    plan: &'a PlanDocument,
    kind: MemberKind,
    id: &str,
) -> Result<&'a ClusterPlan> {
    let id = id.trim();
    if id.is_empty() {
        bail!("member id is empty");
    }
    let matches: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| {
            cp.cluster
                .members
                .iter()
                .any(|m| m.kind == kind && m.id == id)
        })
        .collect();
    match matches.len() {
        0 => bail!("no cluster contains {} member {}", kind_label(&kind), id),
        1 => Ok(matches[0]),
        _ => bail!(
            "internal error: multiple clusters contain the same member {:?}",
            id
        ),
    }
}

fn kind_label(k: &MemberKind) -> &'static str {
    match k {
        MemberKind::Clone => "clone",
        MemberKind::Remote => "remote",
    }
}

fn render_explain_text(cp: &ClusterPlan, snapshot: &InventorySnapshot) -> String {
    let c = &cp.cluster;
    let mut out = String::new();
    out.push_str(&format!(
        "cluster: {} ({}) [{}]\n",
        c.label, c.id, c.cluster_key
    ));
    out.push_str(&format!(
        "status: {:?}  confidence: {:.2}\n",
        c.status, c.confidence
    ));
    out.push_str(&format!(
        "scores: canonical {:.1}  usability {:.1}  recoverability {:.1}  publish_readiness {:.1}  risk {:.1}\n",
        c.scores.canonical,
        c.scores.usability,
        c.scores.recoverability,
        c.scores.oss_readiness,
        c.scores.risk
    ));
    if let Some(cc) = &c.canonical_clone_id {
        let path = snapshot
            .clones
            .iter()
            .find(|cl| cl.id == *cc)
            .map(|cl| cl.path.as_str())
            .unwrap_or("(path not in inventory)");
        out.push_str(&format!("canonical clone: {} — {}\n", cc, path));
    } else {
        out.push_str("canonical clone: (none)\n");
    }
    if let Some(cr) = &c.canonical_remote_id {
        let url = snapshot
            .remotes
            .iter()
            .find(|r| r.id == *cr)
            .map(|r| r.normalized_url.as_str())
            .unwrap_or("(url not in inventory)");
        out.push_str(&format!("canonical remote: {} — {}\n", cr, url));
    } else {
        out.push_str("canonical remote: (none)\n");
    }

    out.push_str(&format!("\nmembers ({}):\n", c.members.len()));
    for m in &c.members {
        let detail = match m.kind {
            MemberKind::Clone => snapshot
                .clones
                .iter()
                .find(|cl| cl.id == m.id)
                .map(|cl| format!(" — {}", cl.path))
                .unwrap_or_default(),
            MemberKind::Remote => snapshot
                .remotes
                .iter()
                .find(|r| r.id == m.id)
                .map(|r| format!(" — {}", r.normalized_url))
                .unwrap_or_default(),
        };
        out.push_str(&format!("  {:?} {}{}\n", m.kind, m.id, detail));
    }

    out.push_str(&format!("\nevidence ({}):\n", c.evidence.len()));
    for ev in &c.evidence {
        out.push_str(&format!(
            "  [{}] {:?} {}  Δ{:.2} — {}\n",
            ev.id, ev.subject_kind, ev.subject_id, ev.score_delta, ev.kind
        ));
    }

    out.push_str(&format!("\nactions ({}):\n", cp.actions.len()));
    for a in &cp.actions {
        out.push_str(&format!(
            "  [{:?}] {:?} {} — {}\n",
            a.priority, a.action_type, a.target_id, a.reason
        ));
    }

    out
}
