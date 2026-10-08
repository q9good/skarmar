use std::collections::HashSet;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::model::*;

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl ApiError {
    fn invalid(message: &str) -> Self {
        Self(StatusCode::UNPROCESSABLE_ENTITY, message.into())
    }
    fn conflict(message: &str) -> Self {
        Self(StatusCode::CONFLICT, message.into())
    }
    fn missing() -> Self {
        Self(StatusCode::NOT_FOUND, "记录不存在".into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error": self.1}))).into_response()
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(_: rusqlite::Error) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "数据保存失败，请保留草稿后重试".into(),
        )
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(_: serde_json::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, "数据格式异常".into())
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;

fn json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn get<T: DeserializeOwned>(db: &Connection, table: &str, id: &str) -> Result<Option<T>> {
    let value: Option<String> = db
        .query_row(
            &format!("SELECT data FROM {table} WHERE id=?1"),
            [id],
            |r| r.get(0),
        )
        .optional()?;
    value
        .map(|v| serde_json::from_str(&v).map_err(ApiError::from))
        .transpose()
}

fn list<T: DeserializeOwned>(db: &Connection, table: &str) -> Result<Vec<T>> {
    let mut statement = db.prepare(&format!("SELECT data FROM {table} ORDER BY rowid DESC"))?;
    let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
    rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
}

fn valid_id(id: &str) -> Result<()> {
    if Uuid::parse_str(id).is_err() {
        return Err(ApiError::invalid("记录 ID 格式错误"));
    }
    Ok(())
}

fn valid_date(date: &str) -> Result<()> {
    if date.len() != 10 || chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return Err(ApiError::invalid("请选择有效的训练或跟进日期"));
    }
    Ok(())
}

pub fn initialize(db: &Connection) -> Result<()> {
    db.execute_batch(include_str!("../migrations/001_initial.sql"))?;
    Ok(())
}

pub fn state(db: &Connection) -> Result<AppState> {
    Ok(AppState {
        goals: list(db, "goals")?,
        sessions: list(db, "sessions")?,
        difficulties: list(db, "difficulties")?,
        experiences: list(db, "experiences")?,
    })
}

pub fn create_goal(db: &mut Connection, input: NewGoal) -> Result<Goal> {
    valid_id(&input.id)?;
    if input.title.trim().is_empty() || input.title.len() > 300 || input.criteria.len() > 5000 {
        return Err(ApiError::invalid("请填写目标名称，并控制名称和标准长度"));
    }
    let tx = db.transaction()?;
    if get::<Goal>(&tx, "goals", &input.id)?.is_some() {
        return Err(ApiError::conflict("目标已存在，请刷新查看"));
    }
    match (&input.level, &input.parent_id) {
        (Level::Long, None) => {}
        (Level::Medium | Level::Short, Some(parent_id)) => {
            let parent: Goal = get(&tx, "goals", parent_id)?.ok_or_else(ApiError::missing)?;
            let expected = if input.level == Level::Medium {
                Level::Long
            } else {
                Level::Medium
            };
            if parent.level != expected || parent.status != GoalStatus::Active {
                return Err(ApiError::invalid("请选择活动中的正确上级目标"));
            }
        }
        _ => {
            return Err(ApiError::invalid(
                "长期目标不设父级；中期关联长期，短期关联中期",
            ));
        }
    }
    let goal = Goal {
        id: input.id,
        title: input.title.trim().into(),
        level: input.level,
        parent_id: input.parent_id,
        area: input.area,
        criteria: input.criteria,
        status: GoalStatus::Active,
        version: 1,
    };
    tx.execute(
        "INSERT INTO goals(id,parent_id,data) VALUES (?1,?2,?3)",
        params![goal.id, goal.parent_id, json(&goal)?],
    )?;
    tx.commit()?;
    Ok(goal)
}

pub fn save_session(db: &mut Connection, id: &str, request: SaveSession) -> Result<SaveResult> {
    valid_id(id)?;
    valid_id(&request.operation_id)?;
    let fingerprint = json(&(id, &request))?;
    if fingerprint.len() > 65_536 {
        return Err(ApiError::invalid("本次记录过长，请缩短内容"));
    }
    let tx = db.transaction()?;
    let previous_operation: Option<(String, String)> = tx
        .query_row(
            "SELECT request,response FROM operations WHERE id=?1",
            [&request.operation_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((original, response)) = previous_operation {
        if original != fingerprint {
            return Err(ApiError::conflict("这次提交标识已用于另一项操作"));
        }
        return Ok(serde_json::from_str(&response)?);
    }
    let previous: Option<Session> = get(&tx, "sessions", id)?;
    if previous.as_ref().map(|s| s.version) != request.expected_version {
        return Err(ApiError::conflict("这条记录已更新，请保留草稿并重新核对"));
    }
    if previous.as_ref().is_some_and(|s| s.status == "completed") {
        return Err(ApiError::conflict(
            "已完成记录暂只支持回看；修订功能将单独提供",
        ));
    }
    let complete = match request.action.as_str() {
        "save_draft" => false,
        "complete" => true,
        _ => return Err(ApiError::invalid("不支持的训练操作")),
    };
    let input = request.session;
    valid_date(&input.business_date)?;
    let mut unique = HashSet::new();
    let mut snapshots = Vec::new();
    for target in &input.targets {
        if !unique.insert(&target.goal_id) {
            return Err(ApiError::invalid("同一目标只添加一次"));
        }
        let current: Goal = get(&tx, "goals", &target.goal_id)?.ok_or_else(ApiError::missing)?;
        let snapshot = previous
            .as_ref()
            .and_then(|s| {
                s.target_snapshots
                    .iter()
                    .find(|t| t.input.goal_id == target.goal_id)
            })
            .map(|t| t.goal_snapshot.clone())
            .unwrap_or_else(|| current.clone());
        if complete {
            if snapshot.version != current.version {
                return Err(ApiError::conflict("关联目标已更新，请重新核对本次目标"));
            }
            if current.level == Level::Long
                || (target.role == Role::Review && current.status != GoalStatus::Archived)
                || (target.role != Role::Review && current.status != GoalStatus::Active)
            {
                return Err(ApiError::invalid("主副目标选当前中短期；复习选归档中短期"));
            }
            if target.outcome.is_none() {
                return Err(ApiError::invalid("请分别确认每个目标的处理结果"));
            }
            if target.outcome == Some(Outcome::Adjust) && target.next_step.trim().is_empty() {
                return Err(ApiError::invalid("调整目标时，请填写下一步说明"));
            }
        }
        snapshots.push(Target {
            input: target.clone(),
            goal_snapshot: snapshot,
        });
    }
    // The SQL index also enforces primary uniqueness for saved drafts.
    if input
        .targets
        .iter()
        .filter(|t| t.role == Role::Primary)
        .count()
        > 1
    {
        return Err(ApiError::invalid("一次训练只能有一个主目标"));
    }
    if complete {
        if input
            .targets
            .iter()
            .filter(|t| t.role == Role::Primary)
            .count()
            != 1
            || !input.targets.iter().any(|t| t.role == Role::Secondary)
        {
            return Err(ApiError::invalid("完成训练需要一个主目标和至少一个副目标"));
        }
        if input.activities.is_empty() || input.organization.trim().is_empty() {
            return Err(ApiError::invalid("请选择训练活动和组织方式"));
        }
        if input.has_difficulty && input.difficulty.trim().is_empty() {
            return Err(ApiError::invalid("遇到困难时，请填写困难详情"));
        }
        if input.has_experience
            && (input.experience.trim().is_empty()
                || !["general", "task"].contains(&input.experience_kind.as_str()))
        {
            return Err(ApiError::invalid("有成功经验时，请填写详情并选择经验类型"));
        }
    }
    let session = Session {
        id: id.into(),
        version: request.expected_version.unwrap_or(0) + 1,
        status: if complete { "completed" } else { "draft" }.into(),
        input,
        target_snapshots: snapshots,
    };
    tx.execute("INSERT INTO sessions(id,data) VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        params![id, json(&session)?])?;
    tx.execute("DELETE FROM session_targets WHERE session_id=?1", [id])?;
    for target in &session.input.targets {
        tx.execute(
            "INSERT INTO session_targets(session_id,goal_id,role) VALUES (?1,?2,?3)",
            params![id, target.goal_id, target.role.as_str()],
        )?;
        if complete && target.outcome == Some(Outcome::Archive) {
            let mut goal: Goal =
                get(&tx, "goals", &target.goal_id)?.ok_or_else(ApiError::missing)?;
            if goal.status != GoalStatus::Archived {
                goal.status = GoalStatus::Archived;
                goal.version += 1;
                tx.execute(
                    "UPDATE goals SET data=?1 WHERE id=?2",
                    params![json(&goal)?, goal.id],
                )?;
            }
        }
    }
    let mut result = SaveResult {
        session,
        difficulty_id: None,
        experience_id: None,
    };
    if complete && result.session.input.has_difficulty {
        let item = FollowUp {
            id: Uuid::new_v4().to_string(),
            version: 1,
            source_session_id: id.into(),
            content: result.session.input.difficulty.trim().into(),
            kind: None,
            status: "pending".into(),
            next_review_date: None,
            conclusion: String::new(),
        };
        tx.execute(
            "INSERT INTO difficulties(id,source_session_id,data) VALUES (?1,?2,?3)",
            params![item.id, id, json(&item)?],
        )?;
        result.difficulty_id = Some(item.id);
    }
    if complete && result.session.input.has_experience {
        let item = FollowUp {
            id: Uuid::new_v4().to_string(),
            version: 1,
            source_session_id: id.into(),
            content: result.session.input.experience.trim().into(),
            kind: Some(result.session.input.experience_kind.clone()),
            status: "pending".into(),
            next_review_date: None,
            conclusion: String::new(),
        };
        tx.execute(
            "INSERT INTO experiences(id,source_session_id,data) VALUES (?1,?2,?3)",
            params![item.id, id, json(&item)?],
        )?;
        result.experience_id = Some(item.id);
    }
    tx.execute(
        "INSERT INTO operations(id,request,response) VALUES (?1,?2,?3)",
        params![request.operation_id, fingerprint, json(&result)?],
    )?;
    tx.commit()?;
    Ok(result)
}

pub fn update_follow_up(
    db: &mut Connection,
    table: &str,
    id: &str,
    request: UpdateFollowUp,
) -> Result<FollowUp> {
    let allowed = if table == "difficulties" {
        ["pending", "active", "resolved"]
    } else {
        ["pending", "verified", "stopped"]
    };
    if !allowed.contains(&request.status.as_str()) {
        return Err(ApiError::invalid("跟进状态无效"));
    }
    if ["resolved", "verified"].contains(&request.status.as_str())
        && request.conclusion.trim().is_empty()
    {
        return Err(ApiError::invalid("请填写解决结论或验证依据"));
    }
    if let Some(date) = &request.next_review_date {
        valid_date(date)?;
    }
    let tx = db.transaction()?;
    let mut item: FollowUp = get(&tx, table, id)?.ok_or_else(ApiError::missing)?;
    if item.version != request.expected_version {
        return Err(ApiError::conflict("跟进记录已更新，请重新核对"));
    }
    item.version += 1;
    item.status = request.status;
    item.next_review_date = request.next_review_date;
    item.conclusion = request.conclusion;
    tx.execute(
        &format!("UPDATE {table} SET data=?1 WHERE id=?2"),
        params![json(&item)?, id],
    )?;
    tx.commit()?;
    Ok(item)
}

pub fn seed_demo(db: &mut Connection) -> Result<()> {
    if !list::<Goal>(db, "goals")?.is_empty() {
        return Ok(());
    }
    let long_id = Uuid::new_v4().to_string();
    let medium_id = Uuid::new_v4().to_string();
    let items = [
        (long_id.clone(), "示例：沟通与日常生活", Level::Long, None),
        (
            medium_id.clone(),
            "示例：在生活活动中表达需要",
            Level::Medium,
            Some(long_id),
        ),
        (
            Uuid::new_v4().to_string(),
            "主动请求帮助",
            Level::Short,
            Some(medium_id.clone()),
        ),
        (
            Uuid::new_v4().to_string(),
            "轮流参与活动",
            Level::Short,
            Some(medium_id.clone()),
        ),
        (
            Uuid::new_v4().to_string(),
            "复习：整理书包",
            Level::Short,
            Some(medium_id),
        ),
    ];
    for (id, title, level, parent_id) in items {
        let goal = create_goal(
            db,
            NewGoal {
                id,
                title: title.into(),
                level,
                parent_id,
                area: "示例领域".into(),
                criteria: "示例内容，可按自己的观察规则新建目标".into(),
            },
        )?;
        if title.starts_with("复习") {
            let archived = Goal {
                status: GoalStatus::Archived,
                ..goal
            };
            db.execute(
                "UPDATE goals SET data=?1 WHERE id=?2",
                params![json(&archived)?, archived.id],
            )?;
        }
    }
    Ok(())
}
