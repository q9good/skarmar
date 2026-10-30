use super::model::{CatalogSnapshot, Goal, Method};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub course_started: Option<bool>,
    pub business_date: String,
    pub course_id: String,
    pub teacher_id: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub teacher_notes: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub helpful_notes: String,
    #[serde(default)]
    pub progress_notes: String,
    #[serde(default)]
    pub interest_notes: String,
    #[serde(default)]
    pub difficulty_notes: String,
    #[serde(default)]
    pub cause_guess: String,
    #[serde(default)]
    pub request_review: bool,
    #[serde(default)]
    pub review_message: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LessonTarget {
    pub goal_id: String,
    #[serde(default)]
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    Experience,
    Difficulty,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceStage {
    Idea,
    Try,
    Helpful,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NewKnowledge {
    pub id: String,
    pub kind: KnowledgeKind,
    pub content: String,
    #[serde(default)]
    pub goal_ids: Vec<String>,
    #[serde(default)]
    pub method_ids: Vec<String>,
    #[serde(default)]
    pub stage: Option<ExperienceStage>,
    #[serde(default)]
    pub next_review_date: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewAction {
    #[serde(default)]
    pub suggested_date: Option<String>,
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub goal_ids: Vec<String>,
    #[serde(default)]
    pub method_ids: Vec<String>,
    #[serde(default)]
    pub assignee_id: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Processing {
    #[default]
    Keep,
    Claim,
    TakeOver,
    Complete,
    Reopen,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationInput {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feedback: Vec<super::process_model::PracticeFeedback>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub feedback_knowledge_versions: std::collections::BTreeMap<String, u64>,
    pub seen_observation_version: u64,
    #[serde(default)]
    pub targets: Vec<LessonTarget>,
    #[serde(default)]
    pub method_ids: Vec<String>,
    #[serde(default)]
    pub focus_goal_ids: Vec<String>,
    #[serde(default)]
    pub processing: Processing,
    #[serde(default)]
    pub request_generation: u64,
    #[serde(default)]
    pub outcome: String,
    #[serde(default)]
    pub new_knowledge: Vec<NewKnowledge>,
    #[serde(default)]
    pub new_actions: Vec<NewAction>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LessonAction {
    Observation(ObservationInput),
    Organization(OrganizationInput),
    Withdraw,
    MarkError { marked: bool },
    Delete,
    Acknowledge,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LessonWrite {
    pub id: String,
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub action: LessonAction,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetSnapshot {
    pub target: LessonTarget,
    pub goal: Goal,
    #[serde(default)]
    pub ancestors: Vec<Goal>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Organization {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feedback_ids: Vec<String>,
    pub seen_observation_version: u64,
    pub targets: Vec<TargetSnapshot>,
    pub methods: Vec<Method>,
    pub focus_goals: Vec<Goal>,
    pub processed_request_generation: u64,
    pub state: String,
    pub claimed_by: Option<String>,
    pub claimed_name: Option<String>,
    pub outcome: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lesson {
    pub id: String,
    pub version: u64,
    pub observation_version: u64,
    pub organization_version: u64,
    pub created_at: String,
    pub creator_id: String,
    pub creator_name: String,
    pub status: String,
    pub marked_error: bool,
    pub observation: ObservationInput,
    pub course: CatalogSnapshot,
    pub teacher: CatalogSnapshot,
    pub organization: Organization,
    pub request_generation: u64,
    pub requested_by: Option<String>,
    pub request_name: Option<String>,
    pub request_message: String,
    pub acknowledgements: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WithdrawalCapability {
    pub allowed: bool,
    pub reason_code: String,
    pub reason: String,
    pub lesson_version: u64,
    pub references: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LessonRevision {
    pub lesson: Lesson,
    pub actor_id: String,
    pub actor_name: String,
    pub event: String,
    pub recorded_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Knowledge {
    pub input: NewKnowledge,
    #[serde(default)]
    pub details: super::process_model::Details,
    #[serde(default)]
    pub derived_from: Vec<super::process_model::Derivation>,
    #[serde(default)]
    pub source_event_id: Option<String>,
    #[serde(default)]
    pub source_review_id: Option<String>,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub audit: Option<super::process_model::Audit>,
    pub version: u64,
    pub source_lesson_id: Option<String>,
    #[serde(default)]
    pub source_session_id: Option<String>,
    #[serde(default)]
    pub source_session_version: Option<u64>,
    pub source_observation_version: u64,
    pub source_observation: Option<ObservationInput>,
    #[serde(default)]
    pub source_summary: Option<super::session_model::Summary>,
    pub author_id: String,
    pub author_name: String,
    pub created_at: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NextAction {
    #[serde(default)]
    pub audit: Option<super::process_model::Audit>,
    #[serde(default)]
    pub source_deleted: bool,
    #[serde(default)]
    pub source_event_id: Option<String>,
    #[serde(default)]
    pub source_review_id: Option<String>,
    pub input: NewAction,
    pub version: u64,
    pub source_lesson_id: Option<String>,
    #[serde(default)]
    pub source_session_id: Option<String>,
    #[serde(default)]
    pub source_session_version: Option<u64>,
    pub source_observation_version: u64,
    pub author_id: String,
    pub author_name: String,
    pub assignee_name: Option<String>,
    pub status: String,
    #[serde(default)]
    pub plan_id: Option<String>,
    pub created_at: String,
}

/// Only explicitly submitted basic records enter this queue; the authenticated
/// actor is checked in the same transaction as each idempotent write.
#[derive(Clone, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum QueuedWrite {
    Catalog(super::model::Write),
    Lesson(LessonWrite),
}
