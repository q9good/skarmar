use super::{
    model::Goal,
    process_model::{Audit, Source},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Write {
    pub id: String,
    pub operation_id: String,
    pub expected_version: Option<u64>,
    pub kind: String,
    pub action: String,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    #[serde(default)]
    pub creator_id: String,
    #[serde(default)]
    pub creator_name: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub source_deleted: bool,
    pub id: String,
    pub kind: String,
    pub version: u64,
    pub input: Value,
    pub audit: Audit,
    pub deleted: bool,
    pub goal: Option<Goal>,
    pub metric: Option<Box<Record>>,
    pub source_snapshot: Option<Value>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    pub goal_id: String,
    pub goal_version: u64,
    pub name: String,
    pub template: String,
    #[serde(default)]
    pub criterion: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub support: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub metric_id: String,
    pub metric_version: u64,
    pub business_date: String,
    #[serde(default)]
    pub support: String,
    #[serde(default)]
    pub opportunities: Option<u32>,
    #[serde(default)]
    pub achieved: Option<u32>,
    #[serde(default)]
    pub amount: Option<f64>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub source: Option<Source>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Moment {
    pub business_date: String,
    pub text: String,
    #[serde(default)]
    pub goal_ids: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Needs {
    #[serde(default)]
    pub communication: String,
    #[serde(default)]
    pub sensory: String,
    #[serde(default)]
    pub discomfort: String,
    #[serde(default)]
    pub calming: String,
    #[serde(default)]
    pub health: String,
}
