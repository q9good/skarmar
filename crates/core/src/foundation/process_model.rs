use super::{
    lesson_model::{NewAction, NewKnowledge},
    model::Method,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Details {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub priority: bool,
    #[serde(default)]
    pub key: bool,
    #[serde(default)]
    pub expected: String,
    #[serde(default)]
    pub adult_response: String,
    #[serde(default)]
    pub guess: String,
    #[serde(default)]
    pub conditions: String,
    #[serde(default)]
    pub evidence: String,
    #[serde(default)]
    pub comparison_start: Option<String>,
    #[serde(default)]
    pub basis: String,
    #[serde(default)]
    pub support: String,
    #[serde(default)]
    pub pause_until: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Audit {
    pub actor_id: String,
    pub actor_name: String,
    pub at: String,
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Derivation {
    pub id: String,
    pub version: u64,
    pub difference: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeWrite {
    pub id: String,
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub action: String,
    #[serde(default)]
    pub input: Option<NewKnowledge>,
    #[serde(default)]
    pub details: Details,
    #[serde(default)]
    pub derived_from: Vec<Derivation>,
    #[serde(default)]
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub kind: String,
    pub id: String,
    pub version: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_version: Option<u64>,
    pub knowledge_id: String,
    pub source: Source,
    pub business_date: String,
    pub result: String,
    #[serde(default)]
    pub observation: String,
    #[serde(default)]
    pub attempt: String,
    #[serde(default)]
    pub adult_response: String,
    #[serde(default)]
    pub conditions: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub comparable: bool,
    #[serde(default)]
    pub basis: String,
    #[serde(default)]
    pub support: String,
    #[serde(default)]
    pub method_refs: Vec<super::session_model::MethodRef>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Feedback {
    pub id: String,
    pub version: u64,
    pub input: FeedbackInput,
    pub methods: Vec<Method>,
    pub source_snapshot: serde_json::Value,
    pub author_id: String,
    pub author_name: String,
    pub created_at: String,
    pub audit: Audit,
    pub deleted: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Annotation {
    pub id: String,
    pub event_id: String,
    pub based_on_version: u64,
    pub text: String,
    pub audit: Audit,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackWrite {
    pub id: String,
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub action: String,
    #[serde(default)]
    pub input: Option<FeedbackInput>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub next_actions: Vec<NewAction>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    pub version: u64,
    pub weekday: u32,
    pub fortnight_anchor: String,
    pub monthly_ordinal: i32,
    pub audit: Audit,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsWrite {
    pub operation_id: String,
    pub expected_version: u64,
    pub weekday: u32,
    pub fortnight_anchor: String,
    pub monthly_ordinal: i32,
    #[serde(default)]
    pub reorder_overdue: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Schedule {
    pub object_kind: String,
    pub object_id: String,
    pub version: u64,
    pub generation: u64,
    pub due_date: Option<String>,
    pub interval: String,
    pub manual: bool,
    pub reminders: bool,
    pub eligible: bool,
    pub mode: String,
    pub signature: String,
    pub last_completed: Option<String>,
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Review {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caregiver_state: Option<String>,
    pub id: String,
    pub version: u64,
    pub object_kind: String,
    pub object_id: String,
    pub generation: u64,
    pub due_date: String,
    pub completed_on: String,
    pub decision: String,
    pub note: String,
    pub evidence: Vec<Source>,
    pub evidence_snapshots: Vec<serde_json::Value>,
    pub previous_actions: Vec<String>,
    #[serde(default)]
    pub seen_versions: std::collections::BTreeMap<String, u64>,
    pub action_ids: Vec<String>,
    pub audit: Audit,
    pub deleted: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewWrite {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caregiver_state: Option<String>,
    pub id: String,
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub object_kind: String,
    pub object_id: String,
    pub object_version: u64,
    pub schedule_version: u64,
    pub generation: u64,
    pub due_date: Option<String>,
    pub action: String,
    #[serde(default)]
    pub decision: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub change_text: String,
    #[serde(default)]
    pub evidence: Vec<Source>,
    #[serde(default)]
    pub next_actions: Vec<NewAction>,
    #[serde(default)]
    pub next_date: Option<String>,
    #[serde(default = "yes")]
    pub reminders: bool,
    #[serde(default)]
    pub group_tokens: Vec<ScheduleToken>,
    #[serde(default)]
    pub seen_versions: std::collections::BTreeMap<String, u64>,
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Read {
    pub view: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub object_kind: String,
    #[serde(default)]
    pub page: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Command {
    Observe(super::observation_model::Write),
    Knowledge(KnowledgeWrite),
    Action(ActionWrite),
    Feedback(FeedbackWrite),
    Settings(SettingsWrite),
    Review(ReviewWrite),
    Read(Read),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeFeedback {
    pub id: String,
    pub knowledge_id: String,
    pub result: String,
    #[serde(default)]
    pub observation: String,
    #[serde(default)]
    pub attempt: String,
    #[serde(default)]
    pub adult_response: String,
    #[serde(default)]
    pub conditions: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub comparable: bool,
    #[serde(default)]
    pub basis: String,
    #[serde(default)]
    pub support: String,
    #[serde(default)]
    pub expected_version: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleToken {
    pub object_kind: String,
    pub object_id: String,
    pub object_version: u64,
    pub schedule_version: u64,
    pub generation: u64,
    pub due_date: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionWrite {
    pub id: String,
    pub operation_id: String,
    pub expected_version: u64,
    pub input: NewAction,
    pub state: String,
    #[serde(default)]
    pub note: String,
}
