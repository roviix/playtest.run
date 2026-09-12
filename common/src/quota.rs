use serde::{Deserialize, Serialize};

use crate::plan::Plan;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub owner: String,
    pub plan: Plan,
    pub expires_at: Option<String>,
}

pub fn key(slug: &str) -> String {
    format!("sites/{slug}/policy.json")
}
