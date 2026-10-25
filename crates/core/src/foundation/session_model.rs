use super::{
    lesson_model::{NewAction, NewKnowledge},
    model::{Goal, Method},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MethodRef {
    pub id: String,
    pub version: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Main,
    Secondary,
    Review,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub goal_id: String,
    pub role: Role,
    #[serde(default)]
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetSnapshot {
    pub input: Target,
    pub goal: Goal,
    pub ancestors: Vec<Goal>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActualTarget {
    pub goal_id: String,
    pub performed: bool,
    #[serde(default)]
    pub observation: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confirmation {
    #[default]
    Unconfirmed,
    Used,
    None,
    Unknown,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Signal {
    pub kind: String,
    #[serde(default)]
    pub goal_ids: Vec<String>,
    #[serde(default)]
    pub method_ids: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOutcome {
    pub action_id: String,
    pub decision: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    #[serde(default)]
    pub targets: Vec<ActualTarget>,
    #[serde(default)]
    pub confirmation: Confirmation,
    #[serde(default)]
    pub methods: Vec<MethodRef>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub signals: Vec<Signal>,
    #[serde(default)]
    pub first_try_ids: Vec<String>,
    #[serde(default)]
    pub new_knowledge: Vec<NewKnowledge>,
    #[serde(default)]
    pub feedback: Vec<super::process_model::PracticeFeedback>,
    #[serde(default)]
    pub knowledge_versions: std::collections::BTreeMap<String, u64>,
    /// Missing means an older client did not submit structured knowledge details.
    /// A present entry may explicitly set or clear the summary-owned fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_details:
        Option<std::collections::BTreeMap<String, super::process_model::Details>>,
    #[serde(default)]
    pub new_actions: Vec<NewAction>,
    #[serde(default)]
    pub action_outcomes: Vec<ActionOutcome>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LessonSource {
    pub id: String,
    pub version: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub business_date: String,
    pub time_slot: String,
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub targets: Vec<Target>,
    #[serde(default)]
    pub methods: Vec<MethodRef>,
    #[serde(default)]
    pub focus_goal_ids: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub lesson_sources: Vec<LessonSource>,
    #[serde(default)]
    pub action_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub action_versions: std::collections::BTreeMap<String, u64>,
    #[serde(default)]
    pub summary: Option<Summary>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    SaveDraft,
    Complete,
    Cancel,
    Delete,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Write {
    pub id: String,
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub action: Action,
    #[serde(default)]
    pub input: Option<Input>,
    #[serde(default)]
    pub change_note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Session {
    pub id: String,
    pub version: u64,
    pub status: String,
    pub creator_id: String,
    pub creator_name: String,
    pub created_at: String,
    pub input: Input,
    pub planned_targets: Vec<TargetSnapshot>,
    pub actual_targets: Vec<TargetSnapshot>,
    pub planned_methods: Vec<Method>,
    pub actual_methods: Vec<Method>,
    pub focus_goals: Vec<Goal>,
    pub lesson_sources: Vec<super::lesson_model::Lesson>,
    pub action_snapshots: Vec<super::lesson_model::NextAction>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Revision {
    pub session: Session,
    pub actor_id: String,
    pub actor_name: String,
    pub recorded_at: String,
    pub event: String,
    pub change_note: String,
}
