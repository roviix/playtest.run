use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod routes {
    pub const COLLECTIONS: &str = "/v1/collections";
    pub const OPEN: &str = "/v1/collections/open";
    pub const COLLECTION: &str = "/v1/collections/{slug}";
    pub const ENTRIES: &str = "/v1/collections/{slug}/entries";
    pub const ENTRY: &str = "/v1/collections/{slug}/entries/{site}";
    pub const BLOCK: &str = "/v1/collections/{slug}/blocks/{site}";
    pub const MODERATE: &str = "/v1/admin/collections/{slug}";
}

pub const PREFIX: &str = "/c/";
pub const INDEX: &str = "/collections";
pub const MAX_COLLECTIONS: usize = 10;
pub const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CollectionKind {
    #[default]
    Collection,
    Challenge,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CreationMethod {
    #[default]
    Unspecified,
    OneShot,
    Iterated,
    Edited,
}

impl CreationMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unspecified => "未说明",
            Self::OneShot => "一次生成",
            Self::Iterated => "多轮修改",
            Self::Edited => "人工修改",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CollectionDraft {
    #[serde(default)]
    pub slug: Option<String>,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub kind: CollectionKind,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub rules: String,
    #[serde(default)]
    pub closes_at: Option<String>,
    #[serde(default)]
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EntryDraft {
    pub slug: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub method: CreationMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CollectionEntry {
    pub slug: String,
    pub title: String,
    pub submitted_version: u32,
    pub submitted_at: String,
    pub model: String,
    pub prompt: String,
    pub method: CreationMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Collection {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub kind: CollectionKind,
    pub prompt: String,
    pub rules: String,
    pub closes_at: Option<String>,
    pub public: bool,
    pub hidden: bool,
    pub creator: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub entries: Vec<CollectionEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_slugs: Vec<String>,
}

impl Collection {
    pub fn path(&self) -> String {
        format!("{PREFIX}{}", self.slug)
    }

    pub fn closed(&self, now: &str) -> bool {
        self.closes_at.as_deref().is_some_and(|close| close <= now)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModerateCollectionRequest {
    pub hidden: bool,
}
