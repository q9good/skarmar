//! Institution lesson links, snapshots and independently curated follow-ups.
use crate::{
    model::*,
    sql::{Sql, params},
    store::{self, ApiError, Result},
};
use uuid::Uuid;

pub fn save(db: &dyn Sql, id: &str, request: SaveLesson) -> Result<Lesson> {
    crate::mutations::ensure_not_deleted(db, "lessons", id)?;
    store::valid_id(id)?;
    let current: Option<Lesson> = store::get(db, "lessons", id)?;
    let mut input = request.lesson;
    store::valid_date(&input.business_date)?;
    input.course = input.course.trim().into();
    input.teacher = input.teacher.trim().into();
    if input.course.is_empty() || input.teacher.is_empty() {
        return Err(ApiError::invalid("请填写课程名称和授课老师"));
    }
    if input.course.chars().count() > 200
        || input.teacher.chars().count() > 100
        || [
            &input.content,
            &input.goals,
            &input.problems,
            &input.home_cooperation,
        ]
        .iter()
        .any(|v| v.chars().count() > 10_000)
    {
        return Err(ApiError::invalid("上课记录内容过长，请适当缩短"));
    }
    // Legacy clients never implicitly clear relationships they cannot display.
    if input.links.is_none() {
        input.links = current.as_ref().and_then(|v| v.input.links.clone());
    }
    if let Some(prior) = &current {
        if prior.input == input
            && (request.expected_version == Some(prior.version)
                || request.expected_version.and_then(|v| v.checked_add(1)) == Some(prior.version)
                || (prior.version == 1 && request.expected_version.is_none()))
        {
            return Ok(prior.clone());
        }
        if request.expected_version != Some(prior.version) {
            return Err(ApiError::conflict("上课记录已被更新，请刷新后核对"));
        }
    } else if request.expected_version.is_some() {
        return Err(ApiError::missing());
    }

    let mut snapshots = vec![];
    if let Some(links) = &input.links {
        let ids: Vec<_> = links.targets.iter().map(|t| t.goal_id.clone()).collect();
        store::validate_links(db, "goals", &ids)?;
        store::validate_links(db, "pathways", &links.pathway_ids)?;
        store::validate_links(db, "goals", &links.problem_goal_ids)?;
        store::validate_links(db, "pathways", &links.problem_pathway_ids)?;
        if !links.targets.is_empty()
            && links
                .targets
                .iter()
                .filter(|t| t.role == Role::Primary)
                .count()
                != 1
        {
            return Err(ApiError::invalid("选择课程目标时需要且只能有一个主目标"));
        }
        for target in &links.targets {
            if target.plan.chars().count() > 10_000 || target.progress.chars().count() > 10_000 {
                return Err(ApiError::invalid("逐目标内容过长"));
            }
            let goal: Goal =
                store::get(db, "goals", &target.goal_id)?.ok_or_else(ApiError::missing)?;
            if goal.level == Level::Long {
                return Err(ApiError::invalid("课程目标请选择中期或短期目标"));
            }
            let snapshot = current
                .as_ref()
                .and_then(|v| {
                    v.target_snapshots
                        .iter()
                        .find(|s| s.goal_id == target.goal_id)
                })
                .cloned()
                .unwrap_or(LessonTargetSnapshot {
                    goal_id: target.goal_id.clone(),
                    goal_snapshot: goal,
                });
            snapshots.push(snapshot);
        }
        if links.track_problem && input.problems.trim().is_empty() {
            return Err(ApiError::invalid("加入困难跟进时请填写存在问题"));
        }
        if links.track_problem
            && links.problem_goal_ids.is_empty()
            && links.problem_pathway_ids.is_empty()
        {
            return Err(ApiError::invalid("请为课程困难选择关联目标或方法"));
        }
    }
    let lesson = Lesson {
        id: id.into(),
        version: match current.as_ref() {
            Some(v) => v
                .version
                .checked_add(1)
                .ok_or_else(|| ApiError::conflict("上课记录版本已达上限"))?,
            None => 1,
        },
        input,
        target_snapshots: snapshots,
    };
    for (kind, value) in [
        ("course", &lesson.input.course),
        ("teacher", &lesson.input.teacher),
    ] {
        store::add_lesson_option(
            db,
            NewLessonOption {
                kind: kind.into(),
                value: value.clone(),
            },
        )?;
    }
    db.execute("INSERT INTO lessons(id,business_date,data) VALUES (?1,?2,?3) ON CONFLICT(id) DO UPDATE SET business_date=excluded.business_date,data=excluded.data",
        params![lesson.id,lesson.input.business_date,store::json(&lesson)?])?;
    db.execute("DELETE FROM lesson_targets WHERE lesson_id=?1", params![id])?;
    if let Some(links) = &lesson.input.links {
        for target in &links.targets {
            db.execute(
                "INSERT INTO lesson_targets(lesson_id,goal_id,role) VALUES (?1,?2,?3)",
                params![id, target.goal_id, target.role.as_str()],
            )?;
        }
    }
    sync_problem(db, current.as_ref(), &lesson, request.difficulty_version)?;
    Ok(lesson)
}

