use crate::model::Level;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalState {
    NotStarted,
    Active,
    Maintaining,
    Paused,
    Achieved,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    Weekly,
    Fortnightly,
    Monthly,
    None,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationTemplate {
    Count,
    Duration,
    Steps,
    Text,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GoalInput {
    /// Family-selected display priority for long-term directions (0 low, 1 normal, 2 high).
    /// Stored with the goal JSON, shared and revisioned like its other fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_priority: Option<u8>,
    #[serde(default)]
    pub resume_on: Option<String>,
    pub title: String,
    pub level: Level,
    #[serde(default)]
    pub meaning: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub domain: Option<CatalogSnapshot>,
    #[serde(default)]
    pub criteria: String,
    #[serde(default)]
    pub support: String,
    #[serde(default)]
    pub template: Option<ObservationTemplate>,
    #[serde(default)]
    pub state: Option<GoalState>,
    #[serde(default)]
    pub archived: bool,
    // Missing inherits a default; Some(None) explicitly disables frequency ranking.
    #[serde(default)]
    pub frequency: Option<Frequency>,
    #[serde(default)]
    pub attention_start: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Goal {
    pub id: String,
    pub version: u64,
    pub input: GoalInput,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MethodState {
    Active,
    Paused,
    Retired,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MethodInput {
    #[serde(default)]
    pub basis: Vec<MethodBasis>,
    #[serde(default)]
    pub resume_on: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub steps: String,
    #[serde(default)]
    pub observe: String,
    #[serde(default)]
    pub scope: String,
    pub goal_ids: Vec<String>,
    #[serde(default)]
    pub guidance: String,
    #[serde(default)]
    pub state: Option<MethodState>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub needs_attention: bool,
    #[serde(default)]
    pub attention_reason: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MethodBasis {
    pub id: String,
    pub version: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MethodBasisSnapshot {
    pub id: String,
    pub version: u64,
    pub content: String,
    pub kind: String,
    pub conditions: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Method {
    #[serde(default)]
    pub basis_snapshots: Vec<MethodBasisSnapshot>,
    pub id: String,
    pub version: u64,
    pub input: MethodInput,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogKind {
    Course,
    Teacher,
    Domain,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogInput {
    pub kind: CatalogKind,
    pub name: String,
    pub active: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CatalogItem {
    pub id: String,
    pub version: u64,
    pub input: CatalogInput,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Mutation {
    Goal(GoalInput),
    Method(MethodInput),
    Catalog(CatalogInput),
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MethodCreateMode {
    Full,
    Quick,
}
#[derive(Clone, Debug)]
pub struct Write {
    /// Deserialization-only field presence. None denotes a fully specified Rust command.
    pub provided_fields: Option<std::collections::BTreeSet<String>>,
    pub method_change: Option<String>,
    pub method_create_mode: Option<MethodCreateMode>,
    pub operation_id: String,
    pub id: String,
    pub expected_version: Option<u64>,
    pub change_note: String,
    pub mutation: Mutation,
}
impl<'de> Deserialize<'de> for Write {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(default)]
            method_change: Option<String>,
            #[serde(default)]
            method_create_mode: Option<String>,
            operation_id: String,
            id: String,
            expected_version: Option<u64>,
            #[serde(default)]
            change_note: String,
            mutation: Mutation,
        }
        let raw = serde_json::Value::deserialize(deserializer)?;
        let fields = raw
            .pointer("/mutation/value")
            .and_then(serde_json::Value::as_object)
            .map(|v| v.keys().cloned().collect());
        let wire: Wire = serde_json::from_value(raw).map_err(serde::de::Error::custom)?;
        let method_create_mode = match wire.method_create_mode.as_deref() {
            None => None,
            Some("full") => Some(MethodCreateMode::Full),
            Some("quick") => Some(MethodCreateMode::Quick),
            Some(_) => {
                return Err(serde::de::Error::custom("方法创建模式须为 full 或 quick"));
            }
        };
        Ok(Self {
            provided_fields: fields,
            method_change: wire.method_change,
            method_create_mode,
            operation_id: wire.operation_id,
            id: wire.id,
            expected_version: wire.expected_version,
            change_note: wire.change_note,
            mutation: wire.mutation,
        })
    }
}
impl Serialize for Write {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            method_change: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            method_create_mode: Option<&'a MethodCreateMode>,
            operation_id: &'a str,
            id: &'a str,
            expected_version: Option<u64>,
            change_note: &'a str,
            mutation: serde_json::Value,
        }
        let mut mutation =
            serde_json::to_value(&self.mutation).map_err(serde::ser::Error::custom)?;
        if let Some(fields) = &self.provided_fields {
            let values = mutation["value"]
                .as_object_mut()
                .ok_or_else(|| serde::ser::Error::custom("mutation fields missing"))?;
            values.retain(|k, _| fields.contains(k));
            for field in fields {
                values
                    .entry(field.clone())
                    .or_insert(serde_json::Value::Null);
            }
        }
        Wire {
            method_change: self.method_change.as_deref(),
            method_create_mode: self.method_create_mode.as_ref(),
            operation_id: &self.operation_id,
            id: &self.id,
            expected_version: self.expected_version,
            change_note: &self.change_note,
            mutation,
        }
        .serialize(serializer)
    }
}
impl Write {
    /// Match the pre-field-mask operation representation exactly for retries
    /// already persisted by an older build. Never use this as a patch payload.
    pub fn legacy_json(&self) -> serde_json::Result<String> {
        #[derive(Serialize)]
        struct Legacy<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            method_change: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            method_create_mode: Option<&'a MethodCreateMode>,
            operation_id: &'a str,
            id: &'a str,
            expected_version: Option<u64>,
            change_note: &'a str,
            mutation: &'a Mutation,
        }
        serde_json::to_string(&Legacy {
            method_change: self.method_change.as_deref(),
            method_create_mode: self.method_create_mode.as_ref(),
            operation_id: &self.operation_id,
            id: &self.id,
            expected_version: self.expected_version,
            change_note: &self.change_note,
            mutation: &self.mutation,
        })
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Saved {
    Goal(Goal),
    Method(Method),
    Catalog(CatalogItem),
}
impl Saved {
    pub fn id(&self) -> &str {
        match self {
            Self::Goal(v) => &v.id,
            Self::Method(v) => &v.id,
            Self::Catalog(v) => &v.id,
        }
    }
    pub fn version(&self) -> u64 {
        match self {
            Self::Goal(v) => v.version,
            Self::Method(v) => v.version,
            Self::Catalog(v) => v.version,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Revision {
    pub entity: Saved,
    pub operation_id: String,
    pub actor_id: String,
    pub recorded_at: String,
    pub event: String,
    pub change_note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snapshot {
    pub schema: String,
    pub goals: Vec<Goal>,
    pub methods: Vec<Method>,
    pub catalog: Vec<CatalogItem>,
    pub lessons: Vec<super::lesson_model::Lesson>,
    pub sessions: Vec<super::session_model::Session>,
    pub knowledge: Vec<super::lesson_model::Knowledge>,
    pub actions: Vec<super::lesson_model::NextAction>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CatalogSnapshot {
    pub id: String,
    pub version: u64,
    pub kind: CatalogKind,
    pub name: String,
}
