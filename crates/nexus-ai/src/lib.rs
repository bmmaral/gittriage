//! Optional AI-assisted explanations for Nexus.
//!
//! Nexus runs correctly without AI. This crate adds optional LLM-powered
//! narrative explanations grounded in the deterministic plan output.
//!
//! The AI never modifies scores, canonical selections, or actions.
//! It only consumes the structured output and produces human-readable summaries.

use anyhow::{bail, Context, Result};
use nexus_core::{ClusterPlan, ClusterStatus, PlanDocument};
use serde::{Deserialize, Serialize};

// ── Config ───────────────────────────────────────────────────────────────────

/// AI configuration, typically loaded from `[ai]` in `nexus.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// Enable AI features. Default: false.
    pub enabled: bool,
    /// OpenAI-compatible API base URL. Default: OpenAI.
    pub api_base: String,
    /// Model name to use. Default: gpt-4o-mini.
    pub model: String,
    /// Maximum tokens for the response.
    pub max_tokens: u32,
    /// Temperature (0.0–1.0). Lower = more deterministic.
    pub temperature: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_base: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            max_tokens: 1024,
            temperature: 0.2,
        }
    }
}

impl AiConfig {
    pub fn validate(&self) -> Result<()> {
        if !self.enabled {
            bail!("AI features are disabled; set `ai.enabled = true` in nexus.toml");
        }
        if resolve_api_key().is_none() {
            bail!("no API key found; set NEXUS_AI_API_KEY or OPENAI_API_KEY environment variable");
        }
        Ok(())
    }
}

/// Resolve the API key from environment. Precedence:
/// 1. `NEXUS_AI_API_KEY`
/// 2. `OPENAI_API_KEY`
pub fn resolve_api_key() -> Option<String> {
    std::env::var("NEXUS_AI_API_KEY")
        .ok()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .filter(|k| !k.trim().is_empty())
}

// ── Grounding ────────────────────────────────────────────────────────────────

/// Build the grounding context for a cluster explanation.
/// This is the structured data the AI receives — it never sees raw repo contents.
fn build_grounding_context(cp: &ClusterPlan) -> String {
    let c = &cp.cluster;
    let status = match c.status {
        ClusterStatus::Resolved => "Resolved",
        ClusterStatus::Ambiguous => "Ambiguous",
        ClusterStatus::ManualReview => "ManualReview",
    };

    let mut ctx = format!(
        "Cluster: {} (key: {}, id: {})\n\
         Status: {} (confidence: {:.2})\n\
         Scores: canonical={:.0}, health={:.0}, recoverability={:.0}, publish={:.0}, risk={:.0}\n\
         Canonical clone: {}\n\
         Canonical remote: {}\n\
         Members: {} clone(s), {} remote(s)\n\n\
         Evidence ({} items):\n",
        c.label,
        c.cluster_key,
        c.id,
        status,
        c.confidence,
        c.scores.canonical,
        c.scores.usability,
        c.scores.recoverability,
        c.scores.oss_readiness,
        c.scores.risk,
        c.canonical_clone_id.as_deref().unwrap_or("none"),
        c.canonical_remote_id.as_deref().unwrap_or("none"),
        c.members
            .iter()
            .filter(|m| m.kind == nexus_core::MemberKind::Clone)
            .count(),
        c.members
            .iter()
            .filter(|m| m.kind == nexus_core::MemberKind::Remote)
            .count(),
        c.evidence.len(),
    );

    for e in &c.evidence {
        ctx.push_str(&format!(
            "  [{:+.0}] {} — {}\n",
            e.score_delta, e.kind, e.detail
        ));
    }

    if !cp.actions.is_empty() {
        ctx.push_str(&format!("\nActions ({}):\n", cp.actions.len()));
        for a in &cp.actions {
            ctx.push_str(&format!(
                "  [{:?}] {:?} → {} — {}\n",
                a.priority, a.action_type, a.target_id, a.reason
            ));
            if let Some(rn) = &a.risk_note {
                ctx.push_str(&format!("    risk: {}\n", rn));
            }
        }
    }

    ctx
}

const SYSTEM_PROMPT: &str = "\
You are Nexus AI, an assistant that explains repository cluster triage results. \
You receive structured scoring data from the Nexus deterministic engine. \
Your role is to provide a clear, concise narrative explanation of what the data means \
and what the user should consider doing next.\n\n\
Rules:\n\
- You MUST NOT invent scores, evidence, or actions that are not in the provided data.\n\
- You MUST NOT claim to modify or override any Nexus scores or decisions.\n\
- You MUST clearly state when something is your interpretation vs deterministic fact.\n\
- Keep explanations concise (3-6 paragraphs max).\n\
- Use plain language; avoid jargon unless the user's data uses it.\n\
- If the cluster status is Ambiguous, emphasize that the user should verify before acting.\n\
- End with 1-3 concrete next steps the user could take.";

// ── API ──────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

/// Generate an AI explanation for a single cluster.
/// Returns the model's narrative text or an error.
pub async fn explain_cluster(config: &AiConfig, cp: &ClusterPlan) -> Result<String> {
    config.validate()?;
    let api_key = resolve_api_key().context("API key disappeared between validate and call")?;

    let grounding = build_grounding_context(cp);
    let user_msg = format!(
        "Explain this Nexus cluster triage result to the user. \
         Here is the structured data:\n\n{}",
        grounding
    );

    let body = ChatRequest {
        model: config.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".into(),
                content: SYSTEM_PROMPT.into(),
            },
            ChatMessage {
                role: "user".into(),
                content: user_msg,
            },
        ],
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    };

    let url = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("AI API request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("AI API returned {status}: {body}");
    }

    let parsed: ChatResponse = resp.json().await.context("failed to parse AI response")?;
    let text = parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "(no response from model)".into());

    Ok(text)
}

