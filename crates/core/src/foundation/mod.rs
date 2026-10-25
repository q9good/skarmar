//! PRD model foundation, intentionally separate from the legacy schema initializer.
//! Adapters use an explicitly selected fresh database or a verified conversion
//! marker. Ordinary startup never performs legacy conversion.
mod comparison;
pub mod coverage;
pub mod export;
pub mod followups;
pub mod lesson_model;
pub mod lessons;
pub mod model;
pub mod observation_model;
pub mod observations;
pub mod process_model;
mod review_changes;
pub mod reviews;
pub mod session_model;
pub mod sessions;
pub mod transfer;
use crate::{
    auth::User,
    model::Level,
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_date, valid_id},
};
use model::*;
use std::collections::HashSet;

/// Refuse old or unknown databases rather than treating a new schema as a migration.
pub fn initialize(db: &dyn Sql) -> Result<()> {
    let mut names = db.query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name<>'_cf_KV'",
        &[],
    )?;
    transfer::filter_retained_legacy(db, &mut names)?;
    let expected = [
        "prd_schema",
        "prd_entities",
        "prd_operations",
        "prd_revisions",
        "prd_method_goals",
    ];
    if names.is_empty() {
        db.execute_batch(include_str!("schema.sql"))?;
        return initialize(db);
    }
    let auth_names = [
        "auth_users",
        "auth_sessions",
        "auth_login_attempts",
        "auth_registration_requests",
        "auth_registration_limits",
    ];
    let domain: Vec<_> = names
        .iter()
        .filter(|row| !auth_names.contains(&row[0].as_str()))
        .collect();
    let auth_count = names.len() - domain.len();
    let extra2 = [
        "prd_lessons",
        "prd_lesson_revisions",
        "prd_knowledge",
        "prd_actions",
        "prd_lesson_refs",
    ];
    let extra3 = [
        "prd_sessions",
        "prd_session_revisions",
        "prd_output_revisions",
    ];
    let extra4 = [
        "prd_settings",
        "prd_schedules",
        "prd_schedule_revisions",
        "prd_feedback",
        "prd_feedback_revisions",
        "prd_annotations",
        "prd_reviews",
        "prd_review_revisions",
    ];
    let extra5 = ["prd_observations", "prd_observation_revisions"];
    let extra6 = [
        "prd_import_previews",
        "prd_import_rows",
        "prd_import_batches",
        "prd_upgrade",
    ];
    if expected
        .iter()
        .any(|name| !domain.iter().any(|row| row[0] == *name))
    {
        return Err(ApiError::conflict(
            "此数据库不是新模型空库；请先制定数据转换方案",
        ));
    }
    let versions = db.query("SELECT CAST(version AS TEXT) FROM prd_schema", &[])?;
    let version = match versions.as_slice() {
        [row] if row.len() == 1 => row[0].parse::<u32>().unwrap_or(0),
        _ => 0,
    };
    if !(1..=7).contains(&version) {
        return Err(ApiError::conflict("新模型版本不兼容，请核对转换方案"));
    }
    let mut allowed = expected.to_vec();
    if version >= 2 {
        allowed.extend(extra2);
    }
    if version >= 3 {
        allowed.extend(extra3);
    }
    if version >= 4 {
        allowed.extend(extra4);
    }
    if version >= 5 {
        allowed.extend(extra5);
    }
    if version >= 6 {
        allowed.extend(extra6);
    }
    if domain.len() != allowed.len()
        || domain.iter().any(|row| !allowed.contains(&row[0].as_str()))
        || (auth_count != 0 && auth_count != auth_names.len())
    {
        return Err(ApiError::conflict(
            "此数据库不是新模型空库；请先制定数据转换方案",
        ));
    }
    match version {
        1 => db.execute_batch(include_str!("lessons_v2.sql"))?,
        2 => db.execute_batch(include_str!("sessions_v3.sql"))?,
        3 => {
            db.execute_batch(include_str!("followups_v4.sql"))?;
            reviews::seed(db)?;
        }
        4 => db.execute_batch(include_str!("observations_v5.sql"))?,
        5 => db.execute_batch(include_str!("transfer_v6.sql"))?,
        6 => db.execute_batch(include_str!("catalog_v7.sql"))?,
        _ => {}
    }
    if version < 7 {
        return initialize(db);
    }

    Ok(())
}
fn get(db: &dyn Sql, id: &str) -> Result<Option<Saved>> {
    db.query("SELECT data FROM prd_entities WHERE id=?1", params![id])?
        .first()
        .map(|row| serde_json::from_str(&row[0]).map_err(ApiError::from))
        .transpose()
}
fn title(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        return Err(ApiError::invalid("名称须为1至120个字符"));
    }
    Ok(value.into())
}
fn parent(db: &dyn Sql, id: &str, input: &GoalInput) -> Result<()> {
    let Some(parent_id) = &input.parent_id else {
        return Ok(());
    };
    valid_id(parent_id)?;
    if parent_id == id {
        return Err(ApiError::invalid("目标不能关联自己为上级"));
    }
    let Some(Saved::Goal(parent)) = get(db, parent_id)? else {
        return Err(ApiError::invalid("上级目标不存在"));
    };
    let rank = |level: &Level| match level {
        Level::Long => 0,
        Level::Medium => 1,
        Level::Short => 2,
    };
    if rank(&parent.input.level) >= rank(&input.level) {
        return Err(ApiError::invalid("上级目标须为更长期的方向"));
    }
    Ok(())
}
/// Internal entry: actor must come from the transport's authenticated session,
/// never from fields supplied by the client. The entire call must be atomic.
fn merge_update(request: &Write, previous: Option<&Saved>) -> Result<Mutation> {
    let Some(fields) = &request.provided_fields else {
        return Ok(request.mutation.clone());
    };
    let mut prior = match (previous, &request.mutation) {
        (Some(Saved::Goal(g)), Mutation::Goal(_)) => serde_json::to_value(&g.input)?,
        (Some(Saved::Method(m)), Mutation::Method(_)) => serde_json::to_value(&m.input)?,
        _ => return Ok(request.mutation.clone()),
    };
    let incoming = serde_json::to_value(&request.mutation)?;
    for field in fields {
        prior[field] = incoming["value"]
            .get(field)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
    }
    Ok(serde_json::from_value(
        serde_json::json!({"kind":incoming["kind"],"value":prior}),
    )?)
}

