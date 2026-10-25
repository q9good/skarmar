//! Calendar schedules and atomic reviews. All dates use the family business day.
use super::{
    followups,
    lesson_model::{ExperienceStage, KnowledgeKind},
    model::{GoalState, MethodState, Mutation, Saved},
    process_model::*,
};
use crate::{
    auth::{AccountRole, User},
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_date, valid_id},
};
use chrono::{Datelike, Duration, NaiveDate};
use serde_json::{Value, json as value};
use sha2::Digest;
fn day(s: &str) -> Result<NaiveDate> {
    valid_date(s)?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| ApiError::invalid("日期无效"))
}
fn blank_audit() -> Audit {
    Audit {
        actor_id: String::new(),
        actor_name: "默认安排".into(),
        at: chrono::Utc::now().to_rfc3339(),
        note: "家庭默认周末节奏".into(),
    }
}
pub fn settings(db: &dyn Sql) -> Result<Settings> {
    if let Some(r) = db
        .query("SELECT data FROM prd_settings WHERE id='family'", &[])?
        .first()
    {
        return Ok(serde_json::from_str(&r[0])?);
    }
    let today = day(&super::sessions::today())?;
    let shift = (5 + 7 - today.weekday().num_days_from_monday()) % 7;
    let s = Settings {
        version: 1,
        weekday: 6,
        fortnight_anchor: (today + Duration::days(i64::from(shift))).to_string(),
        monthly_ordinal: 1,
        audit: blank_audit(),
    };
    db.execute(
        "INSERT INTO prd_settings VALUES('family',?1)",
        params![json(&s)?],
    )?;
    Ok(s)
}
pub fn next_date(
    settings: &Settings,
    interval: &str,
    after: &str,
    inclusive: bool,
) -> Result<String> {
    let boundary = day(after)? + Duration::days(if inclusive { 0 } else { 1 });
    match interval {
        "weekly" => {
            let shift = (settings.weekday - 1 + 7 - boundary.weekday().num_days_from_monday()) % 7;
            Ok((boundary + Duration::days(i64::from(shift))).to_string())
        }
        "fortnightly" => {
            let anchor = day(&settings.fortnight_anchor)?;
            if boundary <= anchor {
                return Ok(anchor.to_string());
            }
            let delta = (boundary - anchor).num_days();
            Ok((anchor + Duration::days(((delta + 13) / 14) * 14)).to_string())
        }
        "monthly" => {
            let mut first = NaiveDate::from_ymd_opt(boundary.year(), boundary.month(), 1).unwrap();
            loop {
                let next = if first.month() == 12 {
                    NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
                }
                .ok_or_else(|| ApiError::invalid("日期超出范围"))?;
                let candidate = if settings.monthly_ordinal == -1 {
                    let last = next - Duration::days(1);
                    last - Duration::days(i64::from(
                        (last.weekday().num_days_from_monday() + 7 - (settings.weekday - 1)) % 7,
                    ))
                } else {
                    let shift =
                        (settings.weekday - 1 + 7 - first.weekday().num_days_from_monday()) % 7;
                    first
                        + Duration::days(
                            i64::from(shift) + 7 * i64::from(settings.monthly_ordinal - 1),
                        )
                };
                if candidate >= boundary {
                    return Ok(candidate.to_string());
                }
                first = next;
            }
        }
        _ => Err(ApiError::invalid("复盘周期无效")),
    }
}
pub fn schedules(db: &dyn Sql) -> Result<Vec<Schedule>> {
    db.query(
        "SELECT data FROM prd_schedules ORDER BY object_kind,object_id",
        &[],
    )?
    .into_iter()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .collect()
}
pub fn schedule(db: &dyn Sql, kind: &str, id: &str) -> Result<Schedule> {
    db.query(
        "SELECT data FROM prd_schedules WHERE object_kind=?1 AND object_id=?2",
        params![kind, id],
    )?
    .first()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .transpose()?
    .ok_or_else(ApiError::missing)
}
fn save_schedule(db: &dyn Sql, s: &Schedule) -> Result<()> {
    db.execute("INSERT INTO prd_schedules(object_kind,object_id,version,data) VALUES(?1,?2,?3,?4) ON CONFLICT(object_kind,object_id) DO UPDATE SET version=excluded.version,data=excluded.data",params![s.object_kind,s.object_id,s.version as i64,json(s)?])?;
    db.execute(
        "INSERT INTO prd_schedule_revisions VALUES(?1,?2,?3,?4)",
        params![s.object_kind, s.object_id, s.version as i64, json(s)?],
    )?;
    Ok(())
}
pub fn seed(db: &dyn Sql) -> Result<()> {
    settings(db)?;
    for row in db.query("SELECT data FROM prd_entities ORDER BY id", &[])? {
        let s: Saved = serde_json::from_str(&row[0])?;
        match s {
            Saved::Goal(g) => sync(db, "goal", &g.id)?,
            Saved::Method(m) => sync(db, "method", &m.id)?,
            _ => {}
        }
    }
    for k in super::lessons::list_knowledge(db)? {
        sync(db, "knowledge", &k.input.id)?;
    }
    Ok(())
}
fn profile(db: &dyn Sql, kind: &str, id: &str) -> Result<(bool, String, Option<String>, String)> {
    match kind {
        "goal" => {
            let Some(Saved::Goal(g)) = super::get(db, id)? else {
                return Err(ApiError::missing());
            };
            let active = !g.input.archived
                && matches!(
                    g.input.state,
                    Some(GoalState::Active | GoalState::Maintaining) | None
                );
            Ok((
                active,
                if g.input.level == crate::model::Level::Long {
                    "monthly"
                } else {
                    "fortnightly"
                }
                .into(),
                if g.input.state == Some(GoalState::Paused) && !g.input.archived {
                    g.input.resume_on.clone()
                } else {
                    None
                },
                format!(
                    "{:?}:{}:{:?}",
                    g.input.state, g.input.archived, g.input.resume_on
                ),
            ))
        }
        "method" => {
            let Some(Saved::Method(m)) = super::get(db, id)? else {
                return Err(ApiError::missing());
            };
            Ok((
                matches!(m.input.state, Some(MethodState::Active) | None),
                "monthly".into(),
                if m.input.state == Some(MethodState::Paused) {
                    m.input.resume_on.clone()
                } else {
                    None
                },
                format!("{:?}:{:?}", m.input.state, m.input.resume_on),
            ))
        }
        "knowledge" => {
            let k = followups::get(db, id)?;
            let state = followups::effective_state(&k);
            let active = !k.deleted
                && if k.input.kind == KnowledgeKind::Difficulty {
                    ["pending", "following"].contains(&state.as_str())
                } else {
                    state == "active"
                        && matches!(
                            k.input.stage,
                            Some(ExperienceStage::Try | ExperienceStage::Helpful)
                        )
                };
            let interval = if k.input.kind == KnowledgeKind::Difficulty {
                if k.details.priority {
                    "weekly"
                } else {
                    "fortnightly"
                }
            } else {
                "monthly"
            };
            let manual = if state == "paused" {
                k.details.pause_until.clone()
            } else if active {
                k.input.next_review_date.clone()
            } else {
                None
            };
            Ok((
                active,
                interval.into(),
                manual,
                format!(
                    "{}:{}:{:?}:{}:{:?}:{:?}",
                    state,
                    k.details.priority,
                    k.input.stage,
                    k.deleted,
                    k.input.next_review_date,
                    k.details.pause_until
                ),
            ))
        }
        _ => Err(ApiError::invalid("复盘对象类型无效")),
    }
}
pub fn sync(db: &dyn Sql, kind: &str, id: &str) -> Result<()> {
    if kind == "catalog" {
        return Ok(());
    }
    let (active, interval, manual, signature) = profile(db, kind, id)?;
    let old = db
        .query(
            "SELECT data FROM prd_schedules WHERE object_kind=?1 AND object_id=?2",
            params![kind, id],
        )?
        .first()
        .map(|r| serde_json::from_str::<Schedule>(&r[0]).map_err(ApiError::from))
        .transpose()?;
    if old.as_ref().is_some_and(|s| s.signature == signature) {
        return Ok(());
    }
    let settings = settings(db)?;
    let initial = old.is_none();
    let mut s = old.unwrap_or(Schedule {
        object_kind: kind.into(),
        object_id: id.into(),
        version: 0,
        generation: 0,
        due_date: None,
        interval: interval.clone(),
        manual: false,
        reminders: true,
        eligible: active,
        mode: "regular".into(),
        signature: String::new(),
        last_completed: None,
        note: String::new(),
    });
    s.version += 1;
    s.generation += 1;
    s.eligible = active;
    s.interval = interval.clone();
    s.signature = signature;
    s.mode = if !active && manual.is_some() {
        "resume"
    } else {
        "regular"
    }
    .into();
    s.due_date = if let Some(d) = manual {
        valid_date(&d)?;
        s.manual = true;
        Some(d)
    } else if active {
        s.manual = false;
        Some(next_date(
            &settings,
            &interval,
            &super::sessions::today(),
            initial,
        )?)
    } else {
        None
    };
    s.note = "对象状态或回看安排已更新".into();
    save_schedule(db, &s)
}
pub fn write_settings(db: &dyn Sql, actor: &User, r: SettingsWrite) -> Result<Value> {
    let old = settings(db)?;
    if old.version != r.expected_version {
        return Err(ApiError::conflict("家庭复盘设置已有更新，请核对"));
    }
    if ![6, 7].contains(&r.weekday)
        || ![-1, 1, 2, 3, 4].contains(&r.monthly_ordinal)
        || day(&r.fortnight_anchor)?.weekday().num_days_from_monday() + 1 != r.weekday
    {
        return Err(ApiError::invalid(
            "请选择周六／周日、月内序次，隔周锚点须为所选周末日",
        ));
    }
    let s = Settings {
        version: old.version + 1,
        weekday: r.weekday,
        fortnight_anchor: r.fortnight_anchor,
        monthly_ordinal: r.monthly_ordinal,
        audit: followups::audit(actor, "修改家庭复盘日程"),
    };
    db.execute(
        "UPDATE prd_settings SET data=?1 WHERE id='family'",
        params![json(&s)?],
    )?;
    for mut schedule in schedules(db)? {
        if schedule.manual
            || schedule.mode == "resume"
            || !schedule.eligible
            || schedule
                .due_date
                .as_ref()
                .is_none_or(|d| d <= &super::sessions::today())
                && !r.reorder_overdue
        {
            continue;
        }
        schedule.version += 1;
        schedule.generation += 1;
        schedule.due_date = Some(next_date(
            &s,
            &schedule.interval,
            &super::sessions::today(),
            schedule.last_completed.is_none(),
        )?);
        schedule.note = "随家庭设置重排自动日期".into();
        save_schedule(db, &schedule)?;
    }
    Ok(serde_json::to_value(s)?)
}
fn object_version(db: &dyn Sql, kind: &str, id: &str) -> Result<u64> {
    if kind == "knowledge" {
        Ok(followups::get(db, id)?.version)
    } else {
        super::get(db, id)?
            .map(|s| s.version())
            .ok_or_else(ApiError::missing)
    }
}
pub fn write_review(db: &dyn Sql, actor: &User, r: ReviewWrite) -> Result<Value> {
    if r.caregiver_state
        .as_ref()
        .is_some_and(|s| !["", "easy", "okay", "stretched"].contains(&s.as_str()))
    {
        return Err(ApiError::invalid("家长状态可留空或选择轻松／还行／吃力"));
    }
    if r.action == "defer_group" {
        if r.group_tokens.is_empty() || r.group_tokens.len() > 50 {
            return Err(ApiError::invalid("请明确选择1至50个无新证据事项"));
        }
        let mut seen = std::collections::HashSet::new();
        let mut changed = vec![];
        for token in &r.group_tokens {
            if !seen.insert(format!("{}:{}", token.object_kind, token.object_id)) {
                return Err(ApiError::invalid("顺延组内对象不能重复"));
            }
            let c = context(db, &token.object_kind, &token.object_id, 0)?;
            if c["total"].as_u64() != Some(0) {
                return Err(ApiError::conflict("组内已有新证据，请逐项查看后决定"));
            }
            let mut item = r.clone();
            item.action = "defer".into();
            item.object_kind = token.object_kind.clone();
            item.object_id = token.object_id.clone();
            item.object_version = token.object_version;
            item.schedule_version = token.schedule_version;
            item.generation = token.generation;
            item.due_date = token.due_date.clone();
            item.group_tokens.clear();
            changed.push(write_review(db, actor, item)?);
        }
        return Ok(value!({"deferred_group":changed.len(),"items":changed}));
    }
    valid_id(&r.id)?;
    valid_id(&r.object_id)?;
    sync(db, &r.object_kind, &r.object_id)?;
    if r.action == "revise" || r.action == "delete" {
        let mut existing: Review = db
            .query("SELECT data FROM prd_reviews WHERE id=?1", params![r.id])?
            .first()
            .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
            .transpose()?
            .ok_or_else(ApiError::missing)?;
        if Some(existing.version) != r.expected_version
            || existing.object_id != r.object_id
            || existing.object_kind != r.object_kind
            || existing.deleted
        {
            return Err(ApiError::conflict("复盘记录已有修订，请核对"));
        }
        if r.action == "delete" {
            if actor.role != AccountRole::Admin {
                return Err(ApiError(
                    axum::http::StatusCode::FORBIDDEN,
                    "只有管理员可删除误建复盘".into(),
                ));
            }
            existing.deleted = true;
        } else {
            if r.note.trim().is_empty() {
                return Err(ApiError::invalid("请说明本次更正，不会再次推进日程"));
            }
            if r.decision != existing.decision {
                return Err(ApiError::invalid(
                    "历史复盘的状态决定不能暗中改写，请执行一次明确的新决定",
                ));
            }
            existing.note = r.note.clone();
            if let Some(state) = &r.caregiver_state {
                existing.caregiver_state = if state.is_empty() {
                    None
                } else {
                    Some(state.clone())
                };
            }
            existing.evidence = r.evidence.clone();
            existing.evidence_snapshots = r
                .evidence
                .iter()
                .map(|s| followups::source(db, s))
                .collect::<Result<_>>()?;
        }
        existing.version += 1;
        existing.audit = followups::audit(actor, &r.note);
        db.execute(
            "UPDATE prd_reviews SET version=?2,data=?3 WHERE id=?1",
            params![existing.id, existing.version as i64, json(&existing)?],
        )?;
        db.execute(
            "INSERT INTO prd_review_revisions VALUES(?1,?2,?3)",
            params![existing.id, existing.version as i64, json(&existing)?],
        )?;
        return Ok(serde_json::to_value(existing)?);
    }
    let mut schedule = schedule(db, &r.object_kind, &r.object_id)?;
    if schedule.version != r.schedule_version
        || schedule.generation != r.generation
        || schedule.due_date != r.due_date
        || object_version(db, &r.object_kind, &r.object_id)? != r.object_version
    {
        return Err(ApiError::conflict(
            "对象或本期复盘安排已更新，请核对另一位家人的结果",
        ));
    }
    if let Some(date) = &r.next_date {
        valid_date(date)?;
        if date <= &super::sessions::today() {
            return Err(ApiError::invalid("下次回看日期须晚于今天"));
        }
    }
    if r.action == "defer" {
        let date = r
            .next_date
            .ok_or_else(|| ApiError::invalid("顺延时请选择下次日期"))?;
        schedule.version += 1;
        schedule.generation += 1;
        schedule.due_date = Some(date);
        schedule.manual = true;
        schedule.reminders = r.reminders;
        schedule.note = format!("{}顺延了此项（不计完成复盘）", actor.display_name);
        save_schedule(db, &schedule)?;
        return Ok(value!({"deferred":true,"schedule":schedule}));
    }
    if r.action != "complete" || r.expected_version.is_some() {
        return Err(ApiError::invalid("请选择完成本期复盘或顺延"));
    }
    if ![
        "continue", "adjust", "pause", "restore", "maintain", "achieve", "resolve", "reopen",
    ]
    .contains(&r.decision.as_str())
    {
        return Err(ApiError::invalid("复盘决定无效"));
    }
    if r.decision != "continue" && r.note.trim().is_empty() {
        return Err(ApiError::invalid("请写下改变决定的依据或原因"));
    }
    if r.evidence.len() > 100
        || r.evidence
            .iter()
            .map(|s| format!("{}:{}", s.kind, s.id))
            .collect::<std::collections::HashSet<_>>()
            .len()
            != r.evidence.len()
    {
        return Err(ApiError::invalid("一次最多引用100条不同依据"));
    }
    let snapshots = r
        .evidence
        .iter()
        .map(|s| followups::source(db, s))
        .collect::<Result<Vec<_>>>()?;
    let current_context = context(db, &r.object_kind, &r.object_id, 0)?;
    if r.seen_versions.len() > 10000
        || r.seen_versions
            .iter()
            .any(|(key, v)| current_context["_all_versions"][key].as_u64() != Some(*v))
    {
        return Err(ApiError::conflict("所查看的复盘依据已有修订，请核对后保存"));
    }
    if last_review(db, &r.object_kind, &r.object_id)?
        .is_some_and(|old| old.completed_on == super::sessions::today())
        && current_context["total"].as_u64() == Some(0)
        && r.decision == "continue"
        && r.next_actions.is_empty()
    {
        return Err(ApiError::conflict(
            "今天已完成回看且暂无新证据；可顺延，或明确调整、补充下一步，无需重复确认继续",
        ));
    }
    let old_actions = last_review(db, &r.object_kind, &r.object_id)?
        .map(|r| r.action_ids)
        .unwrap_or_default();
    let review = Review {
        caregiver_state: r.caregiver_state.clone().filter(|s| !s.is_empty()),
        id: r.id.clone(),
        version: 1,
        object_kind: r.object_kind.clone(),
        object_id: r.object_id.clone(),
        generation: schedule.generation,
        due_date: schedule
            .due_date
            .clone()
            .unwrap_or_else(super::sessions::today),
        completed_on: super::sessions::today(),
        decision: r.decision.clone(),
        note: r.note.clone(),
        evidence: r.evidence.clone(),
        evidence_snapshots: snapshots,
        previous_actions: old_actions,
        seen_versions: {
            let mut seen = last_review(db, &r.object_kind, &r.object_id)?
                .map(|r| r.seen_versions)
                .unwrap_or_default();
            seen.extend(r.seen_versions.clone());
            seen
        },
        action_ids: r.next_actions.iter().map(|a| a.id.clone()).collect(),
        audit: followups::audit(actor, &r.note),
        deleted: false,
    };
    if !db
        .query("SELECT id FROM prd_reviews WHERE id=?1", params![r.id])?
        .is_empty()
    {
        return Err(ApiError::conflict("复盘编号已存在"));
    }
    db.execute("INSERT INTO prd_reviews(id,object_kind,object_id,generation,due_date,version,data) VALUES(?1,?2,?3,?4,?5,1,?6)",params![review.id,review.object_kind,review.object_id,review.generation as i64,review.due_date,json(&review)?])?;
    db.execute(
        "INSERT INTO prd_review_revisions VALUES(?1,1,?2)",
        params![review.id, json(&review)?],
    )?;
    apply_decision(db, actor, &r)?;
    followups::create_actions(db, actor, &r.next_actions, None, Some(&review.id))?;
    // Object writes may have synchronized the schedule; consume the original token once.
    schedule = schedule_after(db, &r.object_kind, &r.object_id)?;
    schedule.version += 1;
    schedule.generation += 1;
    schedule.last_completed = Some(super::sessions::today());
    schedule.reminders = r.reminders;
    let (active, interval, manual, signature) = profile(db, &r.object_kind, &r.object_id)?;
    schedule.eligible = active;
    schedule.signature = signature;
    schedule.interval = interval.clone();
    schedule.mode = if r.decision == "pause" && r.next_date.is_some() {
        "resume"
    } else {
        "regular"
    }
    .into();
    schedule.due_date = if !r.reminders || !active && r.decision != "pause" {
        None
    } else if let Some(date) = &r.next_date {
        schedule.manual = true;
        Some(date.clone())
    } else if active {
        schedule.manual = false;
        Some(next_date(
            &settings(db)?,
            &interval,
            &super::sessions::today(),
            false,
        )?)
    } else {
        manual
    };
    schedule.note = format!("{}已完成本期复盘", actor.display_name);
    save_schedule(db, &schedule)?;
    Ok(value!({"review":review,"schedule":schedule}))
}
fn schedule_after(db: &dyn Sql, kind: &str, id: &str) -> Result<Schedule> {
    schedule(db, kind, id)
}
fn apply_decision(db: &dyn Sql, actor: &User, r: &ReviewWrite) -> Result<()> {
    if r.object_kind == "knowledge" {
        let mut k = followups::get(db, &r.object_id)?;
        if k.deleted {
            return Err(ApiError::conflict("经验或困难已删除"));
        }
        let state = followups::effective_state(&k);
        let next = match (k.input.kind.clone(), r.decision.as_str()) {
            (_, "continue") => state,
            (_, "adjust") => {
                k.details.comparison_start = Some(super::sessions::today());
                if !r.change_text.trim().is_empty() {
                    k.details.conditions = r.change_text.clone();
                }
                state
            }
            (_, "pause") => "paused".into(),
            (KnowledgeKind::Difficulty, "resolve") => {
                k.details.evidence = r.note.clone();
                "resolved".into()
            }
            (KnowledgeKind::Difficulty, "reopen") => {
                k.details.comparison_start = Some(super::sessions::today());
                "following".into()
            }
            (KnowledgeKind::Difficulty, "restore")
                if ["pending", "following"].contains(&r.state.as_str()) =>
            {
                r.state.clone()
            }
            (KnowledgeKind::Experience, "restore") if r.state == "active" => "active".into(),
            _ => return Err(ApiError::invalid("此复盘决定不适用于该经验或困难")),
        };
        if r.decision != "continue" {
            k.details.state = next;
            k.details.pause_until = if r.decision == "pause" {
                r.next_date.clone()
            } else {
                None
            };
            k.input.next_review_date = None;
            k.version += 1;
            k.audit = Some(followups::audit(actor, &format!("复盘：{}", r.note)));
            followups::persist(db, &k)?;
        }
    } else {
        let old = super::get(db, &r.object_id)?.ok_or_else(ApiError::missing)?;
        let version = old.version();
        let mutation = match old {
            Saved::Goal(mut g) => {
                let next = match r.decision.as_str() {
                    "continue" | "adjust" => g.input.state.clone(),
                    "pause" => Some(GoalState::Paused),
                    "maintain" => Some(GoalState::Maintaining),
                    "achieve" if g.input.level != crate::model::Level::Long => {
                        Some(GoalState::Achieved)
                    }
                    "restore" => Some(match r.state.as_str() {
                        "active" => GoalState::Active,
                        "maintaining" => GoalState::Maintaining,
                        "not_started" if g.input.level != crate::model::Level::Long => {
                            GoalState::NotStarted
                        }
                        _ => return Err(ApiError::invalid("请明确恢复后的目标状态")),
                    }),
                    _ => return Err(ApiError::invalid("此决定不适用于该目标")),
                };
                g.input.state = next;
                g.input.resume_on = if r.decision == "pause" {
                    r.next_date.clone()
                } else {
                    None
                };
                if r.decision == "adjust" && !r.change_text.trim().is_empty() {
                    g.input.criteria = r.change_text.clone();
                }
                Mutation::Goal(g.input)
            }
            Saved::Method(mut m) => {
                m.input.state = match r.decision.as_str() {
                    "continue" | "adjust" => m.input.state,
                    "pause" => Some(MethodState::Paused),
                    "restore" if r.state == "active" => Some(MethodState::Active),
                    _ => return Err(ApiError::invalid("此决定不适用于该方法")),
                };
                m.input.resume_on = if r.decision == "pause" {
                    r.next_date.clone()
                } else {
                    None
                };
                if r.decision == "adjust" && !r.change_text.trim().is_empty() {
                    m.input.steps = r.change_text.clone();
                    m.input.description.clear();
                }
                Mutation::Method(m.input)
            }
            _ => return Err(ApiError::invalid("此对象无法复盘")),
        };
        if r.decision != "continue" {
            super::write(
                db,
                actor,
                super::model::Write {
                    provided_fields: None,
                    method_change: (r.decision == "adjust"
                        && matches!(&mutation, Mutation::Method(_)))
                    .then(|| "adjustment".into()),
                    method_create_mode: None,
                    id: r.object_id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    expected_version: Some(version),
                    change_note: format!("复盘：{}", r.note),
                    mutation,
                },
            )?;
        }
    }
    Ok(())
}
fn last_review(db: &dyn Sql, kind: &str, id: &str) -> Result<Option<Review>> {
    for row in db.query(
        "SELECT data FROM prd_reviews WHERE object_kind=?1 AND object_id=?2 ORDER BY rowid DESC",
        params![kind, id],
    )? {
        let r: Review = serde_json::from_str(&row[0])?;
        if !r.deleted {
            return Ok(Some(r));
        }
    }
    Ok(None)
}
pub fn context(db: &dyn Sql, kind: &str, id: &str, page: u32) -> Result<Value> {
    sync(db, kind, id)?;
    let s = schedule(db, kind, id)?;
    let version = object_version(db, kind, id)?;
    let previous = last_review(db, kind, id)?;
    let prior = previous
        .as_ref()
        .map(|r| r.seen_versions.clone())
        .unwrap_or_default();
    let mut seen_versions = std::collections::BTreeMap::<String, u64>::new();
    let from = previous
        .as_ref()
        .map(|r| r.completed_on.clone())
        .unwrap_or_else(|| "0001-01-01".into());
    let today = super::sessions::today();
    let mut evidence = vec![];
    let (mut family, mut lessons) = (0, 0);
    if kind == "knowledge" {
        let k = followups::get(db, id)?;
        for e in followups::events(db, id)?
            .into_iter()
            .filter(|e| !e.deleted && e.input.business_date <= today)
        {
            let key = format!("feedback:{}", e.id);
            seen_versions.insert(key.clone(), e.version);
            let mut notes = vec![];
            let mut fresh_annotation = false;
            for row in db.query(
                "SELECT data FROM prd_annotations WHERE event_id=?1",
                params![e.id],
            )? {
                let note: Annotation = serde_json::from_str(&row[0])?;
                let key = format!("annotation:{}", note.id);
                seen_versions.insert(key.clone(), 1);
                if prior.get(&key).copied().unwrap_or(0) < 1 {
                    fresh_annotation = true;
                }
                notes.push(format!("{}的附注：{}", note.audit.actor_name, note.text));
            }
            if prior.get(&key).copied().unwrap_or(0) >= e.version && !fresh_annotation {
                continue;
            }
            let observation = if notes.is_empty() {
                e.input.observation.clone()
            } else {
                format!("{}；{}", e.input.observation, notes.join("；"))
            };
            let changes = match e.input.source.kind.as_str() {
                "session" => serde_json::from_value::<super::session_model::Session>(
                    e.source_snapshot.clone(),
                )
                .map(|s| super::review_changes::session(&s, &k.input.goal_ids, &k.input.method_ids))
                .unwrap_or_default(),
                "lesson" => {
                    let mut l: super::lesson_model::Lesson =
                        serde_json::from_value(e.source_snapshot.clone())?;
                    if let Some(v) = e.source_snapshot.pointer("/referenced_observation/value") {
                        l.observation = serde_json::from_value(v.clone())?;
                    }
                    super::review_changes::lesson(&l)
                }
                _ => vec![],
            };
            evidence.push(value!({"changes":changes,"source":{"kind":"feedback","id":e.id,"version":e.version},"business_date":e.input.business_date,"observation":observation,"adult":e.input.adult_response,"conditions":e.input.conditions,"result":e.input.result,"signals":[]}));
        }
        evidence.sort_by(|a, b| {
            b["business_date"]
                .as_str()
                .cmp(&a["business_date"].as_str())
                .then_with(|| a["source"]["id"].as_str().cmp(&b["source"]["id"].as_str()))
        });
        return finish_context(
            db,
            page,
            value!({"schedule":s,"object_version":version,"knowledge":k,"from":from,"to":today,"total":evidence.len(),"evidence":evidence.iter().skip(page as usize*30).take(30).collect::<Vec<_>>(),"seen_versions":seen_versions,"summary":followups::summary(db,&k)?,"last_review":last_review(db,kind,id)?}),
        );
    }
    for session in super::sessions::list(db)? {
        if session.status != "completed" || session.input.business_date > today {
            continue;
        }
        let matched = if kind == "method" {
            session.actual_methods.iter().any(|m| m.id == id)
        } else {
            session
                .actual_targets
                .iter()
                .any(|t| t.goal.id == id || t.ancestors.iter().any(|g| g.id == id))
                || session.focus_goals.iter().any(|g| g.id == id)
        };
        if !matched {
            continue;
        }
        let key = format!("session:{}", session.id);
        seen_versions.insert(key.clone(), session.version);
        if prior.get(&key).copied().unwrap_or(0) >= session.version {
            continue;
        }
        family += 1;
        let summary = session.input.summary.as_ref().unwrap();
        let signals: Vec<_> = summary
            .signals
            .iter()
            .filter(|s| {
                if kind == "method" {
                    s.method_ids.iter().any(|v| v == id)
                } else {
                    s.goal_ids.iter().any(|v| v == id)
                }
            })
            .collect();
        let goal_scope = if kind == "goal" {
            vec![id.to_string()]
        } else {
            vec![]
        };
        let method_scope = if kind == "method" {
            vec![id.to_string()]
        } else {
            vec![]
        };
        let changes = super::review_changes::session(&session, &goal_scope, &method_scope);
        evidence.push(value!({"changes":changes,"source":{"kind":"session","id":session.id,"version":session.version},"business_date":session.input.business_date,"observation":summary.targets.iter().filter(|t|t.performed).map(|t|t.observation.clone()).collect::<Vec<_>>().join("；"),"adult":session.actual_methods.iter().map(|m|if m.input.description.is_empty(){m.input.steps.clone()}else{m.input.description.clone()}).collect::<Vec<_>>().join("；"),"conditions":summary.context,"signals":signals,"first_try_ids":summary.first_try_ids}));
    }
    for l in super::lessons::list_lessons(db)? {
        if l.status != "recorded" || l.observation.business_date > today {
            continue;
        }
        let matched = if kind == "method" {
            l.organization.methods.iter().any(|m| m.id == id)
        } else {
            l.organization
                .targets
                .iter()
                .any(|t| t.goal.id == id || t.ancestors.iter().any(|g| g.id == id))
                || l.organization.focus_goals.iter().any(|g| g.id == id)
        };
        if !matched {
            continue;
        }
        let key = format!("lesson:{}", l.id);
        seen_versions.insert(key.clone(), l.version);
        if prior.get(&key).copied().unwrap_or(0) >= l.version {
            continue;
        }
        lessons += 1;
        let changes = super::review_changes::lesson(&l);
        evidence.push(value!({"changes":changes,"source":{"kind":"lesson","id":l.id,"version":l.version},"business_date":l.observation.business_date,"observation":l.observation.notes,"adult":l.observation.teacher_notes,"conditions":l.observation.difficulty_notes,"signals":[]}));
    }
    if kind == "goal" {
        for typ in ["moment", "measurement"] {
            for record in super::observations::list(db, typ)? {
                if record.deleted {
                    continue;
                }
                let day = record.input["business_date"].as_str().unwrap_or("");
                let matched = if typ == "moment" {
                    record.input["goal_ids"]
                        .as_array()
                        .is_some_and(|v| v.iter().any(|g| g == id))
                } else {
                    record.goal.as_ref().is_some_and(|g| g.id == id)
                };
                if !matched || day > today.as_str() {
                    continue;
                }
                let key = format!("{typ}:{}", record.id);
                seen_versions.insert(key.clone(), record.version);
                if prior.get(&key).copied().unwrap_or(0) >= record.version {
                    continue;
                }
                let observation = if typ == "moment" {
                    record.input["text"].as_str().unwrap_or("").to_owned()
                } else {
                    let suffix = record.input["text"].as_str().unwrap_or("");
                    if record.input["opportunities"].is_number() {
                        format!(
                            "{} 次机会，{} 次做到；{}",
                            record.input["opportunities"], record.input["achieved"], suffix
                        )
                    } else if record.input["amount"].is_number() {
                        format!(
                            "实际 {} {}；{}",
                            record.input["amount"],
                            record
                                .metric
                                .as_ref()
                                .and_then(|m| m.input["unit"].as_str())
                                .unwrap_or(""),
                            suffix
                        )
                    } else {
                        suffix.to_owned()
                    }
                };
                evidence.push(value!({"source":{"kind":typ,"id":record.id,"version":record.version},"business_date":day,"observation":observation,"adult":"","conditions":record.input["support"].as_str().unwrap_or(""),"signals":[]}));
            }
        }
    }
    evidence.sort_by(|a, b| {
        b["business_date"]
            .as_str()
            .cmp(&a["business_date"].as_str())
    });
    finish_context(
        db,
        page,
        value!({"schedule":s,"object_version":version,"from":from,"to":today,"seen_versions":seen_versions,"family":family,"lesson":lessons,"total":evidence.len(),"evidence":evidence.iter().skip(page as usize*30).take(30).collect::<Vec<_>>(),"last_review":last_review(db,kind,id)?}),
    )
}

