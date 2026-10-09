use std::collections::HashSet;

use crate::sql::{Sql, params};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::model::*;

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.1)
    }
}
impl std::error::Error for ApiError {}

impl ApiError {
    pub fn storage() -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "数据保存失败，请保留草稿后重试".into(),
        )
    }
    pub fn invalid(message: &str) -> Self {
        Self(StatusCode::UNPROCESSABLE_ENTITY, message.into())
    }
    pub fn conflict(message: &str) -> Self {
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

impl From<serde_json::Error> for ApiError {
    fn from(_: serde_json::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, "数据格式异常".into())
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;

fn json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn get<T: DeserializeOwned>(db: &dyn Sql, table: &str, id: &str) -> Result<Option<T>> {
    db.query(
        &format!("SELECT data FROM {table} WHERE id=?1"),
        params![id],
    )?
    .into_iter()
    .next()
    .map(|row| Ok(serde_json::from_str(&row[0])?))
    .transpose()
}

fn list<T: DeserializeOwned>(db: &dyn Sql, table: &str) -> Result<Vec<T>> {
    db.query(
        &format!("SELECT data FROM {table} ORDER BY rowid DESC"),
        &[],
    )?
    .into_iter()
    .map(|row| Ok(serde_json::from_str(&row[0])?))
    .collect()
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

pub fn initialize(db: &dyn Sql) -> Result<()> {
    const MIGRATIONS: &[(i64, &str)] = &[
        (1, include_str!("../migrations/001_initial.sql")),
        (2, include_str!("../migrations/002_goal_revisions.sql")),
    ];
    db.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY);")?;
    let applied = db
        .query(
            "SELECT CAST(version AS TEXT) FROM schema_migrations ORDER BY version",
            &[],
        )?
        .into_iter()
        .map(|row| row[0].parse::<i64>().map_err(|_| ApiError::storage()))
        .collect::<Result<Vec<_>>>()?;
    let latest = MIGRATIONS.last().map(|(version, _)| *version).unwrap_or(0);
    if applied.iter().any(|version| *version > latest) {
        return Err(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "数据库版本高于程序支持版本，请使用匹配的程序".into(),
        ));
    }
    for (version, sql) in MIGRATIONS {
        if !applied.contains(version) {
            db.execute_batch(sql)?;
            db.execute(
                "INSERT INTO schema_migrations(version) VALUES (?1)",
                params![*version],
            )?;
        }
    }
    Ok(())
}

fn record_goal_revision(db: &dyn Sql, goal: &Goal, note: &str, source: Option<&str>) -> Result<()> {
    let revision = GoalRevision {
        goal: goal.clone(),
        recorded_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
        change_note: note.into(),
        source_session_id: source.map(str::to_owned),
    };
    db.execute(
        "INSERT OR IGNORE INTO goal_revisions(goal_id,version,data) VALUES (?1,?2,?3)",
        params![
            goal.id,
            i64::try_from(goal.version).map_err(|_| ApiError::invalid("目标版本超出范围"))?,
            json(&revision)?
        ],
    )?;
    Ok(())
}

pub fn goal_history(db: &dyn Sql, id: &str) -> Result<Vec<GoalRevision>> {
    if get::<Goal>(db, "goals", id)?.is_none() {
        return Err(ApiError::missing());
    }
    db.query(
        "SELECT data FROM goal_revisions WHERE goal_id=?1 ORDER BY version DESC",
        params![id],
    )?
    .into_iter()
    .map(|row| Ok(serde_json::from_str(&row[0])?))
    .collect()
}

pub fn update_goal(db: &dyn Sql, id: &str, request: UpdateGoal) -> Result<Goal> {
    let previous: Goal = get(db, "goals", id)?.ok_or_else(ApiError::missing)?;
    if previous.version != request.expected_version {
        return Err(ApiError::conflict(
            "目标已被更新，请保留填写内容并核对最新版本",
        ));
    }
    if request.title.trim().is_empty()
        || request.title.len() > 300
        || request.change_note.trim().is_empty()
        || request.criteria.len() > 5000
        || request.review_notes.len() > 10_000
    {
        return Err(ApiError::invalid(
            "请填写目标名称与本次修改说明，并控制内容长度",
        ));
    }
    match (&previous.level, &request.progress_status) {
        (Level::Long, None | Some(GoalProgress::Cultivating | GoalProgress::Internalized)) => {}
        (
            Level::Medium | Level::Short,
            None
            | Some(
                GoalProgress::NotStarted
                | GoalProgress::InProgress
                | GoalProgress::Achieved
                | GoalProgress::Completed
                | GoalProgress::Overdue,
            ),
        ) => {}
        _ => return Err(ApiError::invalid("培养状态与目标层级不匹配")),
    }
    for date in [
        &request.start_date,
        &request.due_date,
        &request.last_review_date,
        &request.next_review_date,
        &request.completion_date,
    ]
    .into_iter()
    .flatten()
    {
        valid_date(date)?;
    }
    if previous.level == Level::Long
        && (request.due_date.is_some() || request.status != GoalStatus::Active)
    {
        return Err(ApiError::invalid(
            "长期目标持续保留，不设置截止日期或归档；内化状态单独记录",
        ));
    }
    if let (Some(start), Some(due)) = (&request.start_date, &request.due_date)
        && due < start
    {
        return Err(ApiError::invalid("截止日期不能早于开始日期"));
    }
    match (&previous.level, &request.parent_id) {
        (Level::Long, None) => {}
        (Level::Medium | Level::Short, Some(parent_id)) => {
            let parent: Goal = get(db, "goals", parent_id)?.ok_or_else(ApiError::missing)?;
            let expected = if previous.level == Level::Medium {
                Level::Long
            } else {
                Level::Medium
            };
            if parent.level != expected
                || (parent.status != GoalStatus::Active
                    && previous.parent_id.as_ref() != Some(parent_id))
            {
                return Err(ApiError::invalid("请选择活动中的正确上级目标"));
            }
        }
        (Level::Medium | Level::Short, None) if previous.parent_id.is_none() => {}
        _ => {
            return Err(ApiError::invalid(
                "长期目标不设父级；中期关联长期，短期关联中期",
            ));
        }
    }
    record_goal_revision(db, &previous, "旧版本基线", None)?;
    let updated = Goal {
        title: request.title.trim().into(),
        parent_id: request.parent_id,
        area: request.area,
        criteria: request.criteria,
        status: request.status,
        progress_status: request.progress_status,
        start_date: request.start_date,
        due_date: request.due_date,
        last_review_date: request.last_review_date,
        next_review_date: request.next_review_date,
        completion_date: request.completion_date,
        review_notes: request.review_notes,
        version: previous.version + 1,
        ..previous
    };
    db.execute(
        "UPDATE goals SET parent_id=?1,data=?2 WHERE id=?3",
        params![updated.parent_id, json(&updated)?, id],
    )?;
    record_goal_revision(db, &updated, &request.change_note, None)?;
    Ok(updated)
}

pub fn state(db: &dyn Sql) -> Result<AppState> {
    Ok(AppState {
        goals: list(db, "goals")?,
        sessions: list(db, "sessions")?,
        difficulties: list(db, "difficulties")?,
        experiences: list(db, "experiences")?,
    })
}

pub fn create_goal(db: &dyn Sql, input: NewGoal) -> Result<Goal> {
    valid_id(&input.id)?;
    if input.title.trim().is_empty() || input.title.len() > 300 || input.criteria.len() > 5000 {
        return Err(ApiError::invalid("请填写目标名称，并控制名称和标准长度"));
    }
    if get::<Goal>(db, "goals", &input.id)?.is_some() {
        return Err(ApiError::conflict("目标已存在，请刷新查看"));
    }
    match (&input.level, &input.parent_id) {
        (Level::Long, None) => {}
        (Level::Medium | Level::Short, Some(parent_id)) => {
            let parent: Goal = get(db, "goals", parent_id)?.ok_or_else(ApiError::missing)?;
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
    let progress = if input.level == Level::Long {
        GoalProgress::Cultivating
    } else {
        GoalProgress::NotStarted
    };
    let goal = Goal {
        id: input.id,
        title: input.title.trim().into(),
        level: input.level,
        parent_id: input.parent_id,
        area: input.area,
        criteria: input.criteria,
        status: GoalStatus::Active,
        version: 1,
        progress_status: Some(progress),
        start_date: None,
        due_date: None,
        last_review_date: None,
        next_review_date: None,
        completion_date: None,
        review_notes: String::new(),
    };
    db.execute(
        "INSERT INTO goals(id,parent_id,data) VALUES (?1,?2,?3)",
        params![goal.id, goal.parent_id, json(&goal)?],
    )?;
    record_goal_revision(db, &goal, "创建目标", None)?;
    Ok(goal)
}

pub fn save_session(db: &dyn Sql, id: &str, request: SaveSession) -> Result<SaveResult> {
    valid_id(id)?;
    valid_id(&request.operation_id)?;
    let fingerprint = json(&(id, &request))?;
    if fingerprint.len() > 65_536 {
        return Err(ApiError::invalid("本次记录过长，请缩短内容"));
    }
    let previous_operation = db
        .query(
            "SELECT request,response FROM operations WHERE id=?1",
            params![request.operation_id],
        )?
        .into_iter()
        .next()
        .map(|row| (row[0].clone(), row[1].clone()));
    if let Some((original, response)) = previous_operation {
        if original != fingerprint {
            return Err(ApiError::conflict("这次提交标识已用于另一项操作"));
        }
        return Ok(serde_json::from_str(&response)?);
    }
    let previous: Option<Session> = get(db, "sessions", id)?;
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
        let current: Goal = get(db, "goals", &target.goal_id)?.ok_or_else(ApiError::missing)?;
        let planned_snapshot = previous
            .as_ref()
            .and_then(|s| {
                s.target_snapshots
                    .iter()
                    .find(|t| t.input.goal_id == target.goal_id)
            })
            .map(|t| t.goal_snapshot.clone())
            .unwrap_or_else(|| current.clone());
        // Explicitly saving a plan acknowledges the latest goal definition.
        // Completion alone must not silently replace the definition used before training.
        let snapshot = if complete {
            planned_snapshot
        } else {
            current.clone()
        };
        if complete {
            if snapshot.version != current.version {
                return Err(ApiError::conflict("关联目标已更新，请重新核对本次目标"));
            }
            if current.level == Level::Long
                || (target.role == Role::Review && !current.is_reviewable())
                || (target.role != Role::Review && !current.is_trainable())
            {
                return Err(ApiError::invalid(
                    "主副目标选待训练中短期；复习选已达成、已完成或已归档中短期",
                ));
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
    db.execute("INSERT INTO sessions(id,data) VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        params![id, json(&session)?])?;
    db.execute(
        "DELETE FROM session_targets WHERE session_id=?1",
        params![id],
    )?;
    for target in &session.input.targets {
        db.execute(
            "INSERT INTO session_targets(session_id,goal_id,role) VALUES (?1,?2,?3)",
            params![id, target.goal_id, target.role.as_str()],
        )?;
        if complete && target.outcome == Some(Outcome::Archive) {
            let mut goal: Goal =
                get(db, "goals", &target.goal_id)?.ok_or_else(ApiError::missing)?;
            if goal.status != GoalStatus::Archived {
                record_goal_revision(db, &goal, "旧版本基线", None)?;
                goal.status = GoalStatus::Archived;
                if !matches!(
                    goal.progress_status,
                    Some(GoalProgress::Achieved | GoalProgress::Completed)
                ) {
                    goal.progress_status = Some(GoalProgress::Achieved);
                }
                goal.completion_date
                    .get_or_insert_with(|| session.input.business_date.clone());
                goal.version += 1;
                db.execute(
                    "UPDATE goals SET data=?1 WHERE id=?2",
                    params![json(&goal)?, goal.id],
                )?;
                record_goal_revision(db, &goal, "训练后确认达成并归档", Some(id))?;
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
        db.execute(
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
        db.execute(
            "INSERT INTO experiences(id,source_session_id,data) VALUES (?1,?2,?3)",
            params![item.id, id, json(&item)?],
        )?;
        result.experience_id = Some(item.id);
    }
    db.execute(
        "INSERT INTO operations(id,request,response) VALUES (?1,?2,?3)",
        params![request.operation_id, fingerprint, json(&result)?],
    )?;
    Ok(result)
}

pub fn update_follow_up(
    db: &dyn Sql,
    table: &str,
    id: &str,
    request: UpdateFollowUp,
) -> Result<FollowUp> {
    if !["difficulties", "experiences"].contains(&table) {
        return Err(ApiError::invalid("跟进类型无效"));
    }
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
    let mut item: FollowUp = get(db, table, id)?.ok_or_else(ApiError::missing)?;
    if item.version != request.expected_version {
        return Err(ApiError::conflict("跟进记录已更新，请重新核对"));
    }
    item.version += 1;
    item.status = request.status;
    item.next_review_date = request.next_review_date;
    item.conclusion = request.conclusion;
    db.execute(
        &format!("UPDATE {table} SET data=?1 WHERE id=?2"),
        params![json(&item)?, id],
    )?;
    Ok(item)
}

pub fn seed_demo(db: &dyn Sql) -> Result<()> {
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
            update_goal(
                db,
                &goal.id,
                UpdateGoal {
                    expected_version: goal.version,
                    title: goal.title,
                    parent_id: goal.parent_id,
                    area: goal.area,
                    criteria: goal.criteria,
                    status: GoalStatus::Archived,
                    progress_status: Some(GoalProgress::Achieved),
                    start_date: None,
                    due_date: None,
                    last_review_date: None,
                    next_review_date: None,
                    completion_date: None,
                    review_notes: String::new(),
                    change_note: "示例归档".into(),
                },
            )?;
        }
    }
    Ok(())
}
