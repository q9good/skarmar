//! Optional observations and measurements are evidence, never a diagnostic score.
use super::{followups, model::Saved, observation_model::*};
use crate::{
    auth::{AccountRole, User},
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_date, valid_id},
};
use serde_json::{Value, json as value};
pub const NEEDS_ID: &str = "00000000-0000-4000-8000-000000000001";
pub fn get(db: &dyn Sql, id: &str) -> Result<Record> {
    valid_id(id)?;
    db.query("SELECT data FROM prd_observations WHERE id=?1", params![id])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()?
        .ok_or_else(ApiError::missing)
}
pub fn historical(db: &dyn Sql, id: &str, version: u64) -> Result<Record> {
    valid_id(id)?;
    let current = get(db, id)?;
    if current.deleted {
        return Err(ApiError::conflict("来源已删除，请核对"));
    }
    db.query(
        "SELECT data FROM prd_observation_revisions WHERE id=?1 AND version=?2",
        params![id, version as i64],
    )?
    .first()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .transpose()?
    .ok_or_else(ApiError::missing)
}
pub fn list(db: &dyn Sql, kind: &str) -> Result<Vec<Record>> {
    db.query(
        "SELECT data FROM prd_observations WHERE kind=?1 ORDER BY id",
        params![kind],
    )?
    .iter()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .collect()
}
fn goal(db: &dyn Sql, id: &str, version: u64) -> Result<super::model::Goal> {
    super::history(db, id)?
        .into_iter()
        .find_map(|r| match r.entity {
            Saved::Goal(g) if g.version == version => Some(g),
            _ => None,
        })
        .ok_or_else(ApiError::missing)
}
fn parse<T: serde::de::DeserializeOwned>(v: Value) -> Result<T> {
    serde_json::from_value(v)
        .map_err(|_| ApiError::invalid("填写字段与所选模板不匹配，请保留输入后核对"))
}
fn text(v: &str, max: usize) -> Result<()> {
    if v.chars().count() > max {
        return Err(ApiError::invalid("文字过长，请精简后保存"));
    }
    Ok(())
}
fn date(v: &str) -> Result<()> {
    valid_date(v)?;
    if v > super::sessions::today().as_str() {
        return Err(ApiError::invalid("未来日期不能作为实际观察保存"));
    }
    Ok(())
}
pub fn command(db: &dyn Sql, actor: &User, r: Write) -> Result<Value> {
    if !actor.active {
        return Err(crate::auth::unauthorized());
    }
    valid_id(&r.id)?;
    valid_id(&r.operation_id)?;
    let encoded = format!("observation:{}", json(&r)?);
    if let Some(old) = db
        .query(
            "SELECT actor_id,request,response FROM prd_operations WHERE id=?1",
            params![r.operation_id],
        )?
        .first()
    {
        if old[0] != actor.id || old[1] != encoded {
            return Err(ApiError::conflict("操作编号已用于其他内容"));
        }
        return Ok(serde_json::from_str(&old[2])?);
    }
    if !["metric", "measurement", "moment", "needs"].contains(&r.kind.as_str())
        || !["save", "delete"].contains(&r.action.as_str())
    {
        return Err(ApiError::invalid("观察操作类型不正确"));
    }
    let previous = db
        .query(
            "SELECT data FROM prd_observations WHERE id=?1",
            params![r.id],
        )?
        .first()
        .map(|v| serde_json::from_str::<Record>(&v[0]).map_err(ApiError::from))
        .transpose()?;
    if previous.as_ref().map(|r| r.version) != r.expected_version
        || previous
            .as_ref()
            .is_some_and(|p| p.kind != r.kind || p.deleted)
    {
        return Err(ApiError::conflict(
            "已有新版本或记录已删除，当前输入仍需保留核对",
        ));
    }
    if previous.is_some() && r.note.trim().is_empty() {
        return Err(ApiError::invalid("修改或删除时请说明原因"));
    }
    let mut record = Record {
        creator_id: previous
            .as_ref()
            .map_or_else(|| actor.id.clone(), |p| p.creator_id.clone()),
        creator_name: previous
            .as_ref()
            .map_or_else(|| actor.display_name.clone(), |p| p.creator_name.clone()),
        created_at: previous
            .as_ref()
            .map_or_else(|| chrono::Utc::now().to_rfc3339(), |p| p.created_at.clone()),
        source_deleted: false,
        id: r.id.clone(),
        kind: r.kind.clone(),
        version: previous.as_ref().map_or(1, |p| p.version + 1),
        input: r.input.clone(),
        audit: followups::audit(actor, &r.note),
        deleted: false,
        goal: None,
        metric: None,
        source_snapshot: None,
    };
    let mut unique = r.id.clone();
    if r.action == "delete" {
        if actor.role != AccountRole::Admin {
            return Err(ApiError(
                axum::http::StatusCode::FORBIDDEN,
                "只有管理员可以删除记录".into(),
            ));
        }
        let old = previous.as_ref().ok_or_else(ApiError::missing)?;
        if old.kind == "needs" {
            return Err(ApiError::invalid("需要说明请编辑清空，不删除整份共享说明"));
        }
        if old.kind == "metric"
            && list(db, "measurement")?
                .iter()
                .any(|m| !m.deleted && m.input["metric_id"] == old.id)
        {
            return Err(ApiError::conflict(
                "已有测量引用该指标，请保留或创建新的指标版本",
            ));
        }
        record = Record {
            version: record.version,
            audit: record.audit,
            deleted: true,
            ..old.clone()
        };
    } else {
        if json(&r.input)?.len() > 24000 {
            return Err(ApiError::invalid("填写内容过长"));
        }
        match r.kind.as_str() {
            "metric" => {
                let input: Metric = parse(r.input.clone())?;
                valid_id(&input.goal_id)?;
                if input.name.trim().is_empty()
                    || input.name.chars().count() > 120
                    || !["count", "duration", "steps", "text"].contains(&input.template.as_str())
                {
                    return Err(ApiError::invalid("请填写指标名称并选择观察模板"));
                }
                if input.template == "duration" && input.unit.trim().is_empty() {
                    return Err(ApiError::invalid("时长或数量须注明单位"));
                }
                if let Some(old) = &previous
                    && old.input["goal_id"] != input.goal_id
                {
                    return Err(ApiError::invalid("指标不能移动到另一个目标"));
                }
                let current = super::snapshot(db)?
                    .goals
                    .into_iter()
                    .find(|g| g.id == input.goal_id)
                    .ok_or_else(ApiError::missing)?;
                if current.version != input.goal_version {
                    return Err(ApiError::conflict("目标说明已更新，请核对后定义指标"));
                }
                record.goal = Some(goal(db, &input.goal_id, input.goal_version)?);
            }
            "measurement" => {
                let input: Measurement = parse(r.input.clone())?;
                date(&input.business_date)?;
                let metric = historical(db, &input.metric_id, input.metric_version)?;
                if metric.kind != "metric" {
                    return Err(ApiError::invalid("请选择有效的测量指标"));
                }
                let definition: Metric = serde_json::from_value(metric.input.clone())?;
                match definition.template.as_str() {
                    "count" => {
                        let (Some(n), Some(k)) = (input.opportunities, input.achieved) else {
                            return Err(ApiError::invalid("请填写机会数和做到数；没有机会可填0"));
                        };
                        if k > n || input.amount.is_some() {
                            return Err(ApiError::invalid(
                                "做到数不能超过机会数，次数模板不填写时长值",
                            ));
                        }
                    }
                    "duration" => {
                        if input.amount.is_none_or(|n| !n.is_finite() || n < 0.0)
                            || input.opportunities.is_some()
                            || input.achieved.is_some()
                        {
                            return Err(ApiError::invalid("请填写非负的实际时长或数量"));
                        }
                    }
                    _ => {
                        if input.text.trim().is_empty()
                            || input.amount.is_some()
                            || input.opportunities.is_some()
                            || input.achieved.is_some()
                        {
                            return Err(ApiError::invalid(
                                "步骤和文字模板记录观察，不从文字推算数值",
                            ));
                        }
                    }
                }
                if let Some(old) = &previous {
                    let p: Measurement = serde_json::from_value(old.input.clone())?;
                    if p.metric_id != input.metric_id
                        || p.metric_version != input.metric_version
                        || p.source != input.source
                    {
                        return Err(ApiError::invalid(
                            "修订不能更换原指标或记录来源，请另建测量",
                        ));
                    }
                }
                if let Some(source) = &input.source {
                    if !["session", "lesson", "moment"].contains(&source.kind.as_str()) {
                        return Err(ApiError::invalid(
                            "请选择实际上过的课程、已完成干预或瞬间观察",
                        ));
                    }
                    let snapshot = followups::source(db, source)?;
                    let source_day = if source.kind == "lesson" {
                        snapshot["observation"]["business_date"].as_str()
                    } else {
                        snapshot["input"]["business_date"].as_str()
                    };
                    if source_day != Some(input.business_date.as_str()) {
                        return Err(ApiError::invalid("测量日期须与来源的实际日期一致"));
                    }
                    let associated = source_matches(&snapshot, &source.kind, &definition.goal_id);
                    if !associated {
                        return Err(ApiError::invalid(
                            "该来源未明确涉及此目标，请先核对关联或独立记录",
                        ));
                    }
                    unique = format!("{}:{}:{}", input.metric_id, source.kind, source.id);
                    record.source_snapshot = Some(snapshot);
                }
                record.goal = metric.goal.clone();
                record.metric = Some(Box::new(metric));
            }
            "moment" => {
                let input: Moment = parse(r.input.clone())?;
                date(&input.business_date)?;
                if input.text.trim().is_empty() {
                    return Err(ApiError::invalid("写下一句实际观察即可"));
                }
                if input.goal_ids.len() > 30 {
                    return Err(ApiError::invalid("关联目标过多"));
                }
                let goals = super::snapshot(db)?.goals;
                for id in &input.goal_ids {
                    if !goals.iter().any(|g| g.id == *id) {
                        return Err(ApiError::invalid("关联目标不存在"));
                    }
                }
            }
            "needs" => {
                let input: Needs = parse(r.input.clone())?;
                for v in [
                    &input.communication,
                    &input.sensory,
                    &input.discomfort,
                    &input.calming,
                    &input.health,
                ] {
                    text(v, 3000)?;
                }
                if r.id != NEEDS_ID {
                    return Err(ApiError::invalid("每个家庭只有一份共享需要说明"));
                }
                unique = "family".into();
            }
            _ => unreachable!(),
        }
    }
    // Preserve the original uniqueness key on deletion, including source-bound measurements.
    if r.action == "delete" {
        unique = db
            .query(
                "SELECT unique_key FROM prd_observations WHERE id=?1",
                params![r.id],
            )?
            .first()
            .ok_or_else(ApiError::missing)?[0]
            .clone();
    }
    if !db
        .query(
            "SELECT id FROM prd_observations WHERE kind=?1 AND unique_key=?2 AND id<>?3",
            params![r.kind, unique, r.id],
        )?
        .is_empty()
    {
        return Err(ApiError::conflict(
            "同一来源已有这项测量，请打开原记录修订，不重复记一次",
        ));
    }
    db.execute("INSERT INTO prd_observations(id,kind,unique_key,version,data) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET version=excluded.version,data=excluded.data",params![record.id,record.kind,unique,record.version as i64,json(&record)?])?;
    db.execute(
        "INSERT INTO prd_observation_revisions(id,version,data) VALUES(?1,?2,?3)",
        params![record.id, record.version as i64, json(&record)?],
    )?;
    let response = serde_json::to_value(record)?;
    db.execute(
        "INSERT INTO prd_operations(id,actor_id,request,response) VALUES(?1,?2,?3,?4)",
        params![r.operation_id, actor.id, encoded, json(&response)?],
    )?;
    Ok(response)
}
fn source_matches(s: &Value, kind: &str, goal: &str) -> bool {
    match kind {
        "session" => {
            s["actual_targets"].as_array().is_some_and(|v| {
                v.iter().any(|t| {
                    t["goal"]["id"] == goal
                        || t["ancestors"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|g| g["id"] == goal))
                })
            }) || s["focus_goals"]
                .as_array()
                .is_some_and(|v| v.iter().any(|g| g["id"] == goal))
        }
        "lesson" => {
            s["organization"]["targets"].as_array().is_some_and(|v| {
                v.iter().any(|t| {
                    t["goal"]["id"] == goal
                        || t["ancestors"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|g| g["id"] == goal))
                })
            }) || s["organization"]["focus_goals"]
                .as_array()
                .is_some_and(|v| v.iter().any(|g| g["id"] == goal))
        }
        "moment" => s["input"]["goal_ids"]
            .as_array()
            .is_some_and(|v| v.iter().any(|g| g == goal)),
        _ => false,
    }
}
pub fn read(db: &dyn Sql, view: &str, id: &str, kind: &str, page: u32) -> Result<Value> {
    match view {
        "observation" => Ok(serde_json::to_value(decorate(db, get(db, id)?)?)?),
        "observation_history" => {
            valid_id(id)?;
            let rows=db.query("SELECT data FROM prd_observation_revisions WHERE id=?1 ORDER BY version DESC LIMIT 31 OFFSET ?2",params![id,i64::from(page)*30])?;
            let items = rows
                .iter()
                .take(30)
                .map(|r| serde_json::from_str::<Record>(&r[0]).map_err(ApiError::from))
                .collect::<Result<Vec<_>>>()?;
            Ok(value!({"items":items,"has_more":rows.len()>30}))
        }
        "observations" => {
            if !["metric", "measurement", "moment", "needs"].contains(&kind) {
                return Err(ApiError::invalid("查看类型无效"));
            }
            let mut records = current_list(db, kind)?;
            let data_version = records.iter().map(|r| r.version).sum::<u64>();
            records.sort_by(|a, b| {
                b.input["business_date"]
                    .as_str()
                    .cmp(&a.input["business_date"].as_str())
                    .then_with(|| b.audit.at.cmp(&a.audit.at))
                    .then_with(|| a.id.cmp(&b.id))
            });
            let all = records
                .into_iter()
                .filter(|r| {
                    id.is_empty()
                        || r.goal.as_ref().is_some_and(|g| g.id == id)
                        || r.input["goal_ids"]
                            .as_array()
                            .is_some_and(|ids| ids.iter().any(|g| g == id))
                })
                .collect::<Vec<_>>();
            Ok(
                value!({"data_version":data_version,"total":all.len(),"items":all.iter().skip(page as usize*30).take(30).collect::<Vec<_>>(),"has_more":all.len()>(page as usize+1)*30}),
            )
        }
        _ => Err(ApiError::invalid("查看类型无效")),
    }
}

