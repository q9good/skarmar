//! Versioned knowledge and practice events. Caller owns the transaction.
use super::{
    lesson_model::{Knowledge, KnowledgeKind, NewAction, NextAction},
    model::Saved,
    process_model::*,
};
use crate::{
    auth::{AccountRole, User},
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_date, valid_id},
};
use chrono::Utc;
use serde_json::{Value, json as value};
use std::collections::{BTreeMap, HashSet};
pub fn audit(actor: &User, note: &str) -> Audit {
    Audit {
        actor_id: actor.id.clone(),
        actor_name: actor.display_name.clone(),
        at: Utc::now().to_rfc3339(),
        note: note.into(),
    }
}
pub fn get(db: &dyn Sql, id: &str) -> Result<Knowledge> {
    valid_id(id)?;
    db.query("SELECT data FROM prd_knowledge WHERE id=?1", params![id])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()?
        .ok_or_else(ApiError::missing)
}
pub fn effective_state(k: &Knowledge) -> String {
    if !k.details.state.is_empty() {
        k.details.state.clone()
    } else if k.input.kind == KnowledgeKind::Difficulty {
        "pending".into()
    } else {
        "active".into()
    }
}
pub fn persist(db: &dyn Sql, k: &Knowledge) -> Result<()> {
    db.execute("INSERT INTO prd_knowledge(id,source_lesson_id,source_session_id,source_event_id,source_review_id,data) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![k.input.id,k.source_lesson_id,k.source_session_id,k.source_event_id,k.source_review_id,json(k)?])?;
    db.execute("INSERT OR IGNORE INTO prd_output_revisions(kind,id,version,data) VALUES('knowledge',?1,?2,?3)",params![k.input.id,k.version as i64,json(k)?])?;
    super::reviews::sync(db, "knowledge", &k.input.id)?;
    Ok(())
}
pub fn command(db: &dyn Sql, actor: &User, request: Command) -> Result<Value> {
    if !actor.active {
        return Err(crate::auth::unauthorized());
    }
    if let Command::Observe(write) = request {
        return super::observations::command(db, actor, write);
    }
    if let Command::Read(read) = request {
        return read_view(db, read);
    }
    let operation = match &request {
        Command::Knowledge(r) => &r.operation_id,
        Command::Action(r) => &r.operation_id,
        Command::Feedback(r) => &r.operation_id,
        Command::Settings(r) => &r.operation_id,
        Command::Review(r) => &r.operation_id,
        _ => unreachable!(),
    };
    valid_id(operation)?;
    let encoded = format!("process:{}", json(&request)?);
    if let Some(r) = db
        .query(
            "SELECT actor_id,request,response FROM prd_operations WHERE id=?1",
            params![operation.as_str()],
        )?
        .first()
    {
        if r[0] != actor.id || r[1] != encoded {
            return Err(ApiError::conflict("操作编号已用于其他内容"));
        }
        return Ok(serde_json::from_str(&r[2])?);
    }
    let response = match &request {
        Command::Action(r) => write_action(db, actor, r.clone())?,
        Command::Knowledge(r) => serde_json::to_value(write_knowledge(db, actor, r.clone())?)?,
        Command::Feedback(r) => write_feedback(db, actor, r.clone())?,
        Command::Settings(r) => super::reviews::write_settings(db, actor, r.clone())?,
        Command::Review(r) => super::reviews::write_review(db, actor, r.clone())?,
        _ => unreachable!(),
    };
    db.execute(
        "INSERT INTO prd_operations(id,actor_id,request,response) VALUES(?1,?2,?3,?4)",
        params![operation.as_str(), actor.id, encoded, json(&response)?],
    )?;
    Ok(response)
}
pub fn write_knowledge(db: &dyn Sql, actor: &User, r: KnowledgeWrite) -> Result<Knowledge> {
    valid_id(&r.id)?;
    let previous = db
        .query("SELECT data FROM prd_knowledge WHERE id=?1", params![r.id])?
        .first()
        .map(|r| serde_json::from_str::<Knowledge>(&r[0]).map_err(ApiError::from))
        .transpose()?;
    if previous.as_ref().map(|k| k.version) != r.expected_version {
        return Err(ApiError::conflict("经验或困难已有新版本，请保留填写后核对"));
    }
    if previous.as_ref().is_some_and(|k| k.deleted) {
        return Err(ApiError::conflict("此条已删除，不能继续修改"));
    }
    if r.action == "delete" {
        if actor.role != AccountRole::Admin {
            return Err(ApiError(
                axum::http::StatusCode::FORBIDDEN,
                "只有管理员可以删除经验或困难".into(),
            ));
        }
        let mut k = previous.ok_or_else(ApiError::missing)?;
        k.deleted = true;
        k.version += 1;
        k.audit = Some(audit(actor, &r.note));
        persist(db, &k)?;
        return Ok(k);
    }
    if !["create", "edit"].contains(&r.action.as_str())
        || ((r.action == "create") != previous.is_none())
    {
        return Err(ApiError::invalid("请选择新建或编辑"));
    }
    let input = r
        .input
        .ok_or_else(|| ApiError::invalid("请填写经验或困难"))?;
    if input.id != r.id || input.content.trim().is_empty() {
        return Err(ApiError::invalid("请填写内容并核对编号"));
    }
    if previous
        .as_ref()
        .is_some_and(|k| k.input.kind != input.kind)
    {
        return Err(ApiError::invalid("经验和困难不能直接互换类型"));
    }
    super::lessons::bindings(db, &input.goal_ids, &input.method_ids, true)?;
    let mut details = r.details;
    if previous.is_none()
        && (details.title.trim().is_empty() || details.title.chars().count() > 120)
    {
        return Err(ApiError::invalid("请写下1至120字的知识名称"));
    }
    if details.state.is_empty() {
        details.state = if input.kind == KnowledgeKind::Difficulty {
            "pending"
        } else {
            "active"
        }
        .into();
    }
    if input.kind == KnowledgeKind::Difficulty {
        if !["pending", "following", "resolved", "paused"].contains(&details.state.as_str()) {
            return Err(ApiError::invalid("困难状态无效"));
        }
    } else if !["active", "paused", "retired"].contains(&details.state.as_str())
        || input.stage.is_none()
    {
        return Err(ApiError::invalid("请明确经验阶段和使用状态"));
    }
    if let Some(date) = &details.comparison_start {
        valid_date(date)?;
    }
    if let Some(date) = &details.pause_until {
        valid_date(date)?;
    }
    if let Some(date) = &input.next_review_date {
        valid_date(date)?;
    }
    if let Some(old) = &previous {
        if (effective_state(old) != details.state
            || old.input.stage != input.stage
            || old.details.priority != details.priority)
            && r.note.trim().is_empty()
        {
            return Err(ApiError::invalid("改变阶段、状态或重点时，请说明依据"));
        }
        if details.state == "resolved"
            && effective_state(old) != "resolved"
            && details.evidence.trim().is_empty()
            && r.note.trim().is_empty()
        {
            return Err(ApiError::invalid("解决困难时请记下实际依据"));
        }
        if (old.details.basis != details.basis || old.details.support != details.support)
            && details.comparison_start == old.details.comparison_start
        {
            details.comparison_start = Some(super::sessions::today());
        }
    }
    if details.state != "paused" {
        details.pause_until = None;
    }
    let mut seen = HashSet::new();
    for d in &r.derived_from {
        if d.id == r.id || !seen.insert(&d.id) || d.difference.trim().is_empty() {
            return Err(ApiError::invalid("派生经验须说明差异且不能关联自身或重复"));
        }
        let old = get(db, &d.id)?;
        if old.version != d.version {
            let rows=db.query("SELECT data FROM prd_output_revisions WHERE kind='knowledge' AND id=?1 AND version=?2",params![d.id,d.version as i64])?;
            if rows.is_empty() {
                return Err(ApiError::conflict("原经验或困难的依据版本不存在，请核对"));
            }
        }
        let mut stack = old
            .derived_from
            .iter()
            .map(|d| d.id.clone())
            .collect::<Vec<_>>();
        let mut checked = HashSet::new();
        while let Some(id) = stack.pop() {
            if id == r.id {
                return Err(ApiError::invalid("派生关系不能形成循环"));
            }
            if checked.insert(id.clone()) {
                if checked.len() > 100 {
                    return Err(ApiError::invalid("派生关系过多，请缩小范围"));
                }
                stack.extend(get(db, &id)?.derived_from.into_iter().map(|d| d.id));
            }
        }
    }
    let k = if let Some(mut k) = previous {
        k.input = input;
        k.details = details;
        k.derived_from = r.derived_from;
        k.version += 1;
        k.audit = Some(audit(actor, &r.note));
        k
    } else {
        Knowledge {
            input,
            version: 1,
            details,
            derived_from: r.derived_from,
            source_lesson_id: None,
            source_session_id: None,
            source_session_version: None,
            source_event_id: None,
            source_review_id: None,
            source_observation_version: 0,
            source_observation: None,
            source_summary: None,
            deleted: false,
            author_id: actor.id.clone(),
            author_name: actor.display_name.clone(),
            created_at: Utc::now().to_rfc3339(),
            audit: Some(audit(actor, &r.note)),
        }
    };
    persist(db, &k)?;
    Ok(k)
}
pub fn feedback(db: &dyn Sql, id: &str) -> Result<Feedback> {
    valid_id(id)?;
    db.query("SELECT data FROM prd_feedback WHERE id=?1", params![id])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()?
        .ok_or_else(ApiError::missing)
}
pub fn source(db: &dyn Sql, s: &Source) -> Result<Value> {
    valid_id(&s.id)?;
    match s.kind.as_str() {
        "feedback" => {
            let current = feedback(db, &s.id)?;
            let e = if current.version == s.version {
                current
            } else {
                db.query(
                    "SELECT data FROM prd_feedback_revisions WHERE event_id=?1 AND version=?2",
                    params![s.id, s.version as i64],
                )?
                .first()
                .map(|r| serde_json::from_str::<Feedback>(&r[0]).map_err(ApiError::from))
                .transpose()?
                .ok_or_else(|| ApiError::invalid("反馈依据版本不存在"))?
            };
            Ok(serde_json::to_value(e)?)
        }
        "independent" => Ok(value!({"kind":"independent","id":s.id})),
        "session" => {
            let current = super::sessions::get_session(db, &s.id)?.ok_or_else(ApiError::missing)?;
            let mut snapshot = if current.version == s.version {
                current
            } else {
                super::sessions::history(db, &s.id)?
                    .into_iter()
                    .find(|r| r.session.version == s.version)
                    .ok_or_else(|| ApiError::invalid("来源干预版本不存在"))?
                    .session
            };
            if snapshot.status != "completed" {
                return Err(ApiError::invalid("反馈来源须为正式完成的干预"));
            }
            snapshot.lesson_sources.clear();
            snapshot.action_snapshots.clear();
            if let Some(summary) = snapshot.input.summary.as_mut() {
                summary.new_knowledge.clear();
                summary.knowledge_details = None;
                summary.new_actions.clear();
                summary.feedback.clear();
                summary.knowledge_versions.clear();
            }
            Ok(serde_json::to_value(snapshot)?)
        }
        "lesson" => {
            let current = super::lessons::get_lesson(db, &s.id)?.ok_or_else(ApiError::missing)?;
            let snapshot = if current.version == s.version {
                current
            } else {
                super::lessons::history(db, &s.id)?
                    .into_iter()
                    .find(|r| r.lesson.version == s.version)
                    .ok_or_else(|| ApiError::invalid("来源课记版本不存在"))?
                    .lesson
            };
            if snapshot.status != "recorded" {
                return Err(ApiError::invalid("来源课记未正式记录"));
            }
            Ok(serde_json::to_value(snapshot)?)
        }
        "moment" | "measurement" => {
            let r = super::observations::historical(db, &s.id, s.version)?;
            if r.kind != s.kind {
                return Err(ApiError::invalid("来源类型不匹配"));
            }
            Ok(serde_json::to_value(r)?)
        }
        _ => Err(ApiError::invalid("来源类型无效")),
    }
}
fn method_snapshots(db: &dyn Sql, input: &FeedbackInput) -> Result<Vec<super::model::Method>> {
    let mut out = vec![];
    let mut seen = HashSet::new();
    for r in &input.method_refs {
        if !seen.insert(&r.id) {
            return Err(ApiError::invalid("方法不能重复"));
        }
        let method = match super::get(db, &r.id)? {
            Some(Saved::Method(m)) if m.version == r.version => m,
            _ => super::history(db, &r.id)?
                .into_iter()
                .find_map(|r0| match r0.entity {
                    Saved::Method(m) if m.version == r.version => Some(m),
                    _ => None,
                })
                .ok_or_else(|| ApiError::invalid("方法版本不存在"))?,
        };
        out.push(method);
    }
    Ok(out)
}
pub fn create_actions(
    db: &dyn Sql,
    actor: &User,
    inputs: &[NewAction],
    event: Option<&str>,
    review: Option<&str>,
) -> Result<Vec<String>> {
    if inputs.len() > 20 {
        return Err(ApiError::invalid("一次最多20项下一步"));
    }
    let mut ids = HashSet::new();
    let mut out = vec![];
    for input in inputs {
        if let Some(date) = &input.suggested_date {
            valid_date(date)?;
        }
        valid_id(&input.id)?;
        if input.content.trim().is_empty() || !ids.insert(&input.id) {
            return Err(ApiError::invalid("请写下下一步，编号不能重复"));
        }
        super::lessons::bindings(db, &input.goal_ids, &input.method_ids, true)?;
        if !db
            .query("SELECT id FROM prd_actions WHERE id=?1", params![input.id])?
            .is_empty()
        {
            return Err(ApiError::conflict("下一步编号已存在"));
        }
        let name = if let Some(id) = &input.assignee_id {
            Some(
                db.query(
                    "SELECT display_name FROM auth_users WHERE id=?1 AND active=1",
                    params![id.as_str()],
                )?
                .first()
                .ok_or_else(|| ApiError::invalid("负责人不是有效成员"))?[0]
                    .clone(),
            )
        } else {
            None
        };
        let a = NextAction {
            audit: Some(super::followups::audit(actor, "明确保存下一步")),
            source_deleted: false,
            input: input.clone(),
            version: 1,
            source_lesson_id: None,
            source_session_id: None,
            source_session_version: None,
            source_observation_version: 0,
            source_event_id: event.map(str::to_string),
            source_review_id: review.map(str::to_string),
            author_id: actor.id.clone(),
            author_name: actor.display_name.clone(),
            assignee_name: name,
            status: "pending".into(),
            plan_id: None,
            created_at: Utc::now().to_rfc3339(),
        };
        db.execute(
            "INSERT INTO prd_actions(id,source_event_id,source_review_id,data) VALUES(?1,?2,?3,?4)",
            params![input.id, a.source_event_id, a.source_review_id, json(&a)?],
        )?;
        db.execute(
            "INSERT INTO prd_output_revisions(kind,id,version,data) VALUES('action',?1,1,?2)",
            params![input.id, json(&a)?],
        )?;
        out.push(input.id.clone());
    }
    Ok(out)
}
pub fn write_feedback(db: &dyn Sql, actor: &User, r: FeedbackWrite) -> Result<Value> {
    valid_id(&r.id)?;
    let previous = db
        .query("SELECT data FROM prd_feedback WHERE id=?1", params![r.id])?
        .first()
        .map(|r| serde_json::from_str::<Feedback>(&r[0]).map_err(ApiError::from))
        .transpose()?;
    if r.action == "annotate" {
        let old = previous.ok_or_else(ApiError::missing)?;
        let version = r
            .expected_version
            .ok_or_else(|| ApiError::invalid("请注明所查看的结果版本"))?;
        if version > old.version || version == 0 || old.deleted || r.note.trim().is_empty() {
            return Err(ApiError::invalid("请填写附注并核对所查看的版本"));
        }
        let a = Annotation {
            id: r.operation_id.clone(),
            event_id: r.id,
            based_on_version: version,
            text: r.note.clone(),
            audit: audit(actor, "补充不同观察"),
        };
        db.execute(
            "INSERT INTO prd_annotations(id,event_id,data) VALUES(?1,?2,?3)",
            params![a.id, a.event_id, json(&a)?],
        )?;
        return Ok(serde_json::to_value(a)?);
    }
    if previous.as_ref().map(|e| e.version) != r.expected_version {
        return Err(ApiError::conflict("此条反馈已有修订，请保留输入并核对"));
    }
    if previous.as_ref().is_some_and(|e| e.deleted) {
        return Err(ApiError::conflict("此条事件已由管理员删除"));
    }
    let mut event = if r.action == "delete" {
        if actor.role != AccountRole::Admin {
            return Err(ApiError(
                axum::http::StatusCode::FORBIDDEN,
                "只有管理员可删除误建的事件".into(),
            ));
        }
        let mut e = previous.clone().ok_or_else(ApiError::missing)?;
        e.deleted = true;
        e.version += 1;
        e.audit = audit(actor, &r.note);
        e
    } else {
        if !["record", "revise"].contains(&r.action.as_str())
            || ((r.action == "record") != previous.is_none())
        {
            return Err(ApiError::invalid("请选择记录、修订或附注"));
        }
        let input = r.input.ok_or_else(|| ApiError::invalid("请填写本次反馈"))?;
        let k = get(db, &input.knowledge_id)?;
        if k.deleted {
            return Err(ApiError::conflict("关联的经验或困难已删除"));
        }
        if let Some(old) = &previous {
            if old.input.knowledge_id != input.knowledge_id
                || old.input.source.kind != input.source.kind
                || old.input.source.id != input.source.id
            {
                return Err(ApiError::invalid("修订不能更换所反馈的对象或来源"));
            }
            if r.note.trim().is_empty() {
                return Err(ApiError::invalid("更正主结果时请说明差异"));
            }
        }
        valid_date(&input.business_date)?;
        if input.business_date > super::sessions::today() {
            return Err(ApiError::invalid("未来的实践不能记为已发生"));
        }
        if input.observation.trim().is_empty() {
            return Err(ApiError::invalid("请记下一句实际观察"));
        }
        let allowed = if k.input.kind == KnowledgeKind::Experience {
            vec!["helpful", "uncertain", "unhelpful"]
        } else {
            vec!["improved", "same", "harder", "unknown", "unobserved"]
        };
        if !allowed.contains(&input.result.as_str()) {
            return Err(ApiError::invalid("反馈结果与对象类型不一致"));
        }
        if !["", "implementation_uncertain", "no_change_observed"].contains(&input.reason.as_str())
        {
            return Err(ApiError::invalid("原因选项无效"));
        }
        if input.comparable
            && (input.result == "unobserved"
                || input.result == "unknown"
                || input.basis.trim().is_empty()
                || input.support.trim().is_empty())
        {
            return Err(ApiError::invalid(
                "可比较观察须注明同口径和帮助条件；未观察到不能算可比较",
            ));
        }
        if !["independent", "lesson", "session"].contains(&input.source.kind.as_str()) {
            return Err(ApiError::invalid("请选择课记、干预或独立实践来源"));
        }
        let mut snapshot = source(db, &input.source)?;
        if let Some(version) = input.observation_version {
            if input.source.kind != "lesson" {
                return Err(ApiError::invalid("观察版本仅适用于课记来源"));
            }
            let lesson: super::lesson_model::Lesson = serde_json::from_value(snapshot.clone())?;
            let observation = super::lessons::source_observation(db, &lesson, version)?;
            snapshot["referenced_observation"] = value!({"version":version,"value":observation});
        }
        if input.source.kind != "independent" {
            let date = snapshot
                .pointer(if input.source.kind == "session" {
                    "/input/business_date"
                } else {
                    if input.observation_version.is_some() {
                        "/referenced_observation/value/business_date"
                    } else {
                        "/observation/business_date"
                    }
                })
                .and_then(Value::as_str)
                .unwrap_or("");
            if date != input.business_date {
                return Err(ApiError::invalid("实践日期须与所选来源日期一致"));
            }
        }
        if let Some(row)=db.query("SELECT id FROM prd_feedback WHERE knowledge_id=?1 AND source_kind=?2 AND source_id=?3",params![input.knowledge_id,input.source.kind,input.source.id])?.first()&&row[0]!=r.id{return Err(ApiError::conflict("同一来源对这条经验或困难已有记录，请打开原记录修订或补附注"));}
        let methods = method_snapshots(db, &input)?;
        if input.source.kind != "independent" {
            let path = if input.source.kind == "session" {
                "/actual_methods"
            } else {
                "/organization/methods"
            };
            let actual = snapshot
                .pointer(path)
                .and_then(Value::as_array)
                .ok_or_else(ApiError::storage)?;
            if input.method_refs.iter().any(|m| {
                !actual.iter().any(|a| {
                    a["id"].as_str() == Some(m.id.as_str())
                        && a["version"].as_u64() == Some(m.version)
                })
            }) {
                return Err(ApiError::invalid(
                    "方法版本须与来源中明确实际使用的方法一致",
                ));
            }
        }

        Feedback {
            id: r.id.clone(),
            version: previous.as_ref().map_or(1, |e| e.version + 1),
            input,
            methods,
            source_snapshot: snapshot,
            author_id: previous
                .as_ref()
                .map_or(actor.id.clone(), |e| e.author_id.clone()),
            author_name: previous
                .as_ref()
                .map_or(actor.display_name.clone(), |e| e.author_name.clone()),
            created_at: previous
                .as_ref()
                .map_or(Utc::now().to_rfc3339(), |e| e.created_at.clone()),
            audit: audit(actor, &r.note),
            deleted: false,
        }
    };
    if previous.is_some() && !r.next_actions.is_empty() {
        return Err(ApiError::invalid(
            "修订已有反馈不能重放创建下一步；请在本次复盘中明确新增",
        ));
    }
    if event.input.result == "unobserved" {
        event.input.comparable = false;
    }
    db.execute("INSERT INTO prd_feedback(id,knowledge_id,source_kind,source_id,version,data) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET version=excluded.version,data=excluded.data",params![event.id,event.input.knowledge_id,event.input.source.kind,event.input.source.id,event.version as i64,json(&event)?])?;
    db.execute(
        "INSERT INTO prd_feedback_revisions(event_id,version,data) VALUES(?1,?2,?3)",
        params![event.id, event.version as i64, json(&event)?],
    )?;
    if event.input.source.kind == "lesson" && !event.deleted {
        db.execute("INSERT OR IGNORE INTO prd_lesson_refs(lesson_id,source_kind,source_id) VALUES(?1,'feedback',?2)",params![event.input.source.id,event.id])?;
    }
    create_actions(db, actor, &r.next_actions, Some(&event.id), None)?;
    Ok(serde_json::to_value(event)?)
}
pub fn events(db: &dyn Sql, id: &str) -> Result<Vec<Feedback>> {
    db.query(
        "SELECT data FROM prd_feedback WHERE knowledge_id=?1 ORDER BY id",
        params![id],
    )?
    .into_iter()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .collect()
}
pub fn summary(db: &dyn Sql, k: &Knowledge) -> Result<Value> {
    let boundary = super::comparison::boundary(db, k)?;
    let mut events: Vec<_> = events(db, &k.input.id)?
        .into_iter()
        .filter(|e| !e.deleted)
        .collect();
    events.sort_by(|a, b| {
        b.input
            .business_date
            .cmp(&a.input.business_date)
            .then(b.created_at.cmp(&a.created_at))
            .then(b.id.cmp(&a.id))
    });
    let mut counts = BTreeMap::<String, u32>::new();
    for e in &events {
        *counts.entry(e.input.result.clone()).or_default() += 1;
    }
    let comparable: Vec<_> = events
        .iter()
        .filter(|e| {
            e.input.comparable
                && super::comparison::in_phase(e, &boundary)
                && e.input.result != "unobserved"
                && e.input.result != "unknown"
                && k.details
                    .comparison_start
                    .as_ref()
                    .is_none_or(|d| e.input.business_date >= *d)
                && !k.details.basis.is_empty()
                && !k.details.support.is_empty()
                && e.input.basis == k.details.basis
                && e.input.support == k.details.support
        })
        .collect();
    let offer = !k.deleted
        && ["pending", "following"].contains(&effective_state(k).as_str())
        && k.input.kind == KnowledgeKind::Difficulty
        && comparable.len() >= 3
        && comparable
            .first()
            .is_some_and(|e| e.input.result == "same" || e.input.result == "harder")
        && !k.details.priority;
    let mut latest_results = BTreeMap::new();
    for e in &events {
        latest_results.entry(e.input.result.clone()).or_insert(e);
    }
    Ok(
        value!({"counts":counts,"total":events.len(),"latest":events.iter().take(3).collect::<Vec<_>>(),"latest_results":latest_results,"comparison_boundary":boundary.as_ref().map(|b|value!({"date":b.date,"at":b.at,"cause":b.cause,"method_id":if b.cause=="method" {Some(&b.method_id)} else {None},"method_version":if b.cause=="method" {Some(b.version)} else {None}})),"comparable_count":comparable.len(),"offer_weekly":offer}),
    )
}
pub fn read_view(db: &dyn Sql, r: Read) -> Result<Value> {
    if r.page > 10000 {
        return Err(ApiError::invalid("页数过大"));
    }
    let offset = u64::from(r.page) * 30;
    match r.view.as_str() {
        "entity_history" | "lesson_history" | "session_history" => {
            valid_id(&r.id)?;
            let (table, column) = match r.view.as_str() {
                "entity_history" => ("prd_revisions", "entity_id"),
                "lesson_history" => ("prd_lesson_revisions", "lesson_id"),
                _ => ("prd_session_revisions", "session_id"),
            };
            let rows=db.query(&format!("SELECT data FROM {table} WHERE {column}=?1 ORDER BY version DESC LIMIT 31 OFFSET {offset}"),params![r.id])?;
            let items = rows
                .iter()
                .take(30)
                .map(|row| serde_json::from_str::<Value>(&row[0]).map_err(ApiError::from))
                .collect::<Result<Vec<_>>>()?;
            Ok(value!({"items":items,"has_more":rows.len()>30,"page":r.page}))
        }
        "review_queue" => {
            // The caller holds the read/command transaction. Relations, names
            // and schedules are one snapshot, never two polling generations.
            let board = read_view(
                db,
                Read {
                    view: "board".into(),
                    id: String::new(),
                    object_kind: String::new(),
                    page: 0,
                },
            )?;
            Ok(value!({"board":board,"state":super::snapshot(db)?}))
        }
        "board" => {
            super::reviews::seed(db)?;
            let mut summaries = serde_json::Map::new();
            for k in super::lessons::list_knowledge(db)?
                .into_iter()
                .filter(|k| !k.deleted)
            {
                summaries.insert(k.input.id.clone(), summary(db, &k)?);
            }
            let requests=super::lessons::list_lessons(db)?.into_iter().filter(|l|l.status=="recorded"&&l.request_generation>0&&(l.organization.processed_request_generation<l.request_generation||l.organization.state!="done")).map(|l|value!({"id":l.id,"business_date":l.observation.business_date,"course":l.course.name,"claimed_by":l.organization.claimed_by,"claimed_name":l.organization.claimed_name,"message":l.request_message})).collect::<Vec<_>>();
            let actions = super::lessons::list_actions(db)?
                .into_iter()
                .filter(|a| a.status == "pending")
                .collect::<Vec<_>>();
            Ok(
                value!({"settings":super::reviews::settings(db)?,"schedules":super::reviews::schedules(db)?,"summaries":summaries,"members":members(db)?,"inbox":{"requests":requests,"actions":actions},"data_version":data_version(db)?}),
            )
        }
        "knowledge" => {
            let k = get(db, &r.id)?;
            let rows=db.query(&format!("SELECT data FROM prd_output_revisions WHERE kind='knowledge' AND id=?1 ORDER BY version DESC LIMIT 31 OFFSET {offset}"),params![r.id])?;
            let history: Vec<Knowledge> = rows
                .iter()
                .take(30)
                .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
                .collect::<Result<_>>()?;
            Ok(
                value!({"knowledge":k,"summary":summary(db,&k)?,"history":history,"has_more":rows.len()>30}),
            )
        }
        "events" => {
            let mut all = events(db, &r.id)?;
            all.sort_by(|a, b| {
                b.input
                    .business_date
                    .cmp(&a.input.business_date)
                    .then(b.created_at.cmp(&a.created_at))
                    .then(b.id.cmp(&a.id))
            });
            let page: Vec<_> = all.iter().skip(offset as usize).take(30).collect();
            let mut annotations = vec![];
            for e in &page {
                for row in db.query(
                    "SELECT data FROM prd_annotations WHERE event_id=?1 ORDER BY id",
                    params![e.id],
                )? {
                    annotations.push(serde_json::from_str::<Annotation>(&row[0])?);
                }
            }
            Ok(
                value!({"events":page,"annotations":annotations,"has_more":all.len()>offset as usize+30}),
            )
        }
        "lesson_feedback" => {
            valid_id(&r.id)?;
            let rows=db.query("SELECT data FROM prd_feedback WHERE source_kind='lesson' AND source_id=?1 ORDER BY id",params![r.id])?;
            Ok(serde_json::to_value(
                rows.into_iter()
                    .map(|r| serde_json::from_str::<Feedback>(&r[0]).map_err(ApiError::from))
                    .collect::<Result<Vec<_>>>()?,
            )?)
        }
        "goal_progress" => super::observations::progress(db),
        "session_feedback" => {
            valid_id(&r.id)?;
            let rows=db.query("SELECT data FROM prd_feedback WHERE source_kind='session' AND source_id=?1 ORDER BY id",params![r.id])?;
            let events: Vec<Feedback> = rows
                .into_iter()
                .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
                .collect::<Result<_>>()?;
            Ok(serde_json::to_value(events)?)
        }
        "event_history" => {
            feedback(db, &r.id)?;
            let rows=db.query(&format!("SELECT data FROM prd_feedback_revisions WHERE event_id=?1 ORDER BY version DESC LIMIT 31 OFFSET {offset}"),params![r.id])?;
            let page: Vec<Feedback> = rows
                .iter()
                .take(30)
                .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
                .collect::<Result<_>>()?;
            Ok(value!({"history":page,"has_more":rows.len()>30}))
        }
        "review_context" => {
            let mut c = super::reviews::context(db, &r.object_kind, &r.id, r.page)?;
            if let Some(v) = c.as_object_mut() {
                v.remove("_all_versions");
            }
            Ok(c)
        }
        "action" => {
            valid_id(&r.id)?;
            let rows=db.query(&format!("SELECT data FROM prd_output_revisions WHERE kind='action' AND id=?1 ORDER BY version DESC LIMIT 31 OFFSET {offset}"),params![r.id])?;
            let history: Vec<NextAction> = rows
                .iter()
                .take(30)
                .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
                .collect::<Result<_>>()?;
            Ok(value!({"history":history,"has_more":rows.len()>30}))
        }
        "review_history" => {
            valid_id(&r.id)?;
            let rows=db.query(&format!("SELECT data FROM prd_review_revisions WHERE review_id=?1 ORDER BY version DESC LIMIT 31 OFFSET {offset}"),params![r.id])?;
            let history = rows
                .iter()
                .take(30)
                .map(|r| serde_json::from_str::<Review>(&r[0]).map_err(ApiError::from))
                .collect::<Result<Vec<_>>>()?;
            Ok(value!({"history":history,"has_more":rows.len()>30}))
        }
        "review" => {
            valid_id(&r.id)?;
            let row = db.query("SELECT data FROM prd_reviews WHERE id=?1", params![r.id])?;
            let review: Review =
                serde_json::from_str(&row.first().ok_or_else(ApiError::missing)?[0])?;
            Ok(serde_json::to_value(review)?)
        }
        "reviews" => {
            let rows = db.query(
                &format!(
                    "SELECT data FROM prd_reviews ORDER BY rowid DESC LIMIT 31 OFFSET {offset}"
                ),
                &[],
            )?;
            let page: Vec<Review> = rows
                .iter()
                .take(30)
                .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
                .collect::<Result<_>>()?;
            Ok(value!({"reviews":page,"has_more":rows.len()>30}))
        }
        "observation" | "observations" | "observation_history" => {
            super::observations::read(db, &r.view, &r.id, &r.object_kind, r.page)
        }
        _ => Err(ApiError::invalid("查看类型无效")),
    }
}

fn data_version(db: &dyn Sql) -> Result<u64> {
    let mut n = 0;
    for table in [
        "prd_entities",
        "prd_lessons",
        "prd_sessions",
        "prd_feedback",
        "prd_reviews",
        "prd_schedules",
        "prd_observations",
    ] {
        let rows = db.query(
            &format!("SELECT CAST(COALESCE(SUM(version),0) AS TEXT) FROM {table}"),
            &[],
        )?;
        n += rows[0][0].parse::<u64>().map_err(|_| ApiError::storage())?;
    }
    for query in [
        "SELECT CAST(COALESCE(SUM(json_extract(data,'$.version')),0) AS TEXT) FROM prd_knowledge",
        "SELECT CAST(COUNT(*) AS TEXT) FROM prd_annotations",
        "SELECT CAST(COALESCE(SUM(json_extract(data,'$.version')),0) AS TEXT) FROM prd_actions",
        "SELECT CAST(COALESCE(SUM(json_extract(data,'$.version')),0) AS TEXT) FROM prd_settings",
    ] {
        n += db.query(query, &[])?[0][0]
            .parse::<u64>()
            .map_err(|_| ApiError::storage())?;
    }
    Ok(n)
}

pub fn practice_feedback(
    db: &dyn Sql,
    actor: &User,
    session: &super::session_model::Session,
    inputs: &[PracticeFeedback],
    note: &str,
) -> Result<Vec<PracticeFeedback>> {
    if inputs.len() > 20 {
        return Err(ApiError::invalid("一次最多20条使用反馈，请分次保存"));
    }
    let mut out = vec![];
    let mut seen = HashSet::new();
    let summary = session
        .input
        .summary
        .as_ref()
        .ok_or_else(ApiError::storage)?;
    let existing = db
        .query(
            "SELECT data FROM prd_feedback WHERE source_kind='session' AND source_id=?1",
            params![session.id],
        )?
        .into_iter()
        .map(|r| serde_json::from_str::<Feedback>(&r[0]).map_err(ApiError::from))
        .collect::<Result<Vec<_>>>()?;
    if existing
        .iter()
        .any(|e| !e.deleted && !inputs.iter().any(|i| i.id == e.id))
    {
        return Err(ApiError::invalid(
            "本次已有经验反馈或困难跟进，请保留并到跟进中核对，不会静默作废",
        ));
    }
    for input in inputs {
        valid_id(&input.id)?;
        if !seen.insert(&input.knowledge_id) {
            return Err(ApiError::invalid("同一次干预对同一对象只能记录一次反馈"));
        }
        let old = existing.iter().find(|e| e.id == input.id);
        if old.is_some_and(|e| e.deleted) {
            continue;
        }
        let old_source_version = old.map_or(session.version, |e| e.input.source.version);
        let observation = if input.observation.trim().is_empty() {
            summary
                .targets
                .iter()
                .filter(|t| t.performed)
                .map(|t| t.observation.clone())
                .collect::<Vec<_>>()
                .join("；")
        } else {
            input.observation.clone()
        };
        let mut value = FeedbackInput {
            observation_version: None,
            knowledge_id: input.knowledge_id.clone(),
            source: Source {
                kind: "session".into(),
                id: session.id.clone(),
                version: old_source_version,
            },
            business_date: session.input.business_date.clone(),
            result: input.result.clone(),
            observation,
            attempt: input.attempt.clone(),
            adult_response: input.adult_response.clone(),
            conditions: input.conditions.clone(),
            reason: input.reason.clone(),
            comparable: input.comparable,
            basis: input.basis.clone(),
            support: input.support.clone(),
            method_refs: session
                .actual_methods
                .iter()
                .map(|m| super::session_model::MethodRef {
                    id: m.id.clone(),
                    version: m.version,
                })
                .collect(),
        };
        if let Some(old) = old
            && json(&old.input)? == json(&value)?
        {
            let mut i = input.clone();
            i.expected_version = Some(old.version);
            out.push(i);
            continue;
        }
        value.source.version = session.version;
        let write = FeedbackWrite {
            id: input.id.clone(),
            operation_id: uuid::Uuid::new_v4().to_string(),
            expected_version: input.expected_version,
            action: if old.is_some() { "revise" } else { "record" }.into(),
            input: Some(value),
            note: note.into(),
            next_actions: vec![],
        };
        let saved: Feedback = serde_json::from_value(write_feedback(db, actor, write)?)?;
        let mut i = input.clone();
        i.expected_version = Some(saved.version);
        out.push(i);
    }
    Ok(out)
}

fn members(db: &dyn Sql) -> Result<Vec<Value>> {
    if db
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='auth_users'",
            &[],
        )?
        .is_empty()
    {
        return Ok(vec![]);
    }
    Ok(db
        .query(
            "SELECT id,display_name FROM auth_users WHERE active=1 ORDER BY display_name,id",
            &[],
        )?
        .into_iter()
        .map(|r| value!({"id":r[0],"name":r[1]}))
        .collect())
}
fn write_action(db: &dyn Sql, actor: &User, r: ActionWrite) -> Result<Value> {
    if let Some(date) = &r.input.suggested_date {
        valid_date(date)?;
    }
    valid_id(&r.id)?;
    let row = db.query("SELECT data FROM prd_actions WHERE id=?1", params![r.id])?;
    let mut a: NextAction = serde_json::from_str(&row.first().ok_or_else(ApiError::missing)?[0])?;
    if a.version != r.expected_version {
        return Err(ApiError::conflict("下一步已有更新，请保留填写后核对"));
    }
    if a.status == "in_plan" {
        return Err(ApiError::conflict(
            "下一步正在某份计划中，请先从该计划处理结果或取消占用",
        ));
    }
    if r.input.id != r.id
        || r.input.content.trim().is_empty()
        || !["pending", "completed", "cancelled"].contains(&r.state.as_str())
    {
        return Err(ApiError::invalid("请填写下一步并明确状态"));
    }
    super::lessons::bindings(db, &r.input.goal_ids, &r.input.method_ids, true)?;
    if a.status != r.state && r.note.trim().is_empty() {
        return Err(ApiError::invalid("改变下一步状态时请说明原因"));
    }
    if a.input.assignee_id != r.input.assignee_id {
        a.assignee_name = if let Some(id) = &r.input.assignee_id {
            Some(
                db.query(
                    "SELECT display_name FROM auth_users WHERE id=?1 AND active=1",
                    params![id.as_str()],
                )?
                .first()
                .ok_or_else(|| ApiError::invalid("负责人不是有效家人"))?[0]
                    .clone(),
            )
        } else {
            None
        };
    }
    a.audit = Some(audit(actor, &r.note));
    a.input = r.input;
    a.status = r.state;
    if a.status != "completed" {
        a.plan_id = None;
    }
    a.version += 1;
    db.execute(
        "UPDATE prd_actions SET data=?2 WHERE id=?1",
        params![a.input.id, json(&a)?],
    )?;
    db.execute(
        "INSERT INTO prd_output_revisions(kind,id,version,data) VALUES('action',?1,?2,?3)",
        params![a.input.id, a.version as i64, json(&a)?],
    )?;
    Ok(serde_json::to_value(a)?)
}