fn sync_problem(
    db: &dyn Sql,
    old: Option<&Lesson>,
    lesson: &Lesson,
    expected: Option<u64>,
) -> Result<()> {
    let Some(links) = lesson.input.links.as_ref().filter(|v| v.track_problem) else {
        return Ok(());
    };
    let before = old.and_then(|v| v.input.links.as_ref());
    let changed = before.is_none_or(|v| {
        !v.track_problem
            || v.problem_goal_ids != links.problem_goal_ids
            || v.problem_pathway_ids != links.problem_pathway_ids
    }) || old.is_none_or(|v| v.input.problems != lesson.input.problems);
    if !changed {
        return Ok(());
    }
    let mut existing = store::list::<FollowUp>(db, "difficulties")?
        .into_iter()
        .find(|v| v.source_lesson_id.as_deref() == Some(&lesson.id));
    if existing.is_none() && !db.query("SELECT id FROM deleted_records WHERE kind='difficulties' AND json_extract(data,'$.source_lesson_id')=?1",params![lesson.id])?.is_empty() {
        return Ok(()); // A lesson edit must never recreate an admin-deleted difficulty.
    }
    if existing.as_ref().map(|v| v.version) != expected {
        return Err(ApiError::conflict(
            "关联困难已有新版本，请刷新核对后重新保存上课记录",
        ));
    }
    if existing.as_ref().is_some_and(|v| v.status == "resolved") {
        return Ok(());
    }
    let fresh = existing.is_none();
    let mut item = existing.take().unwrap_or_else(|| FollowUp {
        id: Uuid::new_v4().to_string(),
        version: 0,
        source_session_id: None,
        source_lesson_id: Some(lesson.id.clone()),
        source_deleted: false,
        content: String::new(),
        kind: None,
        status: "pending".into(),
        next_review_date: None,
        conclusion: String::new(),
        goal_ids: vec![],
        pathway_ids: vec![],
    });
    item.version += 1;
    // Only source fields actually changed are propagated; independently edited fields stay curated.
    if (fresh || old.is_none_or(|v| v.input.problems != lesson.input.problems))
        && (fresh || old.is_some_and(|v| item.content == v.input.problems.trim()))
    {
        item.content = lesson.input.problems.trim().into();
    }
    if (fresh || before.is_none_or(|v| v.problem_goal_ids != links.problem_goal_ids))
        && (fresh || before.is_some_and(|v| item.goal_ids == v.problem_goal_ids))
    {
        item.goal_ids = links.problem_goal_ids.clone();
    }
    if (fresh || before.is_none_or(|v| v.problem_pathway_ids != links.problem_pathway_ids))
        && (fresh || before.is_some_and(|v| item.pathway_ids == v.problem_pathway_ids))
    {
        item.pathway_ids = links.problem_pathway_ids.clone();
    }
    db.execute("INSERT INTO difficulties(id,source_session_id,data) VALUES (?1,NULL,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",params![item.id,store::json(&item)?])
}

pub fn rename_option(db: &dyn Sql, input: UpdateLessonOption) -> Result<LessonOption> {
    let new = input.new_value.as_deref().unwrap_or("").trim();
    let old = catalog_option(db, &input.kind, &input.value)?;
    if old.renamed_to.as_deref() == Some(new)
        && input.expected_version.checked_add(1) == Some(old.version)
    {
        return catalog_option(db, &input.kind, new);
    }
    if old.version != input.expected_version || old.renamed_to.is_some() {
        return Err(ApiError::conflict("选项已被更新，请刷新后核对"));
    }
    if new == old.value {
        return Ok(old);
    }
    if !db
        .query(
            "SELECT value FROM lesson_options WHERE kind=?1 AND value=?2",
            params![input.kind, new],
        )?
        .is_empty()
    {
        return Err(ApiError::conflict(
            "同名选项已经存在，请直接选择或恢复该选项",
        ));
    }
    let mut result = store::add_lesson_option(
        db,
        NewLessonOption {
            kind: input.kind.clone(),
            value: new.into(),
        },
    )?;
    if !old.active {
        db.execute(
            "UPDATE lesson_options SET active=0 WHERE kind=?1 AND value=?2",
            params![input.kind, new],
        )?;
        result.active = false;
    }
    db.execute("UPDATE lesson_options SET active=0,version=version+1,renamed_to=?1 WHERE kind=?2 AND value=?3",params![new,input.kind,input.value])?;
    Ok(result)
}

fn catalog_option(db: &dyn Sql, kind: &str, value: &str) -> Result<LessonOption> {
    let row = db.query("SELECT CAST(active AS TEXT),CAST(version AS TEXT),COALESCE(renamed_to,'') FROM lesson_options WHERE kind=?1 AND value=?2",params![kind,value])?.into_iter().next().ok_or_else(ApiError::missing)?;
    Ok(LessonOption {
        kind: kind.into(),
        value: value.into(),
        active: row[0] == "1",
        version: row[1].parse().map_err(|_| ApiError::storage())?,
        renamed_to: if row[2].is_empty() {
            None
        } else {
            Some(row[2].clone())
        },
    })
}
