use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("IO error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("JSON parse error in {path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
}

#[derive(Debug, Error)]
pub struct ParseError {
    rule_id: String,
    message: String,
}

impl ParseError {
    pub fn new(rule_id: &str, message: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.to_owned(),
            message: message.into(),
        }
    }
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule_id, self.message)
    }
}

#[derive(Debug, Error)]
pub enum StateMachineError {
    #[error("Invalid transition for chain {chain_id}: {from} -> {to} (triggered by {event_id})")]
    InvalidTransition {
        chain_id: String,
        from: String,
        to: String,
        event_id: String,
    },
}