pub fn write(db: &dyn Sql, actor: &User, request: Write) -> Result<Saved> {
    valid_id(&actor.id)?;
    valid_id(&request.operation_id)?;
    valid_id(&request.id)?;
    if !actor.active {
        return Err(crate::auth::unauthorized());
    }
    let legacy_encoded = request.legacy_json()?;
    let fields = request.provided_fields.clone().unwrap_or_else(|| {
        serde_json::to_value(&request.mutation)
            .ok()
            .and_then(|v| v["value"].as_object().map(|v| v.keys().cloned().collect()))
            .unwrap_or_default()
    });
    // Presence changes update semantics, so it is part of new operation identity.
    // Existing pre-mask operations still replay their original confirmed receipt.
    let encoded = format!("fields:v2:{}:{}", json(&fields)?, legacy_encoded);
    if let Some(row) = db
        .query(
            "SELECT actor_id,request,response FROM prd_operations WHERE id=?1",
            params![request.operation_id],
        )?
        .first()
    {
        if row[0] != actor.id || (row[1] != encoded && row[1] != legacy_encoded) {
            return Err(ApiError::conflict(
                "本次操作编号已用于其他内容，请核对后重新提交",
            ));
        }
        return Ok(serde_json::from_str(&row[2])?);
    }
    let previous = get(db, &request.id)?;
    if request
        .method_change
        .as_ref()
        .is_some_and(|v| !["adjustment", "correction"].contains(&v.as_str()))
        || request.method_change.is_some() && !matches!(request.mutation, Mutation::Method(_))
    {
        return Err(ApiError::invalid("请明确是调整实际做法还是更正文案"));
    }
    if request.method_create_mode.is_some() && !matches!(request.mutation, Mutation::Method(_)) {
        return Err(ApiError::invalid("创建模式只适用于方法"));
    }
    match (&previous, request.expected_version) {
        (None, None) => {}
        (Some(old), Some(expected)) if old.version() == expected => {}
        _ => return Err(ApiError::conflict("记录已有新版本，请保留填写并核对")),
    }
    let version = previous
        .as_ref()
        .map_or(Some(1), |old| old.version().checked_add(1))
        .ok_or_else(|| ApiError::conflict("记录版本超出支持范围"))?;
    if version > i64::MAX as u64 {
        return Err(ApiError::conflict("记录版本超出支持范围"));
    }
    let mut event = if previous.is_some() {
        "edited"
    } else {
        "created"
    };
    let (kind, name, saved) = match merge_update(&request, previous.as_ref())? {
        Mutation::Goal(mut input) => {
            input.title = title(&input.title)?;
            if input.state.is_none() {
                input.state = match &previous {
                    Some(Saved::Goal(old)) => old.input.state.clone(),
                    _ => Some(GoalState::Active),
                };
            }
            if input.level == Level::Long
                && matches!(
                    input.state,
                    Some(GoalState::Achieved | GoalState::NotStarted)
                )
            {
                return Err(ApiError::invalid("长期方向不使用未开始或已达成状态"));
            }
            if let Some(date) = &input.attention_start {
                valid_date(date)?;
            }
            if let Some(date) = &input.due {
                valid_date(date)?;
            }
            if let Some(selected) = input.domain.clone() {
                if selected.kind != CatalogKind::Domain {
                    return Err(ApiError::invalid("请选择正确的领域选项"));
                }
                let unchanged = matches!(
                    &previous,
                    Some(Saved::Goal(old)) if old.input.domain.as_ref() == Some(&selected)
                );
                if !unchanged {
                    input.domain = Some(catalog_snapshot(db, &selected.id, CatalogKind::Domain)?);
                }
            }
            if let Some(date) = &input.resume_on {
                valid_date(date)?;
            }
            if input.state != Some(GoalState::Paused) {
                input.resume_on = None;
            }
            parent(db, &request.id, &input)?;
            match &previous {
                Some(Saved::Goal(old)) => {
                    if old.input.level != input.level {
                        return Err(ApiError::invalid("目标期限类型不可直接改变"));
                    }
                    if old.input.state != input.state || old.input.archived != input.archived {
                        event = "state_changed";
                        if request.change_note.trim().is_empty() {
                            return Err(ApiError::invalid("改变状态时请说明原因"));
                        }
                    }
                }
                Some(_) => return Err(ApiError::conflict("记录编号属于其他对象")),
                None => {}
            }
            (
                "goal",
                input.title.clone(),
                Saved::Goal(Goal {
                    id: request.id.clone(),
                    version,
                    input,
                }),
            )
        }
        Mutation::Method(mut input) => {
            if let Some(date) = &input.resume_on {
                valid_date(date)?;
            }
            if input.state != Some(MethodState::Paused) {
                input.resume_on = None;
            }
            input.title = title(&input.title)?;
            if input.state.is_none() {
                input.state = match &previous {
                    Some(Saved::Method(old)) => old.input.state.clone(),
                    _ => Some(MethodState::Active),
                };
            }
            if previous.is_some() && request.method_create_mode.is_some() {
                return Err(ApiError::invalid("创建模式不能用于修改已有方法"));
            }
            if previous.is_none() {
                match request.method_create_mode.as_ref() {
                    Some(MethodCreateMode::Full) if input.steps.trim().is_empty() => {
                        return Err(ApiError::invalid("普通新建方法须填写具体做法"));
                    }
                    Some(MethodCreateMode::Full | MethodCreateMode::Quick) => {}
                    None if request.provided_fields.is_some() => {
                        return Err(ApiError::invalid(
                            "新建方法须明确选择普通创建或上下文快捷创建",
                        ));
                    }
                    None => {} // Trusted, fully specified Rust conversion/import command.
                }
            }
            if input.goal_ids.is_empty() || input.goal_ids.len() > 100 {
                return Err(ApiError::invalid("请选择1至100个明确关联的目标"));
            }
            let mut ids = HashSet::new();
            for id in &input.goal_ids {
                valid_id(id)?;
                if !ids.insert(id) {
                    return Err(ApiError::invalid("关联目标不能重复"));
                }
                if !matches!(get(db, id)?, Some(Saved::Goal(_))) {
                    return Err(ApiError::invalid("关联目标不存在"));
                }
            }
            if input.needs_attention && input.attention_reason.trim().is_empty() {
                return Err(ApiError::invalid("请说明方法需要注意的原因"));
            }
            match &previous {
                Some(Saved::Method(old)) if old.input.state != input.state => {
                    event = "state_changed";
                    if request.change_note.trim().is_empty() {
                        return Err(ApiError::invalid("改变状态时请说明原因"));
                    }
                }
                Some(Saved::Method(_)) | None => {}
                Some(_) => return Err(ApiError::conflict("记录编号属于其他对象")),
            }
            if let Some(Saved::Method(old)) = &previous
                && old.input.basis != input.basis
                && request.change_note.trim().is_empty()
            {
                return Err(ApiError::invalid("调整依据改变时，请说明准备怎样调整"));
            }
            if input.basis.len() > 20 {
                return Err(ApiError::invalid("调整依据过多，请精简"));
            }
            if let Some(Saved::Method(old)) = &previous
                && (request.method_change.as_deref() == Some("adjustment")
                    || request.method_change.is_none()
                        && (old.input.description != input.description
                            || old.input.steps != input.steps
                            || old.input.scope != input.scope))
            {
                event = "practice_adjusted";
            }
            let mut basis_snapshots = vec![];
            let mut seen = HashSet::new();
            for basis in &input.basis {
                valid_id(&basis.id)?;
                if basis.version == 0 || !seen.insert((basis.id.clone(), basis.version)) {
                    return Err(ApiError::invalid("依据编号或版本不正确"));
                }
                if let Some(Saved::Method(old)) = &previous
                    && let Some(snapshot) = old
                        .basis_snapshots
                        .iter()
                        .find(|s| s.id == basis.id && s.version == basis.version)
                {
                    basis_snapshots.push(snapshot.clone());
                    continue;
                }
                let current = followups::get(db, &basis.id)?;
                if current.deleted {
                    return Err(ApiError::invalid("不能新增已删除的知识作为调整依据"));
                }
                let rows=db.query("SELECT data FROM prd_output_revisions WHERE kind='knowledge' AND id=?1 AND version=?2",params![basis.id,basis.version as i64])?;
                let source: lesson_model::Knowledge =
                    serde_json::from_str(&rows.first().ok_or_else(ApiError::missing)?[0])?;
                basis_snapshots.push(MethodBasisSnapshot {
                    id: basis.id.clone(),
                    version: basis.version,
                    content: source.input.content,
                    conditions: source.details.conditions,
                    kind: match source.input.kind {
                        lesson_model::KnowledgeKind::Experience => "experience",
                        lesson_model::KnowledgeKind::Difficulty => "difficulty",
                    }
                    .into(),
                });
            }
            (
                "method",
                input.title.clone(),
                Saved::Method(Method {
                    basis_snapshots,
                    id: request.id.clone(),
                    version,
                    input,
                }),
            )
        }
        Mutation::Catalog(mut input) => {
            input.name = title(&input.name)?;
            let kind = match input.kind {
                CatalogKind::Course => "course",
                CatalogKind::Teacher => "teacher",
                CatalogKind::Domain => "domain",
            };
            match &previous {
                Some(Saved::Catalog(old)) if old.input.kind == input.kind => {}
                Some(_) => return Err(ApiError::conflict("记录编号属于其他目录或对象")),
                None => {}
            }
            let duplicate = db.query(
                "SELECT id FROM prd_entities WHERE kind=?1 AND name=?2 AND id<>?3",
                params![kind, input.name, request.id],
            )?;
            if !duplicate.is_empty() {
                return Err(ApiError::conflict("同名选项已存在，请选择或恢复已有项"));
            }
            if let Some(Saved::Catalog(old)) = &previous
                && old.input.active != input.active
            {
                event = "state_changed";
            }
            (
                kind,
                input.name.clone(),
                Saved::Catalog(CatalogItem {
                    id: request.id.clone(),
                    version,
                    input,
                }),
            )
        }
    };
    let response = json(&saved)?;
    let revision = Revision {
        entity: saved.clone(),
        operation_id: request.operation_id.clone(),
        actor_id: actor.id.clone(),
        recorded_at: chrono::Utc::now().to_rfc3339(),
        event: event.into(),
        change_note: request.change_note,
    };
    db.execute("INSERT INTO prd_entities(id,kind,version,name,data) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET version=excluded.version,name=excluded.name,data=excluded.data",
        params![request.id, kind, version as i64, name, response])?;
    if let Saved::Method(method) = &saved {
        db.execute(
            "DELETE FROM prd_method_goals WHERE method_id=?1",
            params![request.id],
        )?;
        for id in &method.input.goal_ids {
            db.execute(
                "INSERT INTO prd_method_goals(method_id,goal_id) VALUES(?1,?2)",
                params![request.id, id.as_str()],
            )?;
        }
    }
    db.execute(
        "INSERT INTO prd_operations(id,actor_id,request,response) VALUES(?1,?2,?3,?4)",
        params![request.operation_id, actor.id, encoded, response],
    )?;
    db.execute(
        "INSERT INTO prd_revisions(entity_id,version,operation_id,data) VALUES(?1,?2,?3,?4)",
        params![
            request.id,
            version as i64,
            request.operation_id,
            json(&revision)?
        ],
    )?;
    if db
        .query(
            "SELECT name FROM sqlite_master WHERE name='prd_schedules'",
            &[],
        )?
        .len()
        == 1
    {
        reviews::sync(
            db,
            match &saved {
                Saved::Goal(_) => "goal",
                Saved::Method(_) => "method",
                _ => "catalog",
            },
            saved.id(),
        )?;
    }
    Ok(saved)
}
pub fn history(db: &dyn Sql, id: &str) -> Result<Vec<Revision>> {
    valid_id(id)?;
    db.query(
        "SELECT data FROM prd_revisions WHERE entity_id=?1 ORDER BY version",
        params![id],
    )?
    .into_iter()
    .map(|row| serde_json::from_str(&row[0]).map_err(ApiError::from))
    .collect()
}
pub fn catalog_snapshot(db: &dyn Sql, id: &str, expected: CatalogKind) -> Result<CatalogSnapshot> {
    valid_id(id)?;
    let Some(Saved::Catalog(item)) = get(db, id)? else {
        return Err(ApiError::invalid("目录选项不存在"));
    };
    if item.input.kind != expected || !item.input.active {
        return Err(ApiError::invalid("请选择仍在使用的正确目录选项"));
    }
    Ok(CatalogSnapshot {
        id: item.id,
        version: item.version,
        kind: item.input.kind,
        name: item.input.name,
    })
}
pub fn snapshot(db: &dyn Sql) -> Result<Snapshot> {
    let mut result = Snapshot {
        schema: "skarmar.prd.v1".into(),
        goals: vec![],
        methods: vec![],
        catalog: vec![],
        lessons: lessons::list_lessons(db)?,
        sessions: sessions::list(db)?,
        knowledge: lessons::list_knowledge(db)?,
        actions: lessons::list_actions(db)?,
    };
    for row in db.query("SELECT data FROM prd_entities ORDER BY id", &[])? {
        match serde_json::from_str(&row[0])? {
            Saved::Goal(value) => result.goals.push(value),
            Saved::Method(value) => result.methods.push(value),
            Saved::Catalog(value) => result.catalog.push(value),
        }
    }
    Ok(result)
}

/// Authentication is shared, while domain tables and migration accounting stay separate.
pub fn initialize_authenticated(db: &dyn Sql) -> Result<()> {
    initialize(db)?;
    if db
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='auth_users'",
            &[],
        )?
        .is_empty()
    {
        db.execute_batch(include_str!("../../migrations/003_auth.sql"))?;
        db.execute_batch(include_str!("../../migrations/005_registration.sql"))?;
    }
    Ok(())
}
pub fn enabled(db: &dyn Sql) -> Result<bool> {
    Ok(!db
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='prd_schema'",
            &[],
        )?
        .is_empty())
}