fn finish_context(db: &dyn Sql, page: u32, mut value: Value) -> Result<Value> {
    let settings = settings(db)?;
    let today = super::sessions::today();
    let interval = value["schedule"]["interval"].as_str().unwrap_or("monthly");
    value["next_defaults"] = value!({"weekly":next_date(&settings,"weekly",&today,false)?,"fortnightly":next_date(&settings,"fortnightly",&today,false)?,"monthly":next_date(&settings,"monthly",&today,false)?,"current":if value["schedule"]["eligible"]==true{Some(next_date(&settings,interval,&today,false)?)}else{None}});
    let from = value["from"].as_str().unwrap_or("0001-01-01").to_owned();
    let all = value["seen_versions"].clone();
    let mut visible = serde_json::Map::new();
    for evidence in value["evidence"]
        .as_array_mut()
        .ok_or_else(ApiError::storage)?
    {
        let mut item_versions = serde_json::Map::new();
        let kind = evidence["source"]["kind"].as_str().unwrap_or("");
        let id = evidence["source"]["id"].as_str().unwrap_or("");
        let key = format!("{kind}:{id}");
        if let Some(v) = all.get(&key) {
            visible.insert(key, v.clone());
            item_versions.insert(format!("{kind}:{id}"), v.clone());
        }
        if kind == "feedback" {
            for row in db.query(
                "SELECT id FROM prd_annotations WHERE event_id=?1",
                params![id],
            )? {
                let key = format!("annotation:{}", row[0]);
                if let Some(v) = all.get(&key) {
                    visible.insert(key.clone(), v.clone());
                    item_versions.insert(key, v.clone());
                }
            }
        }
        evidence["carried_over"] = evidence["business_date"]
            .as_str()
            .is_some_and(|d| d < from.as_str())
            .into();
        evidence["seen_versions"] = Value::Object(item_versions);
    }
    value["evidence_signature"] =
        format!("{:x}", sha2::Sha256::digest(serde_json::to_vec(&all)?)).into();
    value["has_more"] = (u64::from(page) * 30
        + (value["evidence"].as_array().map_or(0, |v| v.len()) as u64)
        < value["total"].as_u64().unwrap_or(0))
    .into();
    value["page"] = page.into();
    value["_all_versions"] = all;
    value["seen_versions"] = Value::Object(visible);
    Ok(value)
}
