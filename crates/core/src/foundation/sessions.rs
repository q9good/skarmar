//! Shared transactional family intervention commands for SQLite and Durable Objects.
use super::{
    get,
    lesson_model::{Knowledge, KnowledgeKind, NextAction},
    model::{Goal, Method, Saved},
    session_model::*,
};
use crate::{
    auth::{AccountRole, User},
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_date, valid_id},
};
use chrono::{FixedOffset, Utc};
use std::collections::HashSet;
pub fn today() -> String {
    Utc::now()
        .with_timezone(&FixedOffset::east_opt(28800).unwrap())
        .date_naive()
        .to_string()
}
pub fn get_session(db: &dyn Sql, id: &str) -> Result<Option<Session>> {
    db.query("SELECT data FROM prd_sessions WHERE id=?1", params![id])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()
}
pub fn list(db: &dyn Sql) -> Result<Vec<Session>> {
    db.query("SELECT data FROM prd_sessions ORDER BY id", &[])?
        .into_iter()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .collect()
}
pub fn history(db: &dyn Sql, id: &str) -> Result<Vec<Revision>> {
    valid_id(id)?;
    db.query(
        "SELECT data FROM prd_session_revisions WHERE session_id=?1 ORDER BY version",
        params![id],
    )?
    .into_iter()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .collect()
}
fn goal(db: &dyn Sql, id: &str) -> Result<Goal> {
    match get(db, id)? {
        Some(Saved::Goal(g)) => Ok(g),
        _ => Err(ApiError::invalid("目标不存在")),
    }
}
pub(super) fn ancestors(db: &dyn Sql, g: &Goal) -> Result<Vec<Goal>> {
    let mut out = vec![];
    let mut parent = g.input.parent_id.clone();
    while let Some(id) = parent {
        if out.len() > 3 {
            return Err(ApiError::invalid("目标层级无效"));
        }
        let p = goal(db, &id)?;
        parent = p.input.parent_id.clone();
        out.push(p);
    }
    Ok(out)
}
fn unique(ids: &[String]) -> Result<()> {
    if ids.len() > 100 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err(ApiError::invalid("关联项过多或重复"));
    }
    for id in ids {
        valid_id(id)?;
    }
    Ok(())
}
fn methods(db: &dyn Sql, refs: &[MethodRef]) -> Result<Vec<Method>> {
    unique(&refs.iter().map(|r| r.id.clone()).collect::<Vec<_>>())?;
    refs.iter()
        .map(|r| {
            let m = match get(db, &r.id)? {
                Some(Saved::Method(m)) => m,
                _ => return Err(ApiError::invalid("方法不存在")),
            };
            if m.version == r.version {
                return Ok(m);
            }
            super::history(db, &r.id)?
                .into_iter()
                .find_map(|h| match h.entity {
                    Saved::Method(m) if m.version == r.version => Some(m),
                    _ => None,
                })
                .ok_or_else(|| ApiError::invalid("方法版本不存在"))
        })
        .collect()
}
fn targets(db: &dyn Sql, input: &[Target], old: &[TargetSnapshot]) -> Result<Vec<TargetSnapshot>> {
    unique(&input.iter().map(|t| t.goal_id.clone()).collect::<Vec<_>>())?;
    if input.iter().filter(|t| t.role == Role::Main).count() > 1 {
        return Err(ApiError::invalid("主要目标只能选一个"));
    }
    input
        .iter()
        .map(|t| {
            let g = goal(db, &t.goal_id)?;
            if g.input.level == crate::model::Level::Long {
                return Err(ApiError::invalid("长期方向请放在长期关注点"));
            }
            let (g, parents) = if let Some(before) = old.iter().find(|b| b.goal.id == t.goal_id) {
                (before.goal.clone(), before.ancestors.clone())
            } else {
                let parents = ancestors(db, &g)?;
                (g, parents)
            };
            Ok(TargetSnapshot {
                input: t.clone(),
                goal: g,
                ancestors: parents,
            })
        })
        .collect()
}
fn normalized_tags(tags: &mut Vec<String>) -> Result<()> {
    for t in tags.iter_mut() {
        *t = t.trim().to_string();
    }
    tags.retain(|t| !t.is_empty());
    tags.sort();
    tags.dedup();
    if tags.len() > 20 || tags.iter().any(|t| t.chars().count() > 40) {
        return Err(ApiError::invalid("标签最多20个，每项不超过40字"));
    }
    Ok(())
}
fn save_action(db: &dyn Sql, a: &NextAction) -> Result<()> {
    db.execute(
        "UPDATE prd_actions SET data=?2 WHERE id=?1",
        params![a.input.id, json(a)?],
    )?;
    Ok(())
}
fn get_action(db: &dyn Sql, id: &str) -> Result<NextAction> {
    db.query("SELECT data FROM prd_actions WHERE id=?1", params![id])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()?
        .ok_or_else(|| ApiError::invalid("下一步不存在"))
}
fn revision_output<T: serde::Serialize>(
    db: &dyn Sql,
    kind: &str,
    id: &str,
    version: u64,
    v: &T,
) -> Result<()> {
    db.execute(
        "INSERT INTO prd_output_revisions(kind,id,version,data) VALUES(?1,?2,?3,?4)",
        params![kind, id, version as i64, json(v)?],
    )?;
    Ok(())
}
fn change_action(db: &dyn Sql, a: &mut NextAction, actor: &User, note: &str) -> Result<()> {
    a.version += 1;
    a.audit = Some(super::followups::audit(
        actor,
        &format!("干预计划中的行动变更：{note}"),
    ));
    save_action(db, a)?;
    revision_output(db, "action", &a.input.id, a.version, a)
}
pub fn write(db: &dyn Sql, actor: &User, request: Write) -> Result<Session> {
    valid_id(&request.id)?;
    valid_id(&request.operation_id)?;
    if !actor.active {
        return Err(crate::auth::unauthorized());
    }
    let encoded = format!("session:{}", json(&request)?);
    if let Some(r) = db
        .query(
            "SELECT actor_id,request,response FROM prd_operations WHERE id=?1",
            params![request.operation_id],
        )?
        .first()
    {
        if r[0] != actor.id || r[1] != encoded {
            return Err(ApiError::conflict("操作编号已用于其他内容"));
        }
        return Ok(serde_json::from_str(&r[2])?);
    }
    let old = get_session(db, &request.id)?;
    if old.as_ref().map(|s| s.version) != request.expected_version {
        return Err(ApiError::conflict("干预已有新版本，你的填写仍保留，请核对"));
    }
    if old
        .as_ref()
        .is_some_and(|s| s.status == "deleted" || s.status == "cancelled")
    {
        return Err(ApiError::conflict("计划已取消或记录已删除"));
    }
    let stamp = Utc::now().to_rfc3339();
    let removing = matches!(request.action, Action::Cancel | Action::Delete);
    if matches!(request.action, Action::Delete) && actor.role != AccountRole::Admin {
        return Err(ApiError(
            axum::http::StatusCode::FORBIDDEN,
            "只有管理员可以删除干预".into(),
        ));
    }
    if matches!(request.action, Action::Cancel)
        && old.as_ref().is_some_and(|s| s.status == "completed")
    {
        return Err(ApiError::invalid("已完成干预不能取消计划"));
    }
    if old.as_ref().is_some_and(|s| s.status == "completed")
        && matches!(request.action, Action::SaveDraft)
    {
        return Err(ApiError::invalid("已完成总结请使用修订保存，不能退回草稿"));
    }
    let mut input = if removing {
        old.as_ref().ok_or_else(ApiError::missing)?.input.clone()
    } else {
        request
            .input
            .clone()
            .ok_or_else(|| ApiError::invalid("缺少干预内容"))?
    };
    valid_date(&input.business_date)?;
    if !["夜间", "上午", "下午", "晚上"].contains(&input.time_slot.as_str()) {
        return Err(ApiError::invalid("请选择时段"));
    }
    normalized_tags(&mut input.tags)?;
    unique(&input.focus_goal_ids)?;
    unique(&input.action_ids)?;
    let mut session = Session {
        id: request.id.clone(),
        version: old.as_ref().map_or(1, |s| s.version + 1),
        status: String::new(),
        creator_id: old
            .as_ref()
            .map_or(actor.id.clone(), |s| s.creator_id.clone()),
        creator_name: old
            .as_ref()
            .map_or(actor.display_name.clone(), |s| s.creator_name.clone()),
        created_at: old.as_ref().map_or(stamp.clone(), |s| s.created_at.clone()),
        input: input.clone(),
        planned_targets: vec![],
        actual_targets: vec![],
        planned_methods: vec![],
        actual_methods: vec![],
        focus_goals: vec![],
        lesson_sources: vec![],
        action_snapshots: vec![],
    };
    session.planned_targets = targets(
        db,
        &input.targets,
        old.as_ref().map_or(&[], |s| s.planned_targets.as_slice()),
    )?;
    session.planned_methods = methods(db, &input.methods)?;
    for id in &input.focus_goal_ids {
        let g = goal(db, id)?;
        if g.input.level != crate::model::Level::Long {
            return Err(ApiError::invalid("关注点须为长期方向"));
        }
        session.focus_goals.push(
            old.as_ref()
                .and_then(|s| s.focus_goals.iter().find(|g| g.id == *id))
                .cloned()
                .unwrap_or(g),
        );
    }
    unique(
        &input
            .lesson_sources
            .iter()
            .map(|s| s.id.clone())
            .collect::<Vec<_>>(),
    )?;
    for source in &input.lesson_sources {
        let current = super::lessons::get_lesson(db, &source.id)?
            .ok_or_else(|| ApiError::invalid("来源课记不存在"))?;
        if current.status != "recorded" {
            return Err(ApiError::conflict("来源课记已撤销或删除"));
        }
        let snapshot = if current.version == source.version {
            current
        } else {
            super::lessons::history(db, &source.id)?
                .into_iter()
                .find(|r| r.lesson.version == source.version)
                .ok_or_else(|| ApiError::invalid("来源课记版本不存在"))?
                .lesson
        };
        session.lesson_sources.push(snapshot);
    }
    let complete = matches!(request.action, Action::Complete);
    if complete {
        if input.business_date > today() {
            return Err(ApiError::invalid("未来干预不能记为已完成"));
        }
        if old.as_ref().is_some_and(|s| s.status == "completed")
            && request.change_note.trim().is_empty()
        {
            return Err(ApiError::invalid("修改已完成总结时请说明改了什么"));
        }
        let summary = input
            .summary
            .as_mut()
            .ok_or_else(|| ApiError::invalid("请填写本次总结"))?;
        unique(
            &summary
                .targets
                .iter()
                .map(|t| t.goal_id.clone())
                .collect::<Vec<_>>(),
        )?;
        let performed: Vec<_> = summary.targets.iter().filter(|t| t.performed).collect();
        if performed.is_empty() || performed.iter().any(|t| t.observation.trim().is_empty()) {
            return Err(ApiError::invalid("至少记录一个实际目标，并写一句表现"));
        }
        let role_for = |id: &str| {
            input
                .targets
                .iter()
                .find(|t| t.goal_id == id)
                .map_or(Role::Secondary, |t| t.role.clone())
        };
        let actual: Vec<_> = performed
            .iter()
            .map(|t| Target {
                goal_id: t.goal_id.clone(),
                role: role_for(&t.goal_id),
                note: t.observation.clone(),
            })
            .collect();
        session.actual_targets = targets(
            db,
            &actual,
            old.as_ref().map_or(&[], |s| s.actual_targets.as_slice()),
        )?;
        match summary.confirmation {
            Confirmation::Unconfirmed => {
                return Err(ApiError::invalid(
                    "请确认实际使用的方法，也可选择没有或记不清",
                ));
            }
            Confirmation::Used if summary.methods.is_empty() => {
                return Err(ApiError::invalid("请选择实际使用的方法"));
            }
            Confirmation::None | Confirmation::Unknown if !summary.methods.is_empty() => {
                return Err(ApiError::invalid("没有／记不清时不能同时计实际方法"));
            }
            _ => {}
        }
        session.actual_methods = methods(db, &summary.methods)?;
        unique(
            &summary
                .action_outcomes
                .iter()
                .map(|o| o.action_id.clone())
                .collect::<Vec<_>>(),
        )?;
        if summary
            .action_outcomes
            .iter()
            .any(|o| !input.action_ids.contains(&o.action_id))
        {
            return Err(ApiError::invalid("下一步结果必须属于本次纳入的行动"));
        }
        unique(&summary.first_try_ids)?;
        if summary
            .first_try_ids
            .iter()
            .any(|id| !summary.methods.iter().any(|m| m.id == *id))
        {
            return Err(ApiError::invalid("本次开始尝试必须属于实际使用的方法"));
        }

        normalized_tags(&mut summary.tags)?;
        for signal in &summary.signals {
            if signal
                .method_ids
                .iter()
                .any(|id| !summary.methods.iter().any(|m| m.id == *id))
            {
                return Err(ApiError::invalid("信号只能关联本次实际方法"));
            }
            if ![
                "主动开始",
                "主动表达或分享",
                "求助",
                "受挫后再试",
                "休息后回来再试",
                "投入愉快",
                "勉强或抗拒",
                "明显不适或疲劳",
            ]
            .contains(&signal.kind.as_str())
            {
                return Err(ApiError::invalid("参与信号无效"));
            }
            unique(&signal.goal_ids)?;
            if signal.goal_ids.iter().any(|id| {
                !session.actual_targets.iter().any(|t| t.goal.id == *id)
                    && !input.focus_goal_ids.contains(id)
            }) {
                return Err(ApiError::invalid(
                    "信号只能明确关联本次实际目标或长期关注点",
                ));
            }
        }
        if summary
            .signals
            .iter()
            .map(|s| &s.kind)
            .collect::<HashSet<_>>()
            .len()
            != summary.signals.len()
        {
            return Err(ApiError::invalid("参与信号不能重复"));
        }
        if summary.new_knowledge.len() > 20 || summary.new_actions.len() > 20 {
            return Err(ApiError::invalid("一次最多保存20条产出"));
        }
        if summary.knowledge_details.as_ref().is_some_and(|details| {
            details.len() > 20
                || details
                    .keys()
                    .any(|id| !summary.new_knowledge.iter().any(|k| &k.id == id))
        }) {
            return Err(ApiError::invalid("困难结构化内容须对应本次新建的知识"));
        }
    }
    for id in &input.action_ids {
        if !old
            .as_ref()
            .is_some_and(|s| s.input.action_ids.contains(id))
            && let Some(expected) = input.action_versions.get(id)
            && get_action(db, id)?.version != *expected
        {
            return Err(ApiError::conflict(
                "下一步内容已有新版本，请保留安排并核对行动后再纳入",
            ));
        }
        let a = old
            .as_ref()
            .and_then(|s| s.action_snapshots.iter().find(|a| a.input.id == *id))
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| get_action(db, id))?;
        session.action_snapshots.push(a);
    }
    session.status = match request.action {
        Action::SaveDraft => {
            if input.summary.is_some() {
                "summary_draft"
            } else {
                "planned"
            }
        }
        Action::Complete => "completed",
        Action::Cancel => "cancelled",
        Action::Delete => "deleted",
    }
    .into();
    session.input = input;
    // Insert the parent before outputs with FKs; the caller's transaction rolls all of it back on failure.
    db.execute("INSERT INTO prd_sessions(id,version,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET version=excluded.version,data=excluded.data",params![session.id,session.version as i64,json(&session)?])?;
    db.execute(
        "DELETE FROM prd_lesson_refs WHERE source_kind='plan' AND source_id=?1",
        params![session.id],
    )?;
    if !removing {
        for source in &session.input.lesson_sources {
            db.execute(
                "INSERT INTO prd_lesson_refs(lesson_id,source_kind,source_id) VALUES(?1,'plan',?2)",
                params![source.id, session.id],
            )?;
        }
    }
    // Reserve actions on shared drafts; never silently steal one from a concurrent plan.
    let previous_ids = old
        .as_ref()
        .map(|s| s.input.action_ids.clone())
        .unwrap_or_default();
    for id in previous_ids
        .iter()
        .filter(|id| removing || !session.input.action_ids.contains(id))
    {
        let mut a = get_action(db, id)?;
        if a.plan_id.as_deref() == Some(&session.id) && a.status == "in_plan" {
            a.plan_id = None;
            a.status = "pending".into();
            change_action(db, &mut a, actor, &request.change_note)?;
        }
    }
    if !removing {
        for id in &session.input.action_ids {
            let mut a = get_action(db, id)?;
            let before = old
                .as_ref()
                .and_then(|s| s.input.summary.as_ref())
                .and_then(|s| s.action_outcomes.iter().find(|o| o.action_id == *id));
            let outcome = if complete {
                Some(
                    session
                        .input
                        .summary
                        .as_ref()
                        .unwrap()
                        .action_outcomes
                        .iter()
                        .find(|o| o.action_id == *id)
                        .ok_or_else(|| ApiError::invalid("请逐项确认下一步是已做还是还要继续"))?,
                )
            } else {
                None
            };
            if complete
                && old.as_ref().is_some_and(|s| s.status == "completed")
                && before.map(|o| &o.decision) == outcome.map(|o| &o.decision)
            {
                continue;
            }
            if a.plan_id.as_deref() != Some(&session.id)
                && (a.plan_id.is_some() || a.status != "pending")
            {
                return Err(ApiError::conflict("下一步已被其他计划使用或完成，请核对"));
            }
            let (status, plan) = if let Some(o) = outcome {
                match o.decision.as_str() {
                    "done" => ("completed", Some(session.id.clone())),
                    "continue" | "not_done" => ("pending", None),
                    _ => return Err(ApiError::invalid("下一步结果无效")),
                }
            } else {
                ("in_plan", Some(session.id.clone()))
            };
            if a.status != status || a.plan_id != plan {
                a.status = status.into();
                a.plan_id = plan;
                change_action(db, &mut a, actor, &request.change_note)?;
            }
        }
    }
    if complete {
        outputs(db, actor, &session, &stamp, &request.change_note)?;
        let feedback = super::followups::practice_feedback(
            db,
            actor,
            &session,
            &session.input.summary.as_ref().unwrap().feedback,
            &request.change_note,
        )?;
        session.input.summary.as_mut().unwrap().feedback = feedback;
        db.execute(
            "UPDATE prd_sessions SET data=?2 WHERE id=?1",
            params![session.id, json(&session)?],
        )?;
    }
    db.execute(
        "INSERT INTO prd_operations(id,actor_id,request,response) VALUES(?1,?2,?3,?4)",
        params![request.operation_id, actor.id, encoded, json(&session)?],
    )?;
    let revision = Revision {
        session: session.clone(),
        actor_id: actor.id.clone(),
        actor_name: actor.display_name.clone(),
        recorded_at: stamp,
        event: session.status.clone(),
        change_note: request.change_note,
    };
    db.execute("INSERT INTO prd_session_revisions(session_id,version,operation_id,data) VALUES(?1,?2,?3,?4)",params![session.id,session.version as i64,request.operation_id,json(&revision)?])?;
    if let Some(summary) = &session.input.summary {
        for k in &summary.new_knowledge {
            if complete {
                super::reviews::sync(db, "knowledge", &k.id)?;
            }
        }
    }
    Ok(session)
}
fn outputs(db: &dyn Sql, actor: &User, s: &Session, stamp: &str, note: &str) -> Result<()> {
    let summary = s.input.summary.as_ref().unwrap();
    let mut source = summary.clone();
    source.new_knowledge.clear();
    source.knowledge_details = None;
    source.new_actions.clear();
    let existing = super::lessons::list_knowledge(db)?
        .into_iter()
        .filter(|k| k.source_session_id.as_deref() == Some(&s.id))
        .collect::<Vec<_>>();
    if existing
        .iter()
        .filter(|k| !k.deleted)
        .any(|k| !summary.new_knowledge.iter().any(|v| v.id == k.input.id))
    {
        return Err(ApiError::invalid(
            "已沉淀的经验或困难不能随总结静默删除，请保留后到跟进中处理",
        ));
    }
    unique(
        &summary
            .new_knowledge
            .iter()
            .map(|k| k.id.clone())
            .collect::<Vec<_>>(),
    )?;
    for input in &summary.new_knowledge {
        if input.content.trim().is_empty()
            || input.kind == KnowledgeKind::Experience && input.stage.is_none()
        {
            return Err(ApiError::invalid("请填写知识内容并明确经验阶段"));
        }
        super::lessons::bindings(db, &input.goal_ids, &input.method_ids, true)?;
        if let Some(date) = &input.next_review_date {
            valid_date(date)?;
        }
        let old = existing.iter().find(|k| k.input.id == input.id);
        let mut details = old.map(|k| k.details.clone()).unwrap_or_default();
        if let Some(submitted) = summary
            .knowledge_details
            .as_ref()
            .and_then(|values| values.get(&input.id))
        {
            details.adult_response = submitted.adult_response.clone();
            details.guess = submitted.guess.clone();
            details.conditions = submitted.conditions.clone();
        }
        if let Some(k) = old {
            if k.deleted {
                continue;
            }
            if (k.input != *input || k.details != details)
                && summary.knowledge_versions.get(&input.id).copied() != Some(k.version)
            {
                return Err(ApiError::conflict(
                    "经验或困难有独立修订，请核对后再保存总结",
                ));
            }
        }
        if old.is_some_and(|k| k.input == *input && k.details == details) {
            continue;
        }
        if old.is_none()
            && !db
                .query(
                    "SELECT id FROM prd_knowledge WHERE id=?1",
                    params![input.id],
                )?
                .is_empty()
        {
            return Err(ApiError::conflict("知识编号已用于其他来源"));
        }
        let k = Knowledge {
            details,
            derived_from: old.map(|k| k.derived_from.clone()).unwrap_or_default(),
            source_event_id: None,
            source_review_id: None,
            deleted: old.is_some_and(|k| k.deleted),
            audit: Some(super::followups::audit(
                actor,
                &format!("来自干预总结：{note}"),
            )),
            input: input.clone(),
            version: old.map_or(1, |k| k.version + 1),
            source_lesson_id: None,
            source_session_id: Some(s.id.clone()),
            source_session_version: Some(s.version),
            source_observation_version: 0,
            source_observation: None,
            source_summary: Some(source.clone()),
            author_id: old.map_or(actor.id.clone(), |k| k.author_id.clone()),
            author_name: old.map_or(actor.display_name.clone(), |k| k.author_name.clone()),
            created_at: old.map_or(stamp.into(), |k| k.created_at.clone()),
        };
        db.execute("INSERT INTO prd_knowledge(id,source_session_id,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![input.id,s.id,json(&k)?])?;
        revision_output(db, "knowledge", &input.id, k.version, &k)?;
    }
    let existing = super::lessons::list_actions(db)?
        .into_iter()
        .filter(|a| a.source_session_id.as_deref() == Some(&s.id))
        .collect::<Vec<_>>();
    if existing
        .iter()
        .any(|a| !summary.new_actions.iter().any(|v| v.id == a.input.id))
    {
        return Err(ApiError::invalid("已保存的下一步不能随总结静默删除"));
    }
    unique(
        &summary
            .new_actions
            .iter()
            .map(|a| a.id.clone())
            .collect::<Vec<_>>(),
    )?;
    for input in &summary.new_actions {
        if let Some(date) = &input.suggested_date {
            valid_date(date)?;
        }
        if input.content.trim().is_empty() {
            return Err(ApiError::invalid("请填写下一步内容"));
        }
        super::lessons::bindings(db, &input.goal_ids, &input.method_ids, true)?;
        let old = existing.iter().find(|a| a.input.id == input.id);
        if old.is_some_and(|a| json(&a.input).ok() == json(input).ok()) {
            continue;
        }
        let assignee_name = if old.is_some_and(|a| a.input.assignee_id == input.assignee_id) {
            old.and_then(|a| a.assignee_name.clone())
        } else if let Some(id) = &input.assignee_id {
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
        if old.is_none()
            && !db
                .query("SELECT id FROM prd_actions WHERE id=?1", params![input.id])?
                .is_empty()
        {
            return Err(ApiError::conflict("下一步编号已用于其他来源"));
        }
        let a = NextAction {
            audit: Some(super::followups::audit(actor, "明确保存下一步")),
            source_deleted: false,
            source_event_id: None,
            source_review_id: None,
            input: input.clone(),
            version: old.map_or(1, |a| a.version + 1),
            source_lesson_id: None,
            source_session_id: Some(s.id.clone()),
            source_session_version: Some(s.version),
            source_observation_version: 0,
            author_id: old.map_or(actor.id.clone(), |a| a.author_id.clone()),
            author_name: old.map_or(actor.display_name.clone(), |a| a.author_name.clone()),
            created_at: old.map_or(stamp.into(), |a| a.created_at.clone()),
            assignee_name,
            status: old.map_or("pending".into(), |a| a.status.clone()),
            plan_id: old.and_then(|a| a.plan_id.clone()),
        };
        db.execute("INSERT INTO prd_actions(id,source_session_id,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![input.id,s.id,json(&a)?])?;
        revision_output(db, "action", &input.id, a.version, &a)?;
    }
    Ok(())
}