pub fn current_list(db: &dyn Sql, kind: &str) -> Result<Vec<Record>> {
    list(db, kind)?
        .into_iter()
        .map(|r| decorate(db, r))
        .collect()
}
fn decorate(db: &dyn Sql, mut record: Record) -> Result<Record> {
    if record.kind == "measurement"
        && let Some(source) = record.input.get("source").filter(|s| !s.is_null())
    {
        let source: super::process_model::Source = serde_json::from_value(source.clone())?;
        record.source_deleted = match source.kind.as_str() {
            "session" => super::sessions::get_session(db, &source.id)?
                .is_none_or(|s| s.status == "deleted" || s.status == "cancelled"),
            "lesson" => {
                super::lessons::get_lesson(db, &source.id)?.is_none_or(|l| l.status != "recorded")
            }
            "moment" => get(db, &source.id).map_or(true, |r| r.deleted),
            _ => true,
        };
    }
    Ok(record)
}

/// Compact current measurements for goal cards. Full records stay paginated in
/// the goal detail. Never substitute practice counts for a measured outcome.
pub fn progress(db: &dyn Sql) -> Result<Value> {
    use std::collections::BTreeMap;
    let all = current_list(db, "measurement")?;
    let data_version = all.iter().map(|r| r.version).sum::<u64>();
    let today = super::sessions::today();
    let mut goals = BTreeMap::<String, Vec<Record>>::new();
    for r in all {
        if !r.deleted
            && r.input["business_date"]
                .as_str()
                .is_some_and(|d| d <= today.as_str())
            && let Some(g) = &r.goal
        {
            goals.entry(g.id.clone()).or_default().push(r);
        }
    }
    let mut result = vec![];
    for (id, mut rows) in goals {
        rows.sort_by(|a, b| {
            b.input["business_date"]
                .as_str()
                .cmp(&a.input["business_date"].as_str())
                .then_with(|| b.audit.at.cmp(&a.audit.at))
                .then_with(|| a.id.cmp(&b.id))
        });
        let count = rows.len();
        let mut groups = BTreeMap::<String, Record>::new();
        for mut r in rows {
            let metric = r.metric.as_ref().ok_or_else(ApiError::storage)?;
            let methods = r
                .source_snapshot
                .as_ref()
                .and_then(|s| {
                    s.get("actual_methods")
                        .or_else(|| s.pointer("/organization/methods"))
                })
                .cloned()
                .unwrap_or(Value::Null);
            let confirmation = r
                .source_snapshot
                .as_ref()
                .and_then(|s| s.pointer("/input/summary/confirmation"))
                .cloned()
                .unwrap_or(Value::Null);
            let mut method_versions = methods
                .as_array()
                .map(|ms| {
                    ms.iter()
                        .map(|m| {
                            (
                                m["id"].as_str().unwrap_or("").to_string(),
                                m["version"].as_u64().unwrap_or(0),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            method_versions.sort();
            let known = if method_versions.is_empty() {
                if confirmation == "none" {
                    "none"
                } else {
                    "unknown"
                }
            } else {
                "used"
            };
            let key = json(&(
                metric.id.clone(),
                metric.version,
                r.goal.as_ref().map(|g| g.version),
                r.input["support"].as_str().unwrap_or("").trim(),
                r.input
                    .pointer("/source/kind")
                    .and_then(Value::as_str)
                    .unwrap_or("independent"),
                method_versions,
                known,
            ))?;
            r.source_snapshot = Some(
                value!({"actual_methods":methods,"input":{"summary":{"confirmation":confirmation}}}),
            );
            groups.entry(key).or_insert(r);
        }
        let segment_count = groups.len();
        let mut latest = groups.into_values().collect::<Vec<_>>();
        latest.sort_by(|a, b| {
            b.input["business_date"]
                .as_str()
                .cmp(&a.input["business_date"].as_str())
                .then_with(|| a.id.cmp(&b.id))
        });
        latest.truncate(3);
        result.push(value!({"id":id,"measurement_count":count,"segment_count":segment_count,"latest":latest}));
    }
    Ok(value!({"as_of":today,"data_version":data_version,"goals":result}))
}
