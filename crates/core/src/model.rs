use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Long,
    Medium,
    Short,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Active,
    Archived,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalProgress {
    Cultivating,
    Internalized,
    NotStarted,
    InProgress,
    Achieved,
    Completed,
    Overdue,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub level: Level,
    pub parent_id: Option<String>,
    pub area: String,
    pub criteria: String,
    pub status: GoalStatus,
    pub version: u64,
    #[serde(default)]
    pub progress_status: Option<GoalProgress>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub last_review_date: Option<String>,
    #[serde(default)]
    pub next_review_date: Option<String>,
    #[serde(default)]
    pub completion_date: Option<String>,
    #[serde(default)]
    pub review_notes: String,
}

impl Goal {
    pub fn is_reviewable(&self) -> bool {
        self.level != Level::Long
            && (self.status == GoalStatus::Archived
                || matches!(
                    self.progress_status,
                    Some(GoalProgress::Achieved | GoalProgress::Completed)
                ))
    }

    pub fn is_trainable(&self) -> bool {
        self.level != Level::Long && self.status == GoalStatus::Active && !self.is_reviewable()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateGoal {
    pub expected_version: u64,
    pub title: String,
    pub parent_id: Option<String>,
    pub area: String,
    pub criteria: String,
    pub status: GoalStatus,
    pub progress_status: Option<GoalProgress>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub last_review_date: Option<String>,
    pub next_review_date: Option<String>,
    pub completion_date: Option<String>,
    pub review_notes: String,
    pub change_note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GoalRevision {
    pub goal: Goal,
    pub recorded_at: String,
    pub change_note: String,
    pub source_session_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewGoal {
    pub id: String,
    pub title: String,
    pub level: Level,
    pub parent_id: Option<String>,
    pub area: String,
    pub criteria: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Primary,
    Secondary,
    Review,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Review => "review",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Continue,
    Archive,
    Adjust,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetInput {
    pub goal_id: String,
    pub role: Role,
    pub plan: String,
    pub progress: String,
    pub outcome: Option<Outcome>,
    pub next_step: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionInput {
    pub business_date: String,
    pub time_slot: String,
    pub activities: Vec<String>,
    pub organization: String,
    pub plan: String,
    pub observation: String,
    pub targets: Vec<TargetInput>,
    pub has_difficulty: bool,
    pub difficulty: String,
    pub has_experience: bool,
    pub experience: String,
    pub experience_kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub difficulty_goal_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub difficulty_pathway_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub experience_goal_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub experience_pathway_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Target {
    #[serde(flatten)]
    pub input: TargetInput,
    pub goal_snapshot: Goal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Session {
    pub id: String,
    pub version: u64,
    pub status: String,
    #[serde(flatten)]
    pub input: SessionInput,
    pub target_snapshots: Vec<Target>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveSession {
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub action: String,
    pub session: SessionInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<SessionRevisionInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionRevisionInput {
    pub note: String,
    pub difficulty_version: Option<u64>,
    pub experience_version: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionRevision {
    pub session: Session,
    pub recorded_at: String,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteRecord {
    pub expected_version: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeletedRecord {
    pub kind: String,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveResult {
    pub session: Session,
    pub difficulty_id: Option<String>,
    pub experience_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LessonInput {
    pub business_date: String,
    pub course: String,
    pub teacher: String,
    pub content: String,
    pub goals: String,
    pub problems: String,
    pub home_cooperation: String,
    /// Omission preserves links for legacy clients; explicit empty links clear them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<LessonLinks>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct LessonLinks {
    #[serde(default)]
    pub targets: Vec<LessonTarget>,
    #[serde(default)]
    pub pathway_ids: Vec<String>,
    #[serde(default)]
    pub track_problem: bool,
    #[serde(default)]
    pub problem_goal_ids: Vec<String>,
    #[serde(default)]
    pub problem_pathway_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LessonTarget {
    pub goal_id: String,
    pub role: Role,
    pub plan: String,
    pub progress: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LessonTargetSnapshot {
    pub goal_id: String,
    pub goal_snapshot: Goal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lesson {
    pub id: String,
    pub version: u64,
    #[serde(flatten)]
    pub input: LessonInput,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_snapshots: Vec<LessonTargetSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveLesson {
    pub expected_version: Option<u64>,
    pub lesson: LessonInput,
    #[serde(default)]
    pub difficulty_version: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LessonOption {
    pub kind: String,
    pub value: String,
    pub active: bool,
    pub version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renamed_to: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewLessonOption {
    pub kind: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateLessonOption {
    pub kind: String,
    pub value: String,
    pub active: bool,
    pub expected_version: u64,
    #[serde(default)]
    pub new_value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowUp {
    pub id: String,
    pub version: u64,
    pub source_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_lesson_id: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub source_deleted: bool,
    pub content: String,
    pub kind: Option<String>,
    pub status: String,
    pub next_review_date: Option<String>,
    pub conclusion: String,
    #[serde(default)]
    pub goal_ids: Vec<String>,
    #[serde(default)]
    pub pathway_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateFollowUp {
    pub expected_version: u64,
    pub status: String,
    pub next_review_date: Option<String>,
    pub conclusion: String,
    #[serde(default)]
    pub content: Option<String>,
    /// Omitted bindings preserve existing relationships, including for older clients.
    #[serde(default)]
    pub goal_ids: Option<Vec<String>>,
    #[serde(default)]
    pub pathway_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PathwayStatus {
    Active,
    Archived,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pathway {
    pub id: String,
    pub version: u64,
    pub title: String,
    pub description: String,
    pub steps: String,
    pub status: PathwayStatus,
    pub goal_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewPathway {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: String,
    pub goal_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdatePathway {
    pub expected_version: u64,
    pub title: String,
    pub description: String,
    pub steps: String,
    pub status: PathwayStatus,
    pub goal_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewExperience {
    pub id: String,
    pub content: String,
    pub goal_ids: Vec<String>,
    pub pathway_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppState {
    pub goals: Vec<Goal>,
    pub sessions: Vec<Session>,
    pub difficulties: Vec<FollowUp>,
    pub experiences: Vec<FollowUp>,
    #[serde(default)]
    pub pathways: Vec<Pathway>,
    #[serde(default)]
    pub lessons: Vec<Lesson>,
    #[serde(default)]
    pub lesson_options: Vec<LessonOption>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deleted: Vec<DeletedRecord>,
}

/// Business data captured in a single transaction. Never includes account/session tables.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub exported_at: String,
    pub state: AppState,
    pub goal_history: Vec<GoalRevision>,
}
