use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Class(pub u8);
impl Class {
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State(pub String);
impl State {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action(pub String);
impl Action {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Confidence(pub String);
impl Confidence {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowMessage {
    pub class: Class,
    pub origin: String,
    pub state: State,
    pub action: Action,
    pub finality: u8,
    pub confidence: Confidence,
    /// The raw canonical string, e.g. "2:CST:FN:PR:1:C"
    pub raw: String,
}
