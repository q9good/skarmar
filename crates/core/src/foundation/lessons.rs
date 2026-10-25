//! Independent lesson partitions; caller encloses each command in a transaction.
use super::{
    catalog_snapshot, get,
    lesson_model::*,
    model::{CatalogKind, Saved},
};
use crate::{
    auth::{AccountRole, User},
    model::Level,
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_date, valid_id},
};
use chrono::{DateTime, Datelike, Duration, FixedOffset, Utc, Weekday};
use std::collections::HashSet;
fn offset() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).expect("valid Shanghai offset")
}
pub fn get_lesson(db: &dyn Sql, id: &str) -> Result<Option<Lesson>> {
    db.query("SELECT data FROM prd_lessons WHERE id=?1", params![id])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()
}
pub fn history(db: &dyn Sql, id: &str) -> Result<Vec<LessonRevision>> {
    valid_id(id)?;
    db.query(
        "SELECT data FROM prd_lesson_revisions WHERE lesson_id=?1 ORDER BY version",
        params![id],
    )?
    .into_iter()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .collect()
}
pub fn list_lessons(db: &dyn Sql) -> Result<Vec<Lesson>> {
    list(db, "SELECT data FROM prd_lessons ORDER BY id")
}
pub fn list_knowledge(db: &dyn Sql) -> Result<Vec<Knowledge>> {
    list(db, "SELECT data FROM prd_knowledge ORDER BY id")
}
pub fn list_actions(db: &dyn Sql) -> Result<Vec<NextAction>> {
    let mut actions: Vec<NextAction> = list(db, "SELECT data FROM prd_actions ORDER BY id")?;
    for a in &mut actions {
        for (table, id) in [
            ("prd_sessions", a.source_session_id.as_deref()),
            ("prd_lessons", a.source_lesson_id.as_deref()),
            ("prd_feedback", a.source_event_id.as_deref()),
            ("prd_reviews", a.source_review_id.as_deref()),
        ] {
            if let Some(id) = id {
                let rows = db.query(
                    &format!("SELECT data FROM {table} WHERE id=?1"),
                    params![id],
                )?;
                if let Some(row) = rows.first() {
                    let v: serde_json::Value = serde_json::from_str(&row[0])?;
                    if v["deleted"].as_bool() == Some(true)
                        || ["deleted", "cancelled", "withdrawn"]
                            .contains(&v["status"].as_str().unwrap_or(""))
                    {
                        a.source_deleted = true;
                    }
                }
            }
        }
    }
    Ok(actions)
}
fn list<T: serde::de::DeserializeOwned>(db: &dyn Sql, query: &str) -> Result<Vec<T>> {
    db.query(query, &[])?
        .into_iter()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .collect()
}
fn unique(ids: &[String]) -> Result<()> {
    if ids.len() > 100 {
        return Err(ApiError::invalid("一次最多关联100项"));
    }
    let mut seen = HashSet::new();
    for id in ids {
        valid_id(id)?;
        if !seen.insert(id) {
            return Err(ApiError::invalid("关联项不能重复"));
        }
    }
    Ok(())
}
pub(super) fn bindings(
    db: &dyn Sql,
    goals: &[String],
    methods: &[String],
    required: bool,
) -> Result<()> {
    unique(goals)?;
    unique(methods)?;
    if required && goals.is_empty() && methods.is_empty() {
        return Err(ApiError::invalid("请明确关联至少一个目标或方法"));
    }
    for id in goals {
        if !matches!(get(db, id)?, Some(Saved::Goal(_))) {
            return Err(ApiError::invalid("关联目标不存在"));
        }
    }
    for id in methods {
        if !matches!(get(db, id)?, Some(Saved::Method(_))) {
            return Err(ApiError::invalid("关联方法不存在"));
        }
    }
    Ok(())
}
fn admin(actor: &User) -> Result<()> {
    if actor.role != AccountRole::Admin {
        return Err(ApiError(
            axum::http::StatusCode::FORBIDDEN,
            "只有管理员可以删除或清除待核对标记".into(),
        ));
    }
    Ok(())
}
fn value_references_lesson(value: &serde_json::Value, id: &str) -> bool {
    value["evidence"].as_array().is_some_and(|sources| {
        sources
            .iter()
            .any(|source| source["kind"] == "lesson" && source["id"] == id)
    }) || value
        .pointer("/input/source")
        .is_some_and(|source| source["kind"] == "lesson" && source["id"] == id)
}
fn inbound_references(db: &dyn Sql, id: &str) -> Result<Vec<String>> {
    let mut references = db
        .query(
            "SELECT source_kind FROM prd_lesson_refs WHERE lesson_id=?1",
            params![id],
        )?
        .into_iter()
        .map(|row| row[0].clone())
        .collect::<Vec<_>>();
    for (table, kind) in [
        ("prd_reviews", "review"),
        ("prd_review_revisions", "review"),
        ("prd_observations", "measurement"),
        ("prd_observation_revisions", "measurement"),
    ] {
        for row in db.query(&format!("SELECT data FROM {table}"), &[])? {
            let value: serde_json::Value = serde_json::from_str(&row[0])?;
            if value_references_lesson(&value, id) {
                references.push(kind.into());
            }
        }
    }
    references.sort();
    references.dedup();
    Ok(references)
}
fn capability_for(
    db: &dyn Sql,
    actor: &User,
    lesson: &Lesson,
    now: DateTime<Utc>,
) -> Result<WithdrawalCapability> {
    let denied = |code: &str, reason: &str, references: Vec<String>| WithdrawalCapability {
        allowed: false,
        reason_code: code.into(),
        reason: reason.into(),
        lesson_version: lesson.version,
        references,
    };
    if lesson.status != "recorded" {
        return Ok(denied(
            "not_recorded",
            "课记已撤销或删除，不能再次撤销",
            vec![],
        ));
    }
    if lesson.creator_id != actor.id {
        return Ok(denied(
            "not_creator",
            "只有记录者可以撤销；可标记记错，请管理员处理",
            vec![],
        ));
    }
    let created = DateTime::parse_from_rfc3339(&lesson.created_at)
        .map_err(|_| ApiError::storage())?
        .with_timezone(&offset())
        .date_naive();
    if created != now.with_timezone(&offset()).date_naive() {
        return Ok(denied(
            "outside_creation_day",
            "已超过服务器创建时间所属的家庭业务日；可标记记错，请管理员处理",
            vec![],
        ));
    }
    let mut references = inbound_references(db, &lesson.id)?;
    if lesson.organization.claimed_by.is_some() {
        references.push("organization".into());
    }
    if !lesson.acknowledgements.is_empty() {
        references.push("acknowledgement".into());
    }
    references.sort();
    references.dedup();
    if !references.is_empty() {
        return Ok(denied(
            "referenced",
            "课记已被整理、回应或其他记录引用；请标记记错，由管理员核对处理",
            references,
        ));
    }
    Ok(WithdrawalCapability {
        allowed: true,
        reason_code: "allowed".into(),
        reason: "记录者可在创建当天撤销这条未被引用的课记".into(),
        lesson_version: lesson.version,
        references: vec![],
    })
}
pub fn withdrawal_capability(db: &dyn Sql, actor: &User, id: &str) -> Result<WithdrawalCapability> {
    valid_id(id)?;
    if !actor.active {
        return Err(crate::auth::unauthorized());
    }
    let lesson = get_lesson(db, id)?.ok_or_else(ApiError::missing)?;
    capability_for(db, actor, &lesson, Utc::now())
}
fn inbound(db: &dyn Sql, id: &str) -> Result<bool> {
    Ok(!inbound_references(db, id)?.is_empty())
}
pub(super) fn source_observation(
    db: &dyn Sql,
    lesson: &Lesson,
    version: u64,
) -> Result<ObservationInput> {
    if version == lesson.observation_version {
        return Ok(lesson.observation.clone());
    }
    history(db, &lesson.id)?
        .into_iter()
        .find(|r| r.lesson.observation_version == version && r.event == "observation")
        .map(|r| r.lesson.observation)
        .ok_or_else(|| ApiError::invalid("所引用的课堂观察版本不存在"))
}
pub fn write(db: &dyn Sql, actor: &User, request: LessonWrite) -> Result<Lesson> {
    write_at(db, actor, request, Utc::now())
}
pub(crate) fn write_at(
    db: &dyn Sql,
    actor: &User,
    request: LessonWrite,
    now: DateTime<Utc>,
) -> Result<Lesson> {
    valid_id(&actor.id)?;
    valid_id(&request.id)?;
    valid_id(&request.operation_id)?;
    if !actor.active {
        return Err(crate::auth::unauthorized());
    }
    let encoded = format!("lesson:{}", json(&request)?);
    if let Some(row) = db
        .query(
            "SELECT actor_id,request,response FROM prd_operations WHERE id=?1",
            params![request.operation_id],
        )?
        .first()
    {
        if row[0] != actor.id || row[1] != encoded {
            return Err(ApiError::conflict("操作编号已用于其他内容"));
        }
        return Ok(serde_json::from_str(&row[2])?);
    }
    let old = get_lesson(db, &request.id)?;
    let expected = match (&old, &request.action) {
        (None, LessonAction::Observation(_)) => None,
        (Some(l), LessonAction::Observation(_)) => Some(l.observation_version),
        (Some(l), LessonAction::Organization(_)) => Some(l.organization_version),
        (Some(l), _) => Some(l.version),
        _ => return Err(ApiError::missing()),
    };
    if expected != request.expected_version {
        return Err(ApiError::conflict(
            "这一部分已有新版本，你的填写仍保留，请核对",
        ));
    }
    if old.as_ref().is_some_and(|l| l.status != "recorded") {
        return Err(ApiError::conflict("课记已撤销或删除，无法继续修改"));
    }
    let today = now.with_timezone(&offset()).date_naive();
    let stamp = now.to_rfc3339();
    let mut changed = true;
    let mut embedded_feedback = vec![];
    let mut knowledge = vec![];
    let mut actions = vec![];
    let (mut lesson, event) = match request.action {
        LessonAction::Observation(mut input) => {
            if input.course_started.is_none() {
                input.course_started = old.as_ref().and_then(|l| l.observation.course_started);
            }
            valid_date(&input.business_date)?;
            if input.business_date > today.to_string() {
                return Err(ApiError::invalid("未来的课程不能记为已经上课"));
            }
            let course = if let Some(l) = &old
                && l.course.id == input.course_id
            {
                l.course.clone()
            } else {
                catalog_snapshot(db, &input.course_id, CatalogKind::Course)?
            };
            let teacher = if let Some(l) = &old
                && l.teacher.id == input.teacher_id
            {
                l.teacher.clone()
            } else {
                catalog_snapshot(db, &input.teacher_id, CatalogKind::Teacher)?
            };
            let mut l = old.clone().unwrap_or_else(|| Lesson {
                id: request.id.clone(),
                version: 0,
                observation_version: 0,
                organization_version: 1,
                created_at: stamp.clone(),
                creator_id: actor.id.clone(),
                creator_name: actor.display_name.clone(),
                status: "recorded".into(),
                marked_error: false,
                observation: input.clone(),
                course: course.clone(),
                teacher: teacher.clone(),
                organization: Organization::default(),
                request_generation: 0,
                requested_by: None,
                request_name: None,
                request_message: String::new(),
                acknowledgements: vec![],
            });
            l.observation_version += 1;
            l.course = course;
            l.teacher = teacher;
            if input.request_review {
                l.request_generation += 1;
                l.requested_by = Some(actor.id.clone());
                l.request_name = Some(actor.display_name.clone());
                l.request_message = input.review_message.clone();
            }
            l.observation = input;
            (l, "observation")
        }
        LessonAction::Organization(input) => {
            let mut l = old.clone().ok_or_else(ApiError::missing)?;
            let source = source_observation(db, &l, input.seen_observation_version)?;
            let ids: Vec<_> = input.targets.iter().map(|t| t.goal_id.clone()).collect();
            bindings(db, &ids, &input.method_ids, false)?;
            unique(&input.focus_goal_ids)?;
            let mut targets = vec![];
            for t in input.targets {
                let Some(Saved::Goal(g)) = get(db, &t.goal_id)? else {
                    return Err(ApiError::invalid("关联目标不存在"));
                };
                if g.input.level == Level::Long {
                    return Err(ApiError::invalid("长期方向请放在长期关注点中"));
                }
                let snapshot = l
                    .organization
                    .targets
                    .iter()
                    .find(|old| old.target.goal_id == t.goal_id)
                    .map(|old| old.goal.clone())
                    .unwrap_or(g);
                targets.push(TargetSnapshot {
                    target: t.clone(),
                    ancestors: l
                        .organization
                        .targets
                        .iter()
                        .find(|old| old.target.goal_id == t.goal_id)
                        .map(|old| old.ancestors.clone())
                        .unwrap_or(super::sessions::ancestors(db, &snapshot)?),
                    goal: snapshot,
                });
            }
            let mut methods = vec![];
            for id in input.method_ids {
                let Some(Saved::Method(m)) = get(db, &id)? else {
                    return Err(ApiError::invalid("关联方法不存在"));
                };
                methods.push(
                    l.organization
                        .methods
                        .iter()
                        .find(|old| old.id == m.id)
                        .cloned()
                        .unwrap_or(m),
                );
            }
            let mut focus = vec![];
            for id in input.focus_goal_ids {
                let Some(Saved::Goal(g)) = get(db, &id)? else {
                    return Err(ApiError::invalid("长期方向不存在"));
                };
                if g.input.level != Level::Long {
                    return Err(ApiError::invalid("长期关注点须为长期方向"));
                }
                focus.push(
                    l.organization
                        .focus_goals
                        .iter()
                        .find(|old| old.id == g.id)
                        .cloned()
                        .unwrap_or(g),
                );
            }
            if input.processing != Processing::Keep {
                if input.request_generation != l.request_generation {
                    return Err(ApiError::conflict("查看请求已有变化，请刷新后核对"));
                }
                let other = l.organization.processed_request_generation == l.request_generation
                    && l.organization.state == "claimed"
                    && l.organization
                        .claimed_by
                        .as_ref()
                        .is_some_and(|id| id != &actor.id);
                if other && matches!(input.processing, Processing::Claim | Processing::Complete) {
                    return Err(ApiError::conflict("另一位家人已认领，请明确选择接手"));
                }
                // An explicit request to revisit an otherwise unrequested lesson
                // must appear in the queue as well; ordinary edits never do this.
                if input.processing == Processing::Reopen && l.request_generation == 0 {
                    l.request_generation = 1;
                    l.requested_by = Some(actor.id.clone());
                    l.request_name = Some(actor.display_name.clone());
                    l.request_message = input.outcome.clone();
                }
                l.organization.processed_request_generation = l.request_generation;
                l.organization.state = if input.processing == Processing::Complete {
                    "done"
                } else if input.processing == Processing::Reopen {
                    "pending"
                } else {
                    "claimed"
                }
                .into();
                l.organization.claimed_by = if input.processing == Processing::Reopen {
                    None
                } else {
                    Some(actor.id.clone())
                };
                l.organization.claimed_name = if input.processing == Processing::Reopen {
                    None
                } else {
                    Some(actor.display_name.clone())
                };
            }
            if input.new_knowledge.len() > 20 || input.new_actions.len() > 20 {
                return Err(ApiError::invalid("一次最多整理20项，请分次保存"));
            }
            for mut k in input.new_knowledge {
                valid_id(&k.id)?;
                if k.content.trim().is_empty() {
                    return Err(ApiError::invalid("请写下要保留的经验或困难"));
                }
                bindings(db, &k.goal_ids, &k.method_ids, true)?;
                if k.kind == KnowledgeKind::Experience && k.stage.is_none() {
                    return Err(ApiError::invalid("请明确选择经验目前的阶段"));
                }
                if k.kind == KnowledgeKind::Difficulty && k.next_review_date.is_none() {
                    let start = today + Duration::days(14);
                    let shift = (Weekday::Sat.num_days_from_monday() + 7
                        - start.weekday().num_days_from_monday())
                        % 7;
                    k.next_review_date =
                        Some((start + Duration::days(i64::from(shift))).to_string());
                }
                if let Some(date) = &k.next_review_date {
                    valid_date(date)?;
                }
                knowledge.push(Knowledge {
                    details: Default::default(),
                    derived_from: vec![],
                    source_event_id: None,
                    source_review_id: None,
                    deleted: false,
                    audit: None,
                    input: k,
                    version: 1,
                    source_lesson_id: Some(l.id.clone()),
                    source_session_id: None,
                    source_session_version: None,
                    source_observation_version: input.seen_observation_version,
                    source_observation: Some(source.clone()),
                    source_summary: None,
                    author_id: actor.id.clone(),
                    author_name: actor.display_name.clone(),
                    created_at: stamp.clone(),
                });
            }
            for a in input.new_actions {
                if let Some(date) = &a.suggested_date {
                    crate::store::valid_date(date)?;
                }
                valid_id(&a.id)?;
                if a.content.trim().is_empty() {
                    return Err(ApiError::invalid("请写下下一步要做什么"));
                }
                bindings(db, &a.goal_ids, &a.method_ids, true)?;
                let assignee_name = if let Some(id) = &a.assignee_id {
                    valid_id(id)?;
                    Some(
                        db.query(
                            "SELECT display_name FROM auth_users WHERE id=?1 AND active=1",
                            params![id.as_str()],
                        )?
                        .first()
                        .ok_or_else(|| ApiError::invalid("负责人不是有效家人成员"))?[0]
                            .clone(),
                    )
                } else {
                    None
                };
                actions.push(NextAction {
                    audit: Some(super::followups::audit(actor, "明确保存下一步")),
                    source_deleted: false,
                    source_event_id: None,
                    source_review_id: None,
                    input: a,
                    version: 1,
                    source_lesson_id: Some(l.id.clone()),
                    source_session_id: None,
                    source_session_version: None,
                    source_observation_version: input.seen_observation_version,
                    author_id: actor.id.clone(),
                    author_name: actor.display_name.clone(),
                    assignee_name,
                    status: "pending".into(),
                    plan_id: None,
                    created_at: stamp.clone(),
                });
            }
            embedded_feedback = input.feedback;
            let feedback_versions = input.feedback_knowledge_versions;
            if embedded_feedback.len() > 20 {
                return Err(ApiError::invalid("一次最多20条课记反馈"));
            }
            let mut seen = HashSet::new();
            for f in &embedded_feedback {
                valid_id(&f.id)?;
                if f.expected_version.is_some() || !seen.insert(&f.knowledge_id) {
                    return Err(ApiError::invalid(
                        "本课同一经验只能新增一次反馈；已有反馈请到跟进中明确修订",
                    ));
                }
                let k = super::followups::get(db, &f.knowledge_id)?;
                if k.deleted || k.input.kind != KnowledgeKind::Experience {
                    return Err(ApiError::invalid("请选择尚未删除的经验"));
                }
                if feedback_versions.get(&f.knowledge_id).copied() != Some(k.version) {
                    return Err(ApiError::conflict("经验已有新版本，请保留课堂填写并核对"));
                }
                if !k.input.goal_ids.iter().any(|g| {
                    targets.iter().any(|t| &t.goal.id == g) || focus.iter().any(|t| &t.id == g)
                }) && !k
                    .input
                    .method_ids
                    .iter()
                    .any(|id| methods.iter().any(|m| &m.id == id))
                {
                    return Err(ApiError::invalid(
                        "经验须与本课明确目标、方向或实际方法关联",
                    ));
                }
                l.organization.feedback_ids.push(f.id.clone());
            }
            l.organization.seen_observation_version = input.seen_observation_version;
            l.organization.targets = targets;
            l.organization.methods = methods;
            l.organization.focus_goals = focus;
            l.organization.outcome = input.outcome;
            l.organization_version += 1;
            (l, "organization")
        }
        LessonAction::Withdraw => {
            let mut l = old.clone().ok_or_else(ApiError::missing)?;
            let capability = capability_for(db, actor, &l, now)?;
            if !capability.allowed {
                let status = if ["not_creator", "outside_creation_day"]
                    .contains(&capability.reason_code.as_str())
                {
                    axum::http::StatusCode::FORBIDDEN
                } else {
                    axum::http::StatusCode::CONFLICT
                };
                return Err(ApiError(status, capability.reason));
            }
            l.status = "withdrawn".into();
            (l, "withdrawn")
        }
        LessonAction::Delete => {
            admin(actor)?;
            let mut l = old.clone().ok_or_else(ApiError::missing)?;
            if inbound(db, &l.id)? {
                return Err(ApiError::conflict("课记已有派生内容，需先处理引用关系"));
            }
            l.status = "deleted".into();
            (l, "deleted")
        }
        LessonAction::MarkError { marked } => {
            if !marked {
                admin(actor)?;
            }
            let mut l = old.clone().ok_or_else(ApiError::missing)?;
            changed = l.marked_error != marked;
            l.marked_error = marked;
            (l, "marked_error")
        }
        LessonAction::Acknowledge => {
            let mut l = old.clone().ok_or_else(ApiError::missing)?;
            if l.acknowledgements.contains(&actor.id) {
                changed = false;
            } else {
                l.acknowledgements.push(actor.id.clone());
            }
            (l, "acknowledged")
        }
    };
    if changed {
        lesson.version += 1;
    }
    // Insert every referenced output in the same transaction as the organization partition.
    for k in &knowledge {
        if !db
            .query(
                "SELECT id FROM prd_knowledge WHERE id=?1",
                params![k.input.id],
            )?
            .is_empty()
        {
            return Err(ApiError::conflict("经验或困难编号已存在"));
        }
        db.execute(
            "INSERT INTO prd_knowledge(id,source_lesson_id,data) VALUES(?1,?2,?3)",
            params![k.input.id, lesson.id, json(k)?],
        )?;
        db.execute("INSERT INTO prd_lesson_refs(lesson_id,source_kind,source_id) VALUES(?1,'knowledge',?2)",params![lesson.id,k.input.id])?;
    }
    for a in &actions {
        if !db
            .query(
                "SELECT id FROM prd_actions WHERE id=?1",
                params![a.input.id],
            )?
            .is_empty()
        {
            return Err(ApiError::conflict("下一步编号已存在"));
        }
        db.execute(
            "INSERT INTO prd_actions(id,source_lesson_id,data) VALUES(?1,?2,?3)",
            params![a.input.id, lesson.id, json(a)?],
        )?;
        db.execute(
            "INSERT INTO prd_lesson_refs(lesson_id,source_kind,source_id) VALUES(?1,'action',?2)",
            params![lesson.id, a.input.id],
        )?;
    }
    if !embedded_feedback.is_empty() {
        // Publish the new partition ONLY within this transaction, so feedback
        // can reference the exact saved lesson version; any failure rolls it back.
        db.execute(
            "UPDATE prd_lessons SET version=?2,data=?3 WHERE id=?1",
            params![lesson.id, lesson.version as i64, json(&lesson)?],
        )?;
        for f in &embedded_feedback {
            let selected =
                source_observation(db, &lesson, lesson.organization.seen_observation_version)?;
            super::followups::write_feedback(
                db,
                actor,
                super::process_model::FeedbackWrite {
                    id: f.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    expected_version: None,
                    action: "record".into(),
                    input: Some(super::process_model::FeedbackInput {
                        observation_version: Some(lesson.organization.seen_observation_version),
                        knowledge_id: f.knowledge_id.clone(),
                        source: super::process_model::Source {
                            kind: "lesson".into(),
                            id: lesson.id.clone(),
                            version: lesson.version,
                        },
                        business_date: selected.business_date,
                        result: f.result.clone(),
                        observation: f.observation.clone(),
                        attempt: f.attempt.clone(),
                        adult_response: f.adult_response.clone(),
                        conditions: f.conditions.clone(),
                        reason: f.reason.clone(),
                        comparable: f.comparable,
                        basis: f.basis.clone(),
                        support: f.support.clone(),
                        method_refs: lesson
                            .organization
                            .methods
                            .iter()
                            .map(|m| super::session_model::MethodRef {
                                id: m.id.clone(),
                                version: m.version,
                            })
                            .collect(),
                    }),
                    note: "家人明确记录本课经验反馈；与整理一起提交".into(),
                    next_actions: vec![],
                },
            )?;
        }
    }
    db.execute(
        "INSERT INTO prd_operations(id,actor_id,request,response) VALUES(?1,?2,?3,?4)",
        params![request.operation_id, actor.id, encoded, json(&lesson)?],
    )?;
    if changed {
        db.execute("INSERT INTO prd_lessons(id,version,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET version=excluded.version,data=excluded.data",params![lesson.id,lesson.version as i64,json(&lesson)?])?;
        let revision = LessonRevision {
            lesson: lesson.clone(),
            actor_id: actor.id.clone(),
            actor_name: actor.display_name.clone(),
            event: event.into(),
            recorded_at: stamp,
        };
        db.execute("INSERT INTO prd_lesson_revisions(lesson_id,version,operation_id,data) VALUES(?1,?2,?3,?4)",params![lesson.id,lesson.version as i64,request.operation_id,json(&revision)?])?;
    }
    if !db
        .query(
            "SELECT name FROM sqlite_master WHERE name='prd_schedules'",
            &[],
        )?
        .is_empty()
    {
        for k in &knowledge {
            super::followups::persist(db, k)?;
        }
    }
    Ok(lesson)
}
