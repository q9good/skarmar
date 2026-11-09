use crate::{
    auth::User,
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_date},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub version: u64,
    pub minimum: u16,
    pub maximum: u16,
    pub method_per_goal: u16,
    pub method_bonus_cap: u16,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            version: 0,
            minimum: 0,
            maximum: 100,
            method_per_goal: 2,
            method_bonus_cap: 20,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Write {
    pub operation_id: String,
    pub expected_version: u64,
    pub value: Policy,
}
pub fn read(db: &dyn Sql) -> Result<Policy> {
    if db
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='prd_settings'",
            &[],
        )?
        .is_empty()
    {
        return Ok(Policy::default());
    }
    db.query("SELECT data FROM prd_settings WHERE id='priority'", &[])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .unwrap_or_else(|| Ok(Policy::default()))
}
pub fn write(db: &dyn Sql, _actor: &User, r: Write) -> Result<serde_json::Value> {
    let previous = read(db)?;
    if previous.version != r.expected_version {
        return Err(ApiError::conflict(
            "优先级规则已由家人修改，请保留填写并刷新核对",
        ));
    }
    let mut v = r.value;
    if v.minimum >= v.maximum
        || v.maximum > 10000
        || v.method_per_goal > 1000
        || v.method_bonus_cap > 10000
    {
        return Err(ApiError::invalid(
            "优先级范围须为0至10000内且最小值小于最大值；关联加分不超过1000，上限不超过10000",
        ));
    }
    v.version = previous
        .version
        .checked_add(1)
        .ok_or_else(|| ApiError::conflict("规则版本超出范围"))?;
    db.execute("INSERT INTO prd_settings(id,data) VALUES('priority',?1) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![json(&v)?])?;
    Ok(serde_json::to_value(v)?)
}
pub fn validate(
    db: &dyn Sql,
    score: Option<u16>,
    until: &Option<String>,
    previous: Option<u16>,
) -> Result<()> {
    if score.is_some_and(|v| v > 10000) {
        return Err(ApiError::invalid("数字优先级须为0至10000"));
    }
    let policy = read(db)?;
    if score.is_some_and(|v| (v < policy.minimum || v > policy.maximum) && Some(v) != previous) {
        return Err(ApiError::invalid(
            "基础优先级超出家庭设置范围，请核对后填写",
        ));
    }
    if let Some(date) = until {
        valid_date(date)?;
    }
    Ok(())
}
