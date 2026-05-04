use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorCode {
    /// The provided query was empty or invalid.
    InvalidQuery,
    /// No cluster could be found matching the query.
    NoClusterMatch,
    /// The query was ambiguous and matched multiple clusters.
    AmbiguousQuery,
    /// An unexpected internal error occurred.
    InternalError,
}

#[derive(Debug, Error, Serialize)]
#[error("{message}")]
pub struct AgentError {
    pub code: AgentErrorCode,
    pub message: String,
}

impl AgentError {
    pub fn new(code: AgentErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
