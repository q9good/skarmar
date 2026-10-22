//! Deletion and completed-session revision rules shared by both SQL runtimes.
use crate::{
    model::*,
    sql::{Sql, params},
    store::{self, ApiError, Result},
};
use serde_json::Value;
use uuid::Uuid;

pub fn deleted(db: &dyn Sql) -> Result<Vec<DeletedRecord>> {
    db.query("SELECT kind,id FROM deleted_records ORDER BY kind,id", &[])?
        .into_iter()
        .map(|row| {
            Ok(DeletedRecord {
                kind: row[0].clone(),
                id: row[1].clone(),
            })
        })
        .collect()
}

pub(crate) fn ensure_not_deleted(db: &dyn Sql, kind: &str, id: &str) -> Result<()> {
    if !db
        .query(
            "SELECT id FROM deleted_records WHERE kind=?1 AND id=?2",
            params![kind, id],
        )?
        .is_empty()
    {
        return Err(ApiError::conflict(
            "记录已由管理员删除，旧草稿不能重新保存；如需新增请创建另一条记录",
        ));
    }
    Ok(())
}

pub fn record_session_revision(db: &dyn Sql, session: &Session, note: &str) -> Result<()> {
    let revision = SessionRevision {
        session: session.clone(),
        note: note.into(),
        recorded_at: chrono::Utc::now().to_rfc3339(),
    };
    db.execute(
        "INSERT OR IGNORE INTO session_revisions(session_id,version,data) VALUES (?1,?2,?3)",
        params![session.id, session.version as i64, store::json(&revision)?],
    )
}

pub fn session_history(db: &dyn Sql, id: &str) -> Result<Vec<SessionRevision>> {
    ensure_not_deleted(db, "sessions", id)?;
    let current: Session = store::get(db, "sessions", id)?.ok_or_else(ApiError::missing)?;
    let mut rows: Vec<SessionRevision> = db
        .query(
            "SELECT data FROM session_revisions WHERE session_id=?1 ORDER BY version DESC",
            params![id],
        )?
        .into_iter()
        .map(|row| Ok(serde_json::from_str(&row[0])?))
        .collect::<Result<_>>()?;
    if !rows
        .iter()
        .any(|row| row.session.version == current.version)
    {
        rows.insert(
            0,
            SessionRevision {
                session: current,
                note: "现有记录".into(),
                recorded_at: String::new(),
            },
        );
    }
    Ok(rows)
}