/// Generate a plan-wide AI summary.
pub async fn summarize_plan(config: &AiConfig, plan: &PlanDocument) -> Result<String> {
    config.validate()?;
    let api_key = resolve_api_key().context("API key disappeared between validate and call")?;

    let n_clusters = plan.clusters.len();
    let n_actions: usize = plan.clusters.iter().map(|cp| cp.actions.len()).sum();
    let n_ambiguous = plan
        .clusters
        .iter()
        .filter(|cp| matches!(cp.cluster.status, ClusterStatus::Ambiguous))
        .count();

    let mut ctx = format!(
        "Plan summary: {} clusters, {} actions, {} ambiguous, rules v{}\n\n",
        n_clusters, n_actions, n_ambiguous, plan.scoring_rules_version
    );

    for cp in plan.clusters.iter().take(20) {
        let c = &cp.cluster;
        let st = match c.status {
            ClusterStatus::Resolved => "OK",
            ClusterStatus::Ambiguous => "AMB",
            ClusterStatus::ManualReview => "REV",
        };
        ctx.push_str(&format!(
            "  {} [{}] canon={:.0} health={:.0} risk={:.0} actions={}\n",
            c.label,
            st,
            c.scores.canonical,
            c.scores.usability,
            c.scores.risk,
            cp.actions.len()
        ));
    }
    if n_clusters > 20 {
        ctx.push_str(&format!("  ... and {} more clusters\n", n_clusters - 20));
    }

    let user_msg = format!(
        "Provide a brief executive summary of this Nexus plan. \
         Focus on: what needs attention most, overall health, and top 3 priorities.\n\n{}",
        ctx
    );

    let body = ChatRequest {
        model: config.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".into(),
                content: SYSTEM_PROMPT.into(),
            },
            ChatMessage {
                role: "user".into(),
                content: user_msg,
            },
        ],
        max_tokens: config.max_tokens,
        temperature: config.temperature,
    };

    let url = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("AI API request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("AI API returned {status}: {body}");
    }

    let parsed: ChatResponse = resp.json().await.context("failed to parse AI response")?;
    Ok(parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "(no response from model)".into()))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_core::*;

    fn test_cluster() -> ClusterPlan {
        ClusterPlan {
            cluster: ClusterRecord {
                id: "cluster-test".into(),
                cluster_key: "url:github.com/acme/proj".into(),
                label: "proj".into(),
                status: ClusterStatus::Resolved,
                confidence: 0.85,
                canonical_clone_id: Some("clone-1".into()),
                canonical_remote_id: Some("remote-1".into()),
                members: vec![
                    ClusterMember {
                        kind: MemberKind::Clone,
                        id: "clone-1".into(),
                    },
                    ClusterMember {
                        kind: MemberKind::Remote,
                        id: "remote-1".into(),
                    },
                ],
                evidence: vec![EvidenceItem {
                    id: "ev-1".into(),
                    subject_kind: MemberKind::Clone,
                    subject_id: "clone-1".into(),
                    kind: "canonical_clone_pick".into(),
                    score_delta: 14.0,
                    detail: "selected as canonical".into(),
                }],
                scores: ScoreBundle {
                    canonical: 80.0,
                    usability: 60.0,
                    recoverability: 70.0,
                    oss_readiness: 50.0,
                    risk: 24.0,
                },
            },
            actions: vec![PlanAction {
                id: "action-1".into(),
                priority: Priority::High,
                action_type: ActionType::ArchiveLocalDuplicate,
                target_kind: MemberKind::Clone,
                target_id: "clone-2".into(),
                reason: "duplicate".into(),
                commands: vec![],
                evidence_summary: None,
                confidence: Some(0.65),
                risk_note: Some("check first".into()),
            }],
        }
    }

    #[test]
    fn grounding_context_contains_cluster_data() {
        let cp = test_cluster();
        let ctx = build_grounding_context(&cp);
        assert!(ctx.contains("proj"));
        assert!(ctx.contains("canonical_clone_pick"));
        assert!(ctx.contains("ArchiveLocalDuplicate"));
        assert!(ctx.contains("clone-1"));
        assert!(ctx.contains("Resolved"));
    }

    #[test]
    fn grounding_context_includes_all_evidence() {
        let cp = test_cluster();
        let ctx = build_grounding_context(&cp);
        assert!(ctx.contains("[+14]"));
        assert!(ctx.contains("selected as canonical"));
    }

    #[test]
    fn default_config_is_disabled() {
        let cfg = AiConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_fails_without_key() {
        let cfg = AiConfig {
            enabled: true,
            ..Default::default()
        };
        // Will fail unless NEXUS_AI_API_KEY or OPENAI_API_KEY is set in test env
        // This is expected behavior
        let result = cfg.validate();
        if std::env::var("NEXUS_AI_API_KEY").is_err() && std::env::var("OPENAI_API_KEY").is_err() {
            assert!(result.is_err());
        }
    }

    #[test]
    fn system_prompt_has_safety_rules() {
        assert!(SYSTEM_PROMPT.contains("MUST NOT invent"));
        assert!(SYSTEM_PROMPT.contains("MUST NOT claim to modify"));
        assert!(SYSTEM_PROMPT.contains("Ambiguous"));
    }
}
