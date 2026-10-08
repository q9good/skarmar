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
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct SaveSession {
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub action: String,
    pub session: SessionInput,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SaveResult {
    pub session: Session,
    pub difficulty_id: Option<String>,
    pub experience_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowUp {
    pub id: String,
    pub version: u64,
    pub source_session_id: String,
    pub content: String,
    pub kind: Option<String>,
    pub status: String,
    pub next_review_date: Option<String>,
    pub conclusion: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateFollowUp {
    pub expected_version: u64,
    pub status: String,
    pub next_review_date: Option<String>,
    pub conclusion: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    pub goals: Vec<Goal>,
    pub sessions: Vec<Session>,
    pub difficulties: Vec<FollowUp>,
    pub experiences: Vec<FollowUp>,
}