pub fn delete_record(
    db: &dyn Sql,
    kind: &str,
    id: &str,
    request: DeleteRecord,
    actor: &str,
) -> Result<()> {
    if ![
        "goals",
        "pathways",
        "experiences",
        "difficulties",
        "sessions",
        "lessons",
    ]
    .contains(&kind)
    {
        return Err(ApiError::invalid("不支持删除此类型记录"));
    }
    store::valid_id(id)?;
    let prior = db.query(
        "SELECT CAST(version AS TEXT) FROM deleted_records WHERE kind=?1 AND id=?2",
        params![kind, id],
    )?;
    if let Some(row) = prior.first() {
        return if row[0] == request.expected_version.to_string() {
            Ok(())
        } else {
            Err(ApiError::conflict("删除版本不一致，请刷新"))
        };
    }
    let data: Value = store::get(db, kind, id)?.ok_or_else(ApiError::missing)?;
    if data["version"].as_u64() != Some(request.expected_version) {
        return Err(ApiError::conflict("记录已有新版本，请刷新核对后再删除"));
    }
    if kind == "goals" {
        if !db
            .query("SELECT id FROM goals WHERE parent_id=?1", params![id])?
            .is_empty()
        {
            return Err(ApiError::conflict(
                "目标仍有下级目标，请先修改下级关联或删除下级目标",
            ));
        }
        if !db
            .query(
                "SELECT session_id FROM session_targets WHERE goal_id=?1",
                params![id],
            )?
            .is_empty()
        {
            return Err(ApiError::conflict(
                "目标仍被训练引用，请先修改或删除关联训练；停止使用可选择归档",
            ));
        }
        if !db
            .query(
                "SELECT lesson_id FROM lesson_targets WHERE goal_id=?1",
                params![id],
            )?
            .is_empty()
        {
            return Err(ApiError::conflict(
                "目标仍被上课记录引用，请先修改或删除关联上课记录；停止使用可选择归档",
            ));
        }
    }
    db.execute("INSERT INTO deleted_records(kind,id,version,data,deleted_at,deleted_by,source_session_id) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![kind,id,request.expected_version as i64,store::json(&data)?,chrono::Utc::now().to_rfc3339(),actor,data["source_session_id"].as_str().map(str::to_owned)])?;

    // Remove live bindings without deleting any other business entity.
    if kind == "goals" || kind == "pathways" {
        for table in ["experiences", "difficulties"] {
            for mut item in store::list::<FollowUp>(db, table)? {
                let links = if kind == "goals" {
                    &mut item.goal_ids
                } else {
                    &mut item.pathway_ids
                };
                let length = links.len();
                links.retain(|value| value != id);
                if length != links.len() {
                    item.version += 1;
                    db.execute(
                        &format!("UPDATE {table} SET data=?1 WHERE id=?2"),
                        params![store::json(&item)?, item.id],
                    )?;
                    if table == "experiences" {
                        store::save_experience_links(db, &item)?;
                    }
                }
            }
        }
        for mut session in store::list::<Session>(db, "sessions")? {
            let before = store::json(&session.input)?;
            if kind == "goals" {
                session
                    .input
                    .difficulty_goal_ids
                    .retain(|value| value != id);
                session
                    .input
                    .experience_goal_ids
                    .retain(|value| value != id);
            } else {
                session
                    .input
                    .difficulty_pathway_ids
                    .retain(|value| value != id);
                session
                    .input
                    .experience_pathway_ids
                    .retain(|value| value != id);
            }
            if before != store::json(&session.input)? {
                let previous: Session =
                    store::get(db, "sessions", &session.id)?.ok_or_else(ApiError::missing)?;
                if session.status == "completed" {
                    record_session_revision(db, &previous, "关联删除前版本")?;
                }
                session.version += 1;
                db.execute(
                    "UPDATE sessions SET data=?1 WHERE id=?2",
                    params![store::json(&session)?, session.id],
                )?;
                if session.status == "completed" {
                    record_session_revision(db, &session, "管理员删除了关联的目标或方法")?;
                }
            }
        }
    }
    if kind == "goals" || kind == "pathways" {
        for mut lesson in store::list::<Lesson>(db, "lessons")? {
            let before = store::json(&lesson.input)?;
            if let Some(links) = lesson.input.links.as_mut() {
                if kind == "goals" {
                    links.problem_goal_ids.retain(|v| v != id);
                } else {
                    links.pathway_ids.retain(|v| v != id);
                    links.problem_pathway_ids.retain(|v| v != id);
                }
            }
            if before != store::json(&lesson.input)? {
                lesson.version += 1;
                db.execute(
                    "UPDATE lessons SET data=?1 WHERE id=?2",
                    params![store::json(&lesson)?, lesson.id],
                )?;
            }
        }
    }
    match kind {
        "goals" => {
            for mut pathway in store::list::<Pathway>(db, "pathways")? {
                if pathway.goal_ids.contains(&id.to_owned()) {
                    pathway.goal_ids.retain(|value| value != id);
                    pathway.version += 1;
                    db.execute(
                        "UPDATE pathways SET data=?1 WHERE id=?2",
                        params![store::json(&pathway)?, pathway.id],
                    )?;
                }
            }
            db.execute("DELETE FROM goal_pathways WHERE goal_id=?1", params![id])?;
            db.execute("DELETE FROM goal_revisions WHERE goal_id=?1", params![id])?;
        }
        "pathways" => {
            db.execute("DELETE FROM goal_pathways WHERE pathway_id=?1", params![id])?;
        }
        "experiences" => {
            db.execute(
                "DELETE FROM experience_goals WHERE experience_id=?1",
                params![id],
            )?;
            db.execute(
                "DELETE FROM experience_pathways WHERE experience_id=?1",
                params![id],
            )?;
        }
        "sessions" => {
            for table in ["experiences", "difficulties"] {
                for mut item in store::list::<FollowUp>(db, table)? {
                    if item.source_session_id.as_deref() == Some(id) {
                        item.source_session_id = None;
                        item.source_deleted = true;
                        item.version += 1;
                        db.execute(
                            &format!(
                                "UPDATE {table} SET source_session_id=NULL,data=?1 WHERE id=?2"
                            ),
                            params![store::json(&item)?, item.id],
                        )?;
                    }
                }
            }
            db.execute(
                "DELETE FROM session_targets WHERE session_id=?1",
                params![id],
            )?;
            db.execute(
                "DELETE FROM session_revisions WHERE session_id=?1",
                params![id],
            )?;
            // Goal revisions remain historical evidence, but no dead source button.
            for mut revision in store::list::<GoalRevision>(db, "goal_revisions")? {
                if revision.source_session_id.as_deref() == Some(id) {
                    revision.source_session_id = None;
                    db.execute(
                        "UPDATE goal_revisions SET data=?1 WHERE goal_id=?2 AND version=?3",
                        params![
                            store::json(&revision)?,
                            revision.goal.id,
                            revision.goal.version as i64
                        ],
                    )?;
                }
            }
        }
        "lessons" => {
            for mut item in store::list::<FollowUp>(db, "difficulties")? {
                if item.source_lesson_id.as_deref() == Some(id) {
                    item.source_deleted = true;
                    item.version += 1;
                    db.execute(
                        "UPDATE difficulties SET data=?1 WHERE id=?2",
                        params![store::json(&item)?, item.id],
                    )?;
                }
            }
            db.execute("DELETE FROM lesson_targets WHERE lesson_id=?1", params![id])?;
        }
        _ => {}
    }
    db.execute(&format!("DELETE FROM {kind} WHERE id=?1"), params![id])
}

/// Only fields actually changed in the training are propagated. A concurrent
/// follow-up edit rejects the entire revision; unrelated summary edits preserve it.
pub fn sync_followups(
    db: &dyn Sql,
    old: &Session,
    new: &Session,
    revision: &SessionRevisionInput,
) -> Result<(Option<String>, Option<String>)> {
    let primary = |input: &SessionInput| {
        input
            .targets
            .iter()
            .find(|target| target.role == Role::Primary)
            .map(|target| vec![target.goal_id.clone()])
            .unwrap_or_default()
    };
    let mut results = Vec::new();
    for table in ["difficulties", "experiences"] {
        let experience = table == "experiences";
        let fields = |input: &SessionInput| {
            if experience {
                (
                    input.has_experience,
                    input.experience.clone(),
                    input.experience_goal_ids.clone(),
                    input.experience_pathway_ids.clone(),
                    Some(input.experience_kind.clone()),
                )
            } else {
                (
                    input.has_difficulty,
                    input.difficulty.clone(),
                    input.difficulty_goal_ids.clone(),
                    input.difficulty_pathway_ids.clone(),
                    None,
                )
            }
        };
        let before = fields(&old.input);
        let after = fields(&new.input);
        let existing = db
            .query(
                &format!("SELECT data FROM {table} WHERE source_session_id=?1"),
                params![new.id],
            )?
            .into_iter()
            .next()
            .map(|row| serde_json::from_str::<FollowUp>(&row[0]))
            .transpose()?;
        // Removing a summary checkbox cannot grant members deletion permissions.
        if !after.0 {
            results.push(existing.map(|item| item.id));
            continue;
        }
        let bindings_changed = before.2 != after.2
            || before.3 != after.3
            || (before.2.is_empty()
                && before.3.is_empty()
                && primary(&old.input) != primary(&new.input));
        if before.0 == after.0 && before.1 == after.1 && before.4 == after.4 && !bindings_changed {
            results.push(existing.map(|item| item.id));
            continue;
        }
        if existing.is_none()
            && !db
                .query(
                    "SELECT id FROM deleted_records WHERE kind=?1 AND source_session_id=?2",
                    params![table, new.id],
                )?
                .is_empty()
        {
            results.push(None);
            continue; // Never recreate an administrator-deleted follow-up.
        }
        let expected = if experience {
            revision.experience_version
        } else {
            revision.difficulty_version
        };
        if existing.as_ref().map(|item| item.version) != expected {
            return Err(ApiError::conflict(
                "关联困难或经验已有新版本，请刷新核对后重新修订训练",
            ));
        }
        // Validated or deliberately stopped knowledge is independently curated.
        // Correcting its source observation must not reuse old evidence for new text.
        if experience
            && existing
                .as_ref()
                .is_some_and(|item| matches!(item.status.as_str(), "verified" | "stopped"))
        {
            results.push(existing.map(|item| item.id));
            continue;
        }
        let mut item = existing.unwrap_or_else(|| FollowUp {
            id: Uuid::new_v4().to_string(),
            version: 0,
            source_session_id: Some(new.id.clone()),
            source_lesson_id: None,
            source_deleted: false,
            content: String::new(),
            kind: None,
            status: "pending".into(),
            next_review_date: None,
            conclusion: String::new(),
            goal_ids: vec![],
            pathway_ids: vec![],
        });
        let fresh = item.version == 0 || !before.0;
        item.version += 1;
        if fresh || before.1 != after.1 {
            item.content = after.1.trim().into();
        }
        if fresh || before.4 != after.4 {
            item.kind = after.4;
        }
        if fresh || bindings_changed {
            item.goal_ids = if after.2.is_empty() && after.3.is_empty() {
                primary(&new.input)
            } else {
                after.2
            };
            item.pathway_ids = after.3;
        }
        db.execute(&format!("INSERT INTO {table}(id,source_session_id,data) VALUES (?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data"), params![item.id,new.id,store::json(&item)?])?;
        if experience {
            store::save_experience_links(db, &item)?;
        }
        results.push(Some(item.id));
    }
    Ok((results.remove(0), results.remove(0)))
}
